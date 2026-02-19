/**
 * Voile Protocol - Core Types
 * 
 * Type definitions for unlock requests, LP offers, matched deals
 */

// ============================================================================
// BASIC TYPES (Miden-compatible)
// ============================================================================

/** Miden Field Element (64-bit) */
export type Felt = bigint;

/** Miden Word (4 Felts) */
export type Word = [Felt, Felt, Felt, Felt];

/** Account ID */
export type AccountId = string;

/** Note ID */
export type NoteId = string;

/** Timestamp (Unix seconds) */
export type Timestamp = bigint;

// ============================================================================
// UNLOCK REQUEST TYPES
// ============================================================================

/**
 * Private unlock request data
 * This NEVER leaves the user's device unencrypted
 */
export interface UnlockRequest {
  /** Unique request ID */
  requestId: bigint;
  
  /** Amount of staked assets to unlock (raw units) */
  amount: bigint;
  
  /** Unix timestamp when cooldown ends */
  cooldownEndTimestamp: Timestamp;
  
  /** Nullifier secret for preventing double-spend (32 bytes) */
  nullifierSecret: Uint8Array;
  
  /** User's account ID */
  userAccountId: AccountId;
  
  /** Request commitment (public hash) */
  commitment: Word;
  
  /** Created at timestamp */
  createdAt: Timestamp;
  
  /** Request status */
  status: UnlockRequestStatus;
}

export enum UnlockRequestStatus {
  /** Request created, waiting for match */
  PENDING = 'pending',
  /** Matched with LP, waiting for advance */
  MATCHED = 'matched',
  /** User received advance, waiting for cooldown */
  ADVANCED = 'advanced',
  /** Cooldown ended, settlement executed */
  SETTLED = 'settled',
  /** Request cancelled */
  CANCELLED = 'cancelled',
}

/**
 * Unlock request creation parameters
 */
export interface CreateUnlockRequestParams {
  /** Amount of staked assets to unlock (display units, e.g., 100 = 100 tokens) */
  amount: number;
  
  /** Cooldown duration in days (default: 14) */
  cooldownDays?: number;
}

// ============================================================================
// LP OFFER TYPES
// ============================================================================

/**
 * LP offer for providing liquidity
 */
export interface LpOffer {
  /** Unique offer ID */
  offerId: bigint;
  
  /** LP pool account ID */
  lpAccountId: AccountId;
  
  /** Maximum amount to advance (raw units) */
  maxAmount: bigint;
  
  /** Minimum amount to advance (raw units) */
  minAmount: bigint;
  
  /** Custom APR in basis points (optional) */
  customAprBps?: bigint;
  
  /** Offer commitment (public hash) */
  commitment: Word;
  
  /** Is offer currently active */
  isActive: boolean;
  
  /** Available liquidity */
  availableLiquidity: bigint;
}

/**
 * LP offer creation parameters
 */
export interface CreateLpOfferParams {
  /** Maximum amount to advance (display units) */
  maxAmount: number;
  
  /** Minimum amount to advance (display units) */
  minAmount: number;
  
  /** Custom APR percentage (e.g., 8 = 8%) */
  customAprPercent?: number;
}

// ============================================================================
// MATCHED DEAL TYPES
// ============================================================================

/**
 * A matched deal between user and LP
 */
export interface MatchedDeal {
  /** Unique deal ID */
  dealId: Word;
  
  /** The unlock request commitment */
  requestCommitment: Word;
  
  /** The LP offer ID */
  offerId: bigint;
  
  /** LP account ID */
  lpAccountId: AccountId;
  
  /** User account ID */
  userAccountId: AccountId;
  
  /** Staked asset amount */
  stakedAmount: bigint;
  
  /** Net USDC advance amount (after fees) */
  advanceAmount: bigint;
  
  /** Advance fee amount */
  advanceFee: bigint;
  
  /** Expected APR interest */
  expectedInterest: bigint;
  
  /** Settlement note ID */
  settlementNoteId?: NoteId;
  
  /** Advance note ID */
  advanceNoteId?: NoteId;
  
  /** Timestamp when deal was matched */
  matchedAt: Timestamp;
  
  /** Cooldown end timestamp */
  cooldownEndTimestamp: Timestamp;
  
  /** Deal status */
  status: DealStatus;
}

export enum DealStatus {
  /** Deal matched, pending advance */
  PENDING_ADVANCE = 'pending_advance',
  /** Advance note created */
  ADVANCE_CREATED = 'advance_created',
  /** User received advance */
  ADVANCED = 'advanced',
  /** Waiting for cooldown to end */
  PENDING_SETTLEMENT = 'pending_settlement',
  /** Settlement executed */
  SETTLED = 'settled',
  /** Deal failed/cancelled */
  FAILED = 'failed',
}

// ============================================================================
// PRICING TYPES
// ============================================================================

/**
 * Pricing breakdown for a deal
 */
export interface PricingBreakdown {
  /** Principal (staked asset amount) */
  principal: bigint;
  
  /** Advance fee (5%) */
  advanceFee: bigint;
  
  /** Net advance amount */
  netAdvance: bigint;
  
  /** APR interest */
  aprInterest: bigint;
  
  /** LP fee share (80% of advance fee) */
  lpFeeShare: bigint;
  
  /** Protocol fee share (20% of advance fee) */
  protocolFeeShare: bigint;
  
  /** Total LP earnings */
  totalLpEarnings: bigint;
  
  /** Effective APY for LP */
  effectiveApy: number;
}

// ============================================================================
// TRANSACTION TYPES
// ============================================================================

/**
 * Transaction status
 */
export interface TransactionStatus {
  /** Transaction ID */
  txId: string;
  
  /** Status */
  status: 'pending' | 'confirmed' | 'failed';
  
  /** Block number (if confirmed) */
  blockNumber?: bigint;
  
  /** Error message (if failed) */
  error?: string;
}

// ============================================================================
// NOTE TYPES
// ============================================================================

/**
 * Settlement note data
 */
export interface SettlementNote {
  /** Note ID */
  noteId: NoteId;
  
  /** Request ID */
  requestId: bigint;
  
  /** Staked asset amount */
  amount: bigint;
  
  /** Cooldown end timestamp */
  cooldownEndTimestamp: Timestamp;
  
  /** Deal ID */
  dealId: Word;
  
  /** Is note consumed */
  isConsumed: boolean;
}

/**
 * Advance note data
 */
export interface AdvanceNote {
  /** Note ID */
  noteId: NoteId;
  
  /** USDC advance amount */
  advanceAmount: bigint;
  
  /** Deal ID */
  dealId: Word;
  
  /** Offer ID */
  offerId: bigint;
  
  /** User commitment */
  userCommitment: Felt;
  
  /** Is note consumed */
  isConsumed: boolean;
}
