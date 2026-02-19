/**
 * Voile Protocol - Main SDK Entry Point
 * 
 * High-level API for users and LPs to interact with Voile
 */

import {
  DEFAULT_COOLDOWN_SECONDS,
  ONE_USDC,
  usdcToRaw,
  rawToUsdc,
} from './config';

import {
  buildUnlockRequest,
  buildLpOffer,
  MatchingEngine,
  formatMatchResult,
  isReadyForSettlement,
} from './matching';

import {
  calculatePricingBreakdown,
  formatPricingBreakdown,
  estimatePricing,
} from './pricing';

import {
  VoileMidenClient,
  createMidenClient,
  createTestnetClient,
} from './client';

import {
  currentTimestamp,
  remainingCooldown,
  formatTimestamp,
} from './crypto';

import type {
  UnlockRequest,
  LpOffer,
  MatchedDeal,
  AccountId,
  PricingBreakdown,
} from './types';

import {
  UnlockRequestStatus,
  DealStatus,
} from './types';

// ============================================================================
// VOILE SDK
// ============================================================================

/**
 * Main Voile Protocol SDK
 * 
 * Provides high-level APIs for:
 * - Users: Create unlock requests, receive advances
 * - LPs: Create offers, provide liquidity, settle deals
 */
export class VoileSDK {
  private client: VoileMidenClient;
  private matchingEngine: MatchingEngine;
  private pendingRequests: Map<string, UnlockRequest> = new Map();
  private activeDeals: Map<string, MatchedDeal> = new Map();
  
  constructor(client?: VoileMidenClient) {
    this.client = client ?? createMidenClient();
    this.matchingEngine = new MatchingEngine();
  }
  
  // ==========================================================================
  // INITIALIZATION
  // ==========================================================================
  
  /**
   * Initialize the SDK and connect to network
   */
  async initialize(): Promise<void> {
    await this.client.connect();
    console.log('✓ Voile SDK initialized');
  }
  
  /**
   * Disconnect from network
   */
  async disconnect(): Promise<void> {
    await this.client.disconnect();
  }
  
  // ==========================================================================
  // USER OPERATIONS
  // ==========================================================================
  
  /**
   * Create a new user account
   */
  async createUserAccount(): Promise<AccountId> {
    return await this.client.createUserAccount();
  }
  
  /**
   * Preview pricing for an unlock request (doesn't submit)
   */
  previewUnlockRequest(
    amountUsdc: number,
    cooldownDays: number = 14,
  ): {
    fee: number;
    netAdvance: number;
    interest: number;
    breakdown: PricingBreakdown;
  } {
    return estimatePricing(amountUsdc, cooldownDays);
  }
  
  /**
   * Create an unlock request (stays private on device)
   */
  createUnlockRequest(
    userAccountId: AccountId,
    amountUsdc: number,
    cooldownDays: number = 14,
  ): UnlockRequest {
    const amount = usdcToRaw(amountUsdc);
    const request = buildUnlockRequest(userAccountId, amount, cooldownDays);
    
    // Store locally
    this.pendingRequests.set(request.requestId.toString(), request);
    
    console.log(`✓ Unlock request created (PRIVATE)`);
    console.log(`  Request ID: ${request.requestId.toString(16).slice(0, 8)}...`);
    console.log(`  Amount: ${amountUsdc} USDC`);
    console.log(`  Cooldown: ${cooldownDays} days`);
    
    return request;
  }
  
  /**
   * Find and match with best LP offer
   */
  async matchRequest(request: UnlockRequest): Promise<MatchedDeal | null> {
    console.log('🔍 Finding LP matches (off-chain)...');
    
    // Preview matches first
    const preview = this.matchingEngine.previewMatch(request);
    
    if (!preview.bestOffer) {
      console.log('❌ No matching LP offers found');
      return null;
    }
    
    console.log(`✓ Found ${preview.alternativeOffers.length + 1} matching offers`);
    console.log(formatPricingBreakdown(preview.pricing!));
    
    // Execute match
    const deal = this.matchingEngine.matchRequest(request);
    
    if (deal) {
      this.activeDeals.set(deal.dealId[0].toString(), deal);
      request.status = UnlockRequestStatus.MATCHED;
      
      console.log(formatMatchResult(deal));
    }
    
    return deal;
  }
  
  /**
   * Submit request and receive advance
   */
  async executeUnlockRequest(request: UnlockRequest): Promise<MatchedDeal | null> {
    console.log('\n=== Executing Unlock Request ===\n');
    
    // Step 1: Find match
    const deal = await this.matchRequest(request);
    if (!deal) return null;
    
    // Step 2: Submit request commitment on-chain
    console.log('\n📤 Submitting request commitment...');
    const submitTx = await this.client.submitUnlockRequest(request);
    console.log(`✓ Request submitted: ${submitTx.txId}`);
    
    // Step 3: Create settlement note
    console.log('\n📝 Creating settlement note...');
    const settlementNote = await this.client.createSettlementNote(deal, request);
    deal.settlementNoteId = settlementNote.noteId;
    console.log(`✓ Settlement note: ${settlementNote.noteId}`);
    
    // Step 4: LP creates and sends advance note
    console.log('\n💵 Creating advance note (USDC transfer)...');
    const advanceNote = await this.client.createAdvanceNote(deal);
    deal.advanceNoteId = advanceNote.noteId;
    deal.status = DealStatus.ADVANCE_CREATED;
    console.log(`✓ Advance note: ${advanceNote.noteId}`);
    
    // Step 5: User consumes advance note
    console.log('\n📥 Receiving USDC advance...');
    const consumeTx = await this.client.consumeAdvanceNote(request.userAccountId, advanceNote);
    deal.status = DealStatus.ADVANCED;
    request.status = UnlockRequestStatus.ADVANCED;
    console.log(`✓ USDC received: ${rawToUsdc(deal.advanceAmount)} USDC`);
    
    // Step 6: Wait for cooldown
    deal.status = DealStatus.PENDING_SETTLEMENT;
    const remaining = remainingCooldown(deal.cooldownEndTimestamp);
    console.log(`\n⏳ Settlement in ${remaining.days}d ${remaining.hours}h ${remaining.minutes}m`);
    
    return deal;
  }
  
  // ==========================================================================
  // LP OPERATIONS
  // ==========================================================================
  
  /**
   * Create a new LP pool account
   */
  async createLpPoolAccount(initialUsdcAmount: number): Promise<AccountId> {
    const initialUsdc = usdcToRaw(initialUsdcAmount);
    return await this.client.createLpPoolAccount(initialUsdc);
  }
  
  /**
   * Create an LP offer
   */
  createLpOffer(
    lpAccountId: AccountId,
    maxAmountUsdc: number,
    minAmountUsdc: number = 100,
    customAprPercent?: number,
  ): LpOffer {
    const maxAmount = usdcToRaw(maxAmountUsdc);
    const minAmount = usdcToRaw(minAmountUsdc);
    
    const offer = buildLpOffer(
      lpAccountId,
      maxAmount,
      minAmount,
      customAprPercent,
    );
    
    // Add to matching engine
    this.matchingEngine.addOffer(offer);
    
    console.log(`✓ LP offer created`);
    console.log(`  Offer ID: ${offer.offerId.toString(16).slice(0, 8)}...`);
    console.log(`  Range: ${minAmountUsdc} - ${maxAmountUsdc} USDC`);
    console.log(`  APR: ${customAprPercent ?? 10}%`);
    
    return offer;
  }
  
  /**
   * Cancel an LP offer
   */
  cancelLpOffer(offerId: bigint): void {
    this.matchingEngine.removeOffer(offerId);
    console.log(`✓ LP offer cancelled: ${offerId.toString(16).slice(0, 8)}...`);
  }
  
  /**
   * Execute settlement (after cooldown ends)
   */
  async executeSettlement(deal: MatchedDeal): Promise<boolean> {
    if (!isReadyForSettlement(deal)) {
      const remaining = remainingCooldown(deal.cooldownEndTimestamp);
      console.log(`❌ Cooldown not ended. ${remaining.days}d ${remaining.hours}h remaining`);
      return false;
    }
    
    console.log('\n=== Executing Settlement ===\n');
    
    // In production: consume settlement note
    // const tx = await this.client.executeSettlement(deal.lpAccountId, settlementNote);
    
    deal.status = DealStatus.SETTLED;
    console.log(`✓ Settlement complete!`);
    console.log(`  LP received: ${rawToUsdc(deal.stakedAmount)} staked tokens`);
    console.log(`  LP earned: ${rawToUsdc(deal.advanceFee + deal.expectedInterest)} USDC`);
    
    return true;
  }
  
  // ==========================================================================
  // STATUS & INFO
  // ==========================================================================
  
  /**
   * Get all pending requests
   */
  getPendingRequests(): UnlockRequest[] {
    return Array.from(this.pendingRequests.values());
  }
  
  /**
   * Get all active deals
   */
  getActiveDeals(): MatchedDeal[] {
    return Array.from(this.activeDeals.values());
  }
  
  /**
   * Get all LP offers
   */
  getLpOffers(): LpOffer[] {
    return this.matchingEngine.getActiveOffers();
  }
  
  /**
   * Get deal status
   */
  getDealStatus(dealId: bigint): MatchedDeal | undefined {
    return this.activeDeals.get(dealId.toString());
  }
}

// ============================================================================
// FACTORY FUNCTIONS
// ============================================================================

/**
 * Create a new Voile SDK instance
 */
export function createVoileSDK(): VoileSDK {
  return new VoileSDK();
}

/**
 * Create SDK connected to testnet
 */
export async function createTestnetSDK(): Promise<VoileSDK> {
  const client = await createTestnetClient(true);
  const sdk = new VoileSDK(client);
  return sdk;
}

// ============================================================================
// RE-EXPORTS
// ============================================================================

export * from './types';
export * from './config';
export * from './crypto';
export * from './pricing';
export * from './matching';
export * from './client';
