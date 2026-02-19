/**
 * Voile Protocol - Pricing Calculator
 * 
 * Calculates fees, interest, and earnings for all parties
 */

import {
  DEFAULT_ADVANCE_FEE_BPS,
  DEFAULT_APR_BPS,
  DEFAULT_COOLDOWN_SECONDS,
  PROTOCOL_FEE_BPS,
  LP_FEE_BPS,
  ONE_USDC,
  usdcToRaw,
  rawToUsdc,
} from './config';

import type { PricingBreakdown } from './types';

// ============================================================================
// CORE PRICING FUNCTIONS
// ============================================================================

/**
 * Calculate advance fee
 * @param principal - Principal amount in raw units
 * @param feeBps - Fee in basis points (default: 500 = 5%)
 * @returns Fee amount in raw units
 */
export function calculateAdvanceFee(
  principal: bigint,
  feeBps: bigint = DEFAULT_ADVANCE_FEE_BPS,
): bigint {
  return (principal * feeBps) / 10000n;
}

/**
 * Calculate net advance after fee
 * @param principal - Principal amount in raw units
 * @returns Net advance amount in raw units
 */
export function calculateNetAdvance(principal: bigint): bigint {
  return principal - calculateAdvanceFee(principal);
}

/**
 * Calculate APR interest for cooldown period
 * @param principal - Principal amount in raw units
 * @param days - Number of days
 * @param aprBps - APR in basis points (default: 1000 = 10%)
 * @returns Interest amount in raw units
 */
export function calculateAprInterest(
  principal: bigint,
  days: bigint,
  aprBps: bigint = DEFAULT_APR_BPS,
): bigint {
  return (principal * aprBps * days) / (10000n * 365n);
}

/**
 * Calculate LP's share of the advance fee
 * @param totalFee - Total fee amount
 * @returns LP's share (80% by default)
 */
export function calculateLpFeeShare(totalFee: bigint): bigint {
  return (totalFee * LP_FEE_BPS) / 10000n;
}

/**
 * Calculate protocol's share of the advance fee
 * @param totalFee - Total fee amount
 * @returns Protocol's share (20% by default)
 */
export function calculateProtocolFeeShare(totalFee: bigint): bigint {
  return (totalFee * PROTOCOL_FEE_BPS) / 10000n;
}

// ============================================================================
// FULL PRICING BREAKDOWN
// ============================================================================

/**
 * Calculate full pricing breakdown for a deal
 * @param principal - Principal amount in raw units
 * @param cooldownDays - Number of cooldown days
 * @param customAprBps - Custom APR (optional)
 * @returns Complete pricing breakdown
 */
export function calculatePricingBreakdown(
  principal: bigint,
  cooldownDays: bigint = 14n,
  customAprBps?: bigint,
): PricingBreakdown {
  const aprBps = customAprBps ?? DEFAULT_APR_BPS;
  
  const advanceFee = calculateAdvanceFee(principal);
  const netAdvance = principal - advanceFee;
  const aprInterest = calculateAprInterest(principal, cooldownDays, aprBps);
  const lpFeeShare = calculateLpFeeShare(advanceFee);
  const protocolFeeShare = calculateProtocolFeeShare(advanceFee);
  const totalLpEarnings = lpFeeShare + aprInterest;
  
  // Calculate effective APY for LP
  // APY = (earnings / netAdvance) * (365 / days) * 100
  const effectiveApy = Number(totalLpEarnings) / Number(netAdvance) * 
    (365 / Number(cooldownDays)) * 100;
  
  return {
    principal,
    advanceFee,
    netAdvance,
    aprInterest,
    lpFeeShare,
    protocolFeeShare,
    totalLpEarnings,
    effectiveApy,
  };
}

// ============================================================================
// DISPLAY HELPERS
// ============================================================================

/**
 * Format a pricing breakdown for display
 */
export function formatPricingBreakdown(breakdown: PricingBreakdown): string {
  return `
Pricing Breakdown:
─────────────────────────────────────
Principal:         $${rawToUsdc(breakdown.principal).toLocaleString()} USDC
Advance Fee (5%):  $${rawToUsdc(breakdown.advanceFee).toLocaleString()} USDC
Net Advance:       $${rawToUsdc(breakdown.netAdvance).toLocaleString()} USDC
APR Interest:      $${rawToUsdc(breakdown.aprInterest).toFixed(2)} USDC
─────────────────────────────────────
LP Fee Share:      $${rawToUsdc(breakdown.lpFeeShare).toLocaleString()} USDC
Protocol Share:    $${rawToUsdc(breakdown.protocolFeeShare).toLocaleString()} USDC
Total LP Earnings: $${rawToUsdc(breakdown.totalLpEarnings).toFixed(2)} USDC
Effective APY:     ${breakdown.effectiveApy.toFixed(2)}%
─────────────────────────────────────
  `.trim();
}

/**
 * Estimate pricing for user display
 */
export function estimatePricing(
  amountUsdc: number,
  cooldownDays: number = 14,
): {
  fee: number;
  netAdvance: number;
  interest: number;
  breakdown: PricingBreakdown;
} {
  const principal = usdcToRaw(amountUsdc);
  const breakdown = calculatePricingBreakdown(principal, BigInt(cooldownDays));
  
  return {
    fee: rawToUsdc(breakdown.advanceFee),
    netAdvance: rawToUsdc(breakdown.netAdvance),
    interest: rawToUsdc(breakdown.aprInterest),
    breakdown,
  };
}

// ============================================================================
// VALIDATION
// ============================================================================

/**
 * Validate that pricing is profitable for LP
 */
export function isPricingProfitable(breakdown: PricingBreakdown): boolean {
  // LP should earn positive return
  return breakdown.totalLpEarnings > 0n;
}

/**
 * Validate minimum deal size
 */
export function isMinimumDealSize(principal: bigint, minUsdc: number = 100): boolean {
  return principal >= usdcToRaw(minUsdc);
}

/**
 * Validate cooldown period
 */
export function isValidCooldown(cooldownSeconds: bigint): boolean {
  // Minimum 1 day, maximum 365 days
  const oneDay = 24n * 60n * 60n;
  const oneYear = 365n * oneDay;
  return cooldownSeconds >= oneDay && cooldownSeconds <= oneYear;
}
