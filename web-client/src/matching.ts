/**
 * Voile Protocol - Off-Chain Matching Engine
 * 
 * Private matching between unlock requests and LP offers
 * All matching happens CLIENT-SIDE without broadcasting intent
 */

import {
  DEFAULT_APR_BPS,
  DEFAULT_COOLDOWN_SECONDS,
  ONE_USDC,
} from './config';

import {
  calculatePricingBreakdown,
  calculateNetAdvance,
} from './pricing';

import {
  computeRequestCommitment,
  computeOfferCommitment,
  computeDealId,
  generateNullifierSecret,
  currentTimestamp,
  cooldownEndTimestamp,
  randomFelt,
} from './crypto';

import type {
  UnlockRequest,
  LpOffer,
  MatchedDeal,
  AccountId,
  Word,
  Timestamp,
} from './types';

import {
  UnlockRequestStatus,
  DealStatus,
} from './types';

// ============================================================================
// UNLOCK REQUEST BUILDER
// ============================================================================

/**
 * Build an unlock request locally
 * This NEVER broadcasts intent to the network
 */
export function buildUnlockRequest(
  userAccountId: AccountId,
  amount: bigint,
  cooldownDays: number = 14,
): UnlockRequest {
  const requestId = randomFelt();
  const cooldownSeconds = BigInt(cooldownDays) * 24n * 60n * 60n;
  const cooldownEnd = cooldownEndTimestamp(cooldownSeconds);
  const nullifierSecret = generateNullifierSecret();
  
  const commitment = computeRequestCommitment(
    amount,
    cooldownEnd,
    nullifierSecret,
    userAccountId,
  );
  
  return {
    requestId,
    amount,
    cooldownEndTimestamp: cooldownEnd,
    nullifierSecret,
    userAccountId,
    commitment,
    createdAt: currentTimestamp(),
    status: UnlockRequestStatus.PENDING,
  };
}

// ============================================================================
// LP OFFER BUILDER
// ============================================================================

/**
 * Build an LP offer
 */
export function buildLpOffer(
  lpAccountId: AccountId,
  maxAmount: bigint,
  minAmount: bigint,
  customAprPercent?: number,
  availableLiquidity?: bigint,
): LpOffer {
  const offerId = randomFelt();
  const customAprBps = customAprPercent ? BigInt(Math.floor(customAprPercent * 100)) : undefined;
  
  const commitment = computeOfferCommitment(
    offerId,
    lpAccountId,
    maxAmount,
    minAmount,
  );
  
  return {
    offerId,
    lpAccountId,
    maxAmount,
    minAmount,
    customAprBps,
    commitment,
    isActive: true,
    availableLiquidity: availableLiquidity ?? maxAmount,
  };
}

// ============================================================================
// MATCHING ENGINE
// ============================================================================

/**
 * Off-chain matching engine
 * Finds the best LP offer for a user's unlock request
 */
export class MatchingEngine {
  private offers: Map<string, LpOffer> = new Map();
  
  /**
   * Add an LP offer to the engine
   */
  addOffer(offer: LpOffer): void {
    this.offers.set(offer.offerId.toString(), offer);
  }
  
  /**
   * Remove an offer
   */
  removeOffer(offerId: bigint): void {
    this.offers.delete(offerId.toString());
  }
  
  /**
   * Update offer status
   */
  updateOffer(offerId: bigint, updates: Partial<LpOffer>): void {
    const offer = this.offers.get(offerId.toString());
    if (offer) {
      this.offers.set(offerId.toString(), { ...offer, ...updates });
    }
  }
  
  /**
   * Get all active offers
   */
  getActiveOffers(): LpOffer[] {
    return Array.from(this.offers.values()).filter(o => o.isActive);
  }
  
  /**
   * Find matching offers for a request
   * Returns offers sorted by best terms (lowest APR)
   */
  findMatches(request: UnlockRequest): LpOffer[] {
    return this.getActiveOffers()
      .filter(offer => this.canMatch(offer, request))
      .sort((a, b) => {
        const aprA = a.customAprBps ?? DEFAULT_APR_BPS;
        const aprB = b.customAprBps ?? DEFAULT_APR_BPS;
        return Number(aprA - aprB);
      });
  }
  
  /**
   * Check if an offer can match a request
   */
  canMatch(offer: LpOffer, request: UnlockRequest): boolean {
    return (
      offer.isActive &&
      request.amount >= offer.minAmount &&
      request.amount <= offer.maxAmount &&
      offer.availableLiquidity >= calculateNetAdvance(request.amount)
    );
  }
  
  /**
   * Match a request with the best available offer
   */
  matchRequest(request: UnlockRequest): MatchedDeal | null {
    const matches = this.findMatches(request);
    
    if (matches.length === 0) {
      return null;
    }
    
    const bestOffer = matches[0];
    return this.createDeal(request, bestOffer);
  }
  
  /**
   * Match a request with a specific offer
   */
  matchWithOffer(request: UnlockRequest, offerId: bigint): MatchedDeal | null {
    const offer = this.offers.get(offerId.toString());
    
    if (!offer || !this.canMatch(offer, request)) {
      return null;
    }
    
    return this.createDeal(request, offer);
  }
  
  /**
   * Create a matched deal
   */
  private createDeal(request: UnlockRequest, offer: LpOffer): MatchedDeal {
    const dealId = computeDealId(
      request.commitment,
      offer.commitment,
      currentTimestamp(),
    );
    
    const netAdvance = calculateNetAdvance(request.amount);
    const advanceFee = request.amount - netAdvance;
    
    const cooldownDays = Number(
      (request.cooldownEndTimestamp - currentTimestamp()) / (24n * 60n * 60n)
    );
    
    const breakdown = calculatePricingBreakdown(
      request.amount,
      BigInt(Math.max(1, cooldownDays)),
      offer.customAprBps,
    );
    
    // Update offer's available liquidity
    this.updateOffer(offer.offerId, {
      availableLiquidity: offer.availableLiquidity - netAdvance,
    });
    
    return {
      dealId,
      requestCommitment: request.commitment,
      offerId: offer.offerId,
      lpAccountId: offer.lpAccountId,
      userAccountId: request.userAccountId,
      stakedAmount: request.amount,
      advanceAmount: netAdvance,
      advanceFee,
      expectedInterest: breakdown.aprInterest,
      matchedAt: currentTimestamp(),
      cooldownEndTimestamp: request.cooldownEndTimestamp,
      status: DealStatus.PENDING_ADVANCE,
    };
  }
  
  /**
   * Simulate matching for pricing preview
   * Does NOT create an actual deal
   */
  previewMatch(request: UnlockRequest): {
    bestOffer: LpOffer | null;
    pricing: ReturnType<typeof calculatePricingBreakdown> | null;
    alternativeOffers: LpOffer[];
  } {
    const matches = this.findMatches(request);
    
    if (matches.length === 0) {
      return { bestOffer: null, pricing: null, alternativeOffers: [] };
    }
    
    const bestOffer = matches[0];
    const cooldownDays = Number(
      (request.cooldownEndTimestamp - currentTimestamp()) / (24n * 60n * 60n)
    );
    
    const pricing = calculatePricingBreakdown(
      request.amount,
      BigInt(Math.max(1, cooldownDays)),
      bestOffer.customAprBps,
    );
    
    return {
      bestOffer,
      pricing,
      alternativeOffers: matches.slice(1),
    };
  }
}

// ============================================================================
// MATCHING RESULT HELPERS
// ============================================================================

/**
 * Format matching result for display
 */
export function formatMatchResult(deal: MatchedDeal): string {
  return `
Match Found!
─────────────────────────────────────
Deal ID:        ${deal.dealId[0].toString(16).slice(0, 8)}...
Staked Amount:  ${Number(deal.stakedAmount / ONE_USDC).toLocaleString()} USDC
Advance Amount: ${Number(deal.advanceAmount / ONE_USDC).toLocaleString()} USDC
Advance Fee:    ${Number(deal.advanceFee / ONE_USDC).toLocaleString()} USDC
Expected Int:   ${Number(deal.expectedInterest / ONE_USDC).toFixed(2)} USDC
─────────────────────────────────────
Status: ${deal.status}
  `.trim();
}

/**
 * Check if a deal is ready for settlement
 */
export function isReadyForSettlement(deal: MatchedDeal): boolean {
  return (
    deal.status === DealStatus.PENDING_SETTLEMENT &&
    currentTimestamp() >= deal.cooldownEndTimestamp
  );
}
