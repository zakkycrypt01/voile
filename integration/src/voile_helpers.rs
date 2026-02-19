//! Voile Protocol - Integration Helpers
//! Extended helpers for Voile-specific testing and deployment

use miden_client::{
    account::AccountId,
    asset::NoteAssets,
    note::{NoteTag, NoteType},
    Felt, Word,
};
use rand::RngCore;

use crate::helpers::NoteCreationConfig;

// ============================================================================
// VOILE PROTOCOL CONSTANTS
// ============================================================================

/// Default advance fee: 5% = 500 basis points
pub const DEFAULT_ADVANCE_FEE_BPS: u64 = 500;

/// Default APR: 10% = 1000 basis points
pub const DEFAULT_APR_BPS: u64 = 1000;

/// Default cooldown: 14 days in seconds
pub const DEFAULT_COOLDOWN_SECONDS: u64 = 14 * 24 * 60 * 60;

/// Protocol fee split: 20% to Voile
pub const PROTOCOL_FEE_BPS: u64 = 2000;

/// LP fee split: 80% to LP
pub const LP_FEE_BPS: u64 = 8000;

/// USDC decimals
pub const USDC_DECIMALS: u64 = 6;

/// 1 USDC in raw units
pub const ONE_USDC: u64 = 1_000_000;

// ============================================================================
// UNLOCK REQUEST TYPES
// ============================================================================

/// Private unlock request data
/// This stays on the user's device and is never broadcast
#[derive(Clone, Debug)]
pub struct UnlockRequest {
    /// Unique request ID
    pub request_id: u64,
    /// Amount of staked assets to unlock
    pub amount: u64,
    /// Unix timestamp when cooldown ends
    pub cooldown_end_timestamp: u64,
    /// Nullifier secret for preventing double-spend
    pub nullifier_secret: [u8; 32],
    /// User's account ID
    pub user_account_id: AccountId,
    /// Request commitment (public hash)
    pub commitment: Word,
}

impl UnlockRequest {
    /// Create a new unlock request
    pub fn new(
        request_id: u64,
        amount: u64,
        cooldown_end_timestamp: u64,
        user_account_id: AccountId,
        rng: &mut impl RngCore,
    ) -> Self {
        let mut nullifier_secret = [0u8; 32];
        rng.fill_bytes(&mut nullifier_secret);
        
        // Compute commitment = hash(amount, cooldown_end, nullifier_secret, user_id)
        let commitment = Self::compute_commitment(
            amount,
            cooldown_end_timestamp,
            &nullifier_secret,
            user_account_id,
        );
        
        Self {
            request_id,
            amount,
            cooldown_end_timestamp,
            nullifier_secret,
            user_account_id,
            commitment,
        }
    }
    
    /// Compute request commitment
    fn compute_commitment(
        amount: u64,
        cooldown_end: u64,
        nullifier: &[u8; 32],
        _user_id: AccountId,
    ) -> Word {
        // Simplified commitment: in production, use proper hash
        let nullifier_felt = u64::from_le_bytes(nullifier[0..8].try_into().unwrap());
        Word::from([
            Felt::new(amount),
            Felt::new(cooldown_end),
            Felt::new(nullifier_felt),
            Felt::new(0), // Placeholder for user_id
        ])
    }
    
    /// Calculate net advance amount after fees
    pub fn net_advance(&self) -> u64 {
        let fee = (self.amount * DEFAULT_ADVANCE_FEE_BPS) / 10000;
        self.amount - fee
    }
    
    /// Calculate advance fee
    pub fn advance_fee(&self) -> u64 {
        (self.amount * DEFAULT_ADVANCE_FEE_BPS) / 10000
    }
    
    /// Calculate APR interest for cooldown period
    pub fn apr_interest(&self, cooldown_days: u64) -> u64 {
        (self.amount * DEFAULT_APR_BPS * cooldown_days) / (10000 * 365)
    }
}

// ============================================================================
// LP OFFER TYPES
// ============================================================================

/// LP offer for providing liquidity
#[derive(Clone, Debug)]
pub struct LpOffer {
    /// Unique offer ID
    pub offer_id: u64,
    /// LP pool account ID
    pub lp_account_id: AccountId,
    /// Maximum USDC to advance
    pub max_amount: u64,
    /// Minimum USDC to advance
    pub min_amount: u64,
    /// Custom APR (basis points), or use default
    pub custom_apr_bps: Option<u64>,
    /// Offer commitment (public hash)
    pub commitment: Word,
    /// Is offer currently active
    pub is_active: bool,
}

impl LpOffer {
    /// Create a new LP offer
    pub fn new(
        offer_id: u64,
        lp_account_id: AccountId,
        max_amount: u64,
        min_amount: u64,
        custom_apr_bps: Option<u64>,
    ) -> Self {
        let commitment = Self::compute_commitment(
            offer_id,
            lp_account_id,
            max_amount,
            min_amount,
        );
        
        Self {
            offer_id,
            lp_account_id,
            max_amount,
            min_amount,
            custom_apr_bps,
            commitment,
            is_active: true,
        }
    }
    
    /// Compute offer commitment
    fn compute_commitment(
        offer_id: u64,
        _lp_id: AccountId,
        max_amount: u64,
        min_amount: u64,
    ) -> Word {
        Word::from([
            Felt::new(offer_id),
            Felt::new(0), // Placeholder for lp_id
            Felt::new(max_amount),
            Felt::new(min_amount),
        ])
    }
    
    /// Check if offer can match a request
    pub fn can_match(&self, request_amount: u64) -> bool {
        self.is_active && 
        request_amount >= self.min_amount && 
        request_amount <= self.max_amount
    }
}

// ============================================================================
// MATCHED DEAL TYPES
// ============================================================================

/// A matched deal between user and LP
#[derive(Clone, Debug)]
pub struct MatchedDeal {
    /// Unique deal ID
    pub deal_id: Word,
    /// The unlock request
    pub request: UnlockRequest,
    /// The matched LP offer
    pub offer: LpOffer,
    /// Net USDC advance amount
    pub advance_amount: u64,
    /// Settlement note hash
    pub settlement_note_hash: Word,
    /// Advance note hash
    pub advance_note_hash: Word,
    /// Timestamp when deal was matched
    pub matched_at: u64,
    /// Is deal settled
    pub is_settled: bool,
}

impl MatchedDeal {
    /// Create a new matched deal
    pub fn new(
        request: UnlockRequest,
        offer: LpOffer,
        rng: &mut impl RngCore,
    ) -> Self {
        // Generate unique deal ID
        let mut deal_id_bytes = [0u8; 32];
        rng.fill_bytes(&mut deal_id_bytes);
        let deal_id = Word::from([
            Felt::new(u64::from_le_bytes(deal_id_bytes[0..8].try_into().unwrap())),
            Felt::new(u64::from_le_bytes(deal_id_bytes[8..16].try_into().unwrap())),
            Felt::new(u64::from_le_bytes(deal_id_bytes[16..24].try_into().unwrap())),
            Felt::new(u64::from_le_bytes(deal_id_bytes[24..32].try_into().unwrap())),
        ]);
        
        let advance_amount = request.net_advance();
        
        Self {
            deal_id,
            request,
            offer,
            advance_amount,
            settlement_note_hash: Word::default(),
            advance_note_hash: Word::default(),
            matched_at: 0,
            is_settled: false,
        }
    }
    
    /// Calculate LP earnings
    pub fn lp_earnings(&self, cooldown_days: u64) -> (u64, u64) {
        let fee = self.request.advance_fee();
        let interest = self.request.apr_interest(cooldown_days);
        let lp_fee = (fee * LP_FEE_BPS) / 10000;
        (lp_fee, interest)
    }
    
    /// Calculate protocol earnings
    pub fn protocol_earnings(&self) -> u64 {
        let fee = self.request.advance_fee();
        (fee * PROTOCOL_FEE_BPS) / 10000
    }
}

// ============================================================================
// OFF-CHAIN MATCHING ENGINE
// ============================================================================

/// Private off-chain matching engine
/// All matching happens locally without broadcasting intent
pub struct MatchingEngine {
    /// Available LP offers (would be fetched privately in production)
    pub offers: Vec<LpOffer>,
}

impl MatchingEngine {
    /// Create a new matching engine
    pub fn new() -> Self {
        Self { offers: Vec::new() }
    }
    
    /// Add an LP offer to the engine
    pub fn add_offer(&mut self, offer: LpOffer) {
        self.offers.push(offer);
    }
    
    /// Find matching offers for a request
    /// Returns offers sorted by best terms (lowest APR)
    pub fn find_matches(&self, request: &UnlockRequest) -> Vec<&LpOffer> {
        let mut matches: Vec<&LpOffer> = self.offers
            .iter()
            .filter(|offer| offer.can_match(request.amount))
            .collect();
        
        // Sort by APR (lower is better for user)
        matches.sort_by(|a, b| {
            let apr_a = a.custom_apr_bps.unwrap_or(DEFAULT_APR_BPS);
            let apr_b = b.custom_apr_bps.unwrap_or(DEFAULT_APR_BPS);
            apr_a.cmp(&apr_b)
        });
        
        matches
    }
    
    /// Match a request with the best offer
    pub fn match_request(
        &self,
        request: UnlockRequest,
        rng: &mut impl RngCore,
    ) -> Option<MatchedDeal> {
        let matches = self.find_matches(&request);
        
        if let Some(best_offer) = matches.first() {
            Some(MatchedDeal::new(request, (*best_offer).clone(), rng))
        } else {
            None
        }
    }
}

impl Default for MatchingEngine {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// PRICING HELPERS
// ============================================================================

/// Calculate all pricing components for a deal
pub struct PricingCalculator;

impl PricingCalculator {
    /// Calculate advance fee
    pub fn advance_fee(principal: u64) -> u64 {
        (principal * DEFAULT_ADVANCE_FEE_BPS) / 10000
    }
    
    /// Calculate net advance after fee
    pub fn net_advance(principal: u64) -> u64 {
        principal - Self::advance_fee(principal)
    }
    
    /// Calculate APR interest
    pub fn apr_interest(principal: u64, days: u64) -> u64 {
        (principal * DEFAULT_APR_BPS * days) / (10000 * 365)
    }
    
    /// Calculate LP share of fee
    pub fn lp_fee_share(total_fee: u64) -> u64 {
        (total_fee * LP_FEE_BPS) / 10000
    }
    
    /// Calculate protocol share of fee
    pub fn protocol_fee_share(total_fee: u64) -> u64 {
        (total_fee * PROTOCOL_FEE_BPS) / 10000
    }
    
    /// Convert USDC display amount to raw (6 decimals)
    pub fn usdc_to_raw(display: u64) -> u64 {
        display * ONE_USDC
    }
    
    /// Convert raw USDC to display amount
    pub fn raw_to_usdc(raw: u64) -> u64 {
        raw / ONE_USDC
    }
}

// ============================================================================
// TIMESTAMP HELPERS
// ============================================================================

/// Get current Unix timestamp
pub fn current_timestamp() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

/// Calculate cooldown end timestamp
pub fn cooldown_end_timestamp(cooldown_seconds: u64) -> u64 {
    current_timestamp() + cooldown_seconds
}

/// Check if cooldown has ended
pub fn is_cooldown_ended(cooldown_end: u64) -> bool {
    current_timestamp() >= cooldown_end
}

// ============================================================================
// NOTE CREATION HELPERS
// ============================================================================

/// Create settlement note configuration
pub fn settlement_note_config(
    request_id: Felt,
    amount: Felt,
    cooldown_end: Felt,
    deal_id: Felt,
) -> NoteCreationConfig {
    NoteCreationConfig {
        note_type: NoteType::Private, // Encrypted note
        tag: NoteTag::for_local_use_case(1, 0).expect("Failed to create settlement note tag"),
        assets: NoteAssets::default(),
        inputs: vec![request_id, amount, cooldown_end, deal_id],
        ..Default::default()
    }
}

/// Create advance note configuration
pub fn advance_note_config(
    advance_amount: Felt,
    deal_id: Felt,
    offer_id: Felt,
    user_commitment: Felt,
) -> NoteCreationConfig {
    NoteCreationConfig {
        note_type: NoteType::Private, // Encrypted note
        tag: NoteTag::for_local_use_case(2, 0).expect("Failed to create advance note tag"),
        assets: NoteAssets::default(),
        inputs: vec![advance_amount, deal_id, offer_id, user_commitment],
        ..Default::default()
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_pricing_calculator() {
        let principal = 3000 * ONE_USDC; // $3,000
        
        // 5% fee = $150
        let fee = PricingCalculator::advance_fee(principal);
        assert_eq!(fee, 150 * ONE_USDC);
        
        // Net advance = $2,850
        let net = PricingCalculator::net_advance(principal);
        assert_eq!(net, 2850 * ONE_USDC);
        
        // APR interest for 14 days at 10%
        let interest = PricingCalculator::apr_interest(principal, 14);
        // = 3000 * 1000 * 14 / (10000 * 365) ≈ $11.50
        assert!(interest > 11 * ONE_USDC && interest < 12 * ONE_USDC);
    }
    
    #[test]
    fn test_fee_split() {
        let total_fee = 100 * ONE_USDC;
        
        // LP gets 80%
        let lp_share = PricingCalculator::lp_fee_share(total_fee);
        assert_eq!(lp_share, 80 * ONE_USDC);
        
        // Protocol gets 20%
        let protocol_share = PricingCalculator::protocol_fee_share(total_fee);
        assert_eq!(protocol_share, 20 * ONE_USDC);
    }
}
