/**
 * Voile Protocol - Configuration Constants
 * 
 * Default pricing parameters and protocol settings
 */

// ============================================================================
// PRICING PARAMETERS
// ============================================================================

/** Advance fee: 5% = 500 basis points */
export const DEFAULT_ADVANCE_FEE_BPS = 500n;

/** APR: 10% = 1000 basis points */
export const DEFAULT_APR_BPS = 1000n;

/** Default cooldown: 14 days in seconds */
export const DEFAULT_COOLDOWN_SECONDS = 14n * 24n * 60n * 60n;

/** Protocol fee split: 20% to Voile */
export const PROTOCOL_FEE_BPS = 2000n;

/** LP fee split: 80% to LP */
export const LP_FEE_BPS = 8000n;

// ============================================================================
// TOKEN CONSTANTS
// ============================================================================

/** USDC decimals */
export const USDC_DECIMALS = 6;

/** 1 USDC in raw units */
export const ONE_USDC = 1_000_000n;

// ============================================================================
// NETWORK CONFIGURATION
// ============================================================================

/** Miden testnet RPC endpoint */
export const MIDEN_TESTNET_RPC = 'https://rpc.testnet.miden.io';

/** Miden testnet chain ID */
export const MIDEN_TESTNET_CHAIN_ID = 'miden-testnet';

// ============================================================================
// NOTE TYPES
// ============================================================================

/** Settlement note tag */
export const SETTLEMENT_NOTE_TAG = 1;

/** Advance note tag */
export const ADVANCE_NOTE_TAG = 2;

// ============================================================================
// HELPERS
// ============================================================================

/** Convert USDC display amount to raw (6 decimals) */
export function usdcToRaw(display: number | bigint): bigint {
  return BigInt(display) * ONE_USDC;
}

/** Convert raw USDC to display amount */
export function rawToUsdc(raw: bigint): number {
  return Number(raw / ONE_USDC);
}

/** Calculate basis points */
export function bpsToPercent(bps: bigint): number {
  return Number(bps) / 100;
}
