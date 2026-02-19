// Voile Protocol - User Account Contract
// Manages staked assets and generates private unlock requests
#![no_std]

extern crate alloc;

use alloc::vec::Vec;
use miden::{component, felt, Felt, StorageMap, StorageMapAccess, StorageValue, Word};

// Storage slot indices
const SLOT_UNLOCK_REQUESTS: u8 = 0;  // Map of unlock request commitments
const SLOT_STAKED_BALANCE: u8 = 1;   // User's staked asset balance
const SLOT_REQUEST_COUNTER: u8 = 2;  // Counter for unique request IDs

// Default pricing parameters (minimal for v1)
// Advance fee: 5% = 500 basis points
const DEFAULT_ADVANCE_FEE_BPS: u64 = 500;
// APR: 10% = 1000 basis points  
const DEFAULT_APR_BPS: u64 = 1000;
// Default cooldown: 14 days in seconds
const DEFAULT_COOLDOWN_SECONDS: u64 = 14 * 24 * 60 * 60;

/// Voile User Account - holds staked assets and manages private unlock requests
#[component]
struct VoileUserAccount {
    /// Storage map for unlock request commitments (request_id -> commitment)
    #[storage(slot(0), description = "unlock request commitments")]
    unlock_requests: StorageMap,
    
    /// User's staked asset balance (mock staked ETH or similar)
    #[storage(slot(1), description = "staked asset balance")]
    staked_balance: StorageValue,
    
    /// Counter for generating unique request IDs
    #[storage(slot(2), description = "request counter")]
    request_counter: StorageValue,
}

#[component]
impl VoileUserAccount {
    // =========================================================================
    // BALANCE MANAGEMENT
    // =========================================================================
    
    /// Get the current staked asset balance
    pub fn get_staked_balance(&self) -> Felt {
        self.staked_balance.get()
    }
    
    /// Deposit staked assets into the account
    /// In production, this would verify asset receipt from staking protocol
    pub fn deposit_staked_assets(&self, amount: Felt) -> Felt {
        let current = self.staked_balance.get();
        let new_balance = current + amount;
        self.staked_balance.set(new_balance);
        new_balance
    }
    
    /// Internal: reduce staked balance when locked for unlock request
    fn lock_staked_assets(&self, amount: Felt) -> bool {
        let current = self.staked_balance.get();
        if current.as_int() < amount.as_int() {
            return false;
        }
        let new_balance = current - amount;
        self.staked_balance.set(new_balance);
        true
    }
    
    // =========================================================================
    // UNLOCK REQUEST MANAGEMENT (Private)
    // =========================================================================
    
    /// Get the current request counter
    pub fn get_request_counter(&self) -> Felt {
        self.request_counter.get()
    }
    
    /// Create a private unlock request
    /// 
    /// This generates a commitment to the unlock request without revealing details.
    /// The actual request data (amount, cooldown_end, lp_match) stays private.
    ///
    /// # Arguments
    /// * `amount` - Amount of staked assets to unlock
    /// * `cooldown_end_timestamp` - Unix timestamp when cooldown ends
    /// * `request_commitment` - Hash commitment to full request details
    /// * `nullifier_secret` - Secret for generating nullifier (prevents double-spend)
    ///
    /// # Returns
    /// * Request ID (Felt) on success
    pub fn create_unlock_request(
        &self,
        amount: Felt,
        cooldown_end_timestamp: Felt,
        request_commitment: Word,
        nullifier_secret: Felt,
    ) -> Felt {
        // Verify sufficient balance and lock assets
        assert!(self.lock_staked_assets(amount), "Insufficient staked balance");
        
        // Get next request ID
        let request_id = self.request_counter.get();
        let new_counter = request_id + felt!(1);
        self.request_counter.set(new_counter);
        
        // Create storage key from request ID
        let request_key = Word::from([request_id, felt!(0), felt!(0), felt!(0)]);
        
        // Store the commitment (only the commitment is stored, not the details)
        // Commitment = hash(amount, cooldown_end, nullifier_secret, user_id)
        let commitment_value = request_commitment[0]; // Store first element of commitment
        self.unlock_requests.set(request_key, commitment_value);
        
        request_id
    }
    
    /// Get an unlock request commitment by ID
    pub fn get_request_commitment(&self, request_id: Felt) -> Felt {
        let request_key = Word::from([request_id, felt!(0), felt!(0), felt!(0)]);
        self.unlock_requests.get(&request_key)
    }
    
    /// Verify a request exists and matches commitment
    pub fn verify_request(&self, request_id: Felt, expected_commitment: Word) -> bool {
        let stored = self.get_request_commitment(request_id);
        stored == expected_commitment[0]
    }
    
    /// Mark request as matched (called when LP accepts)
    /// Stores LP commitment alongside request
    pub fn mark_request_matched(
        &self,
        request_id: Felt,
        lp_commitment: Word,
        settlement_note_hash: Word,
    ) -> bool {
        // Verify request exists
        let request_key = Word::from([request_id, felt!(0), felt!(0), felt!(0)]);
        let stored = self.unlock_requests.get(&request_key);
        if stored == felt!(0) {
            return false;
        }
        
        // Store LP match info (using offset keys)
        let lp_key = Word::from([request_id, felt!(1), felt!(0), felt!(0)]);
        self.unlock_requests.set(lp_key, lp_commitment[0]);
        
        let settlement_key = Word::from([request_id, felt!(2), felt!(0), felt!(0)]);
        self.unlock_requests.set(settlement_key, settlement_note_hash[0]);
        
        true
    }
    
    /// Cancel an unmatched request and return assets
    pub fn cancel_request(&self, request_id: Felt, amount: Felt) -> bool {
        let request_key = Word::from([request_id, felt!(0), felt!(0), felt!(0)]);
        let stored = self.unlock_requests.get(&request_key);
        
        // Verify request exists and is not matched
        if stored == felt!(0) {
            return false;
        }
        
        let lp_key = Word::from([request_id, felt!(1), felt!(0), felt!(0)]);
        let lp_match = self.unlock_requests.get(&lp_key);
        if lp_match != felt!(0) {
            return false; // Already matched, cannot cancel
        }
        
        // Clear the request
        self.unlock_requests.set(request_key, felt!(0));
        
        // Return locked assets to balance
        let current = self.staked_balance.get();
        self.staked_balance.set(current + amount);
        
        true
    }
    
    // =========================================================================
    // SETTLEMENT (Called by settlement note)
    // =========================================================================
    
    /// Release staked assets for settlement
    /// Called when cooldown completes and LP should receive assets
    ///
    /// # Arguments
    /// * `request_id` - The unlock request ID
    /// * `amount` - Amount to release
    /// * `settlement_proof` - Proof that cooldown has ended
    ///
    /// # Returns
    /// * true if settlement authorized
    pub fn authorize_settlement(
        &self,
        request_id: Felt,
        amount: Felt,
        current_timestamp: Felt,
        cooldown_end_timestamp: Felt,
    ) -> bool {
        // Verify cooldown has ended
        if current_timestamp.as_int() < cooldown_end_timestamp.as_int() {
            return false;
        }
        
        // Verify request exists and is matched
        let lp_key = Word::from([request_id, felt!(1), felt!(0), felt!(0)]);
        let lp_match = self.unlock_requests.get(&lp_key);
        if lp_match == felt!(0) {
            return false; // Not matched
        }
        
        // Mark as settled
        let settled_key = Word::from([request_id, felt!(3), felt!(0), felt!(0)]);
        self.unlock_requests.set(settled_key, felt!(1));
        
        true
    }
    
    // =========================================================================
    // PRICING HELPERS (Pure functions for off-chain computation)
    // =========================================================================
    
    /// Calculate advance fee amount
    /// fee = principal * fee_bps / 10000
    pub fn calculate_advance_fee(&self, principal: Felt) -> Felt {
        let fee_bps = felt!(DEFAULT_ADVANCE_FEE_BPS);
        let basis = felt!(10000);
        // Simple integer math: (principal * fee_bps) / 10000
        felt!((principal.as_int() * fee_bps.as_int()) / basis.as_int())
    }
    
    /// Calculate APR interest for cooldown period
    /// interest = principal * apr_bps / 10000 * days / 365
    pub fn calculate_apr_interest(&self, principal: Felt, cooldown_days: Felt) -> Felt {
        let apr_bps = felt!(DEFAULT_APR_BPS);
        let basis = felt!(10000);
        let days_per_year = felt!(365);
        
        // interest = (principal * apr_bps * days) / (10000 * 365)
        let numerator = principal.as_int() * apr_bps.as_int() * cooldown_days.as_int();
        let denominator = basis.as_int() * days_per_year.as_int();
        felt!(numerator / denominator)
    }
    
    /// Calculate net amount user receives after fees
    /// net = principal - advance_fee
    pub fn calculate_net_advance(&self, principal: Felt) -> Felt {
        let fee = self.calculate_advance_fee(principal);
        principal - fee
    }
    
    /// Get default cooldown in seconds
    pub fn get_default_cooldown_seconds(&self) -> Felt {
        felt!(DEFAULT_COOLDOWN_SECONDS)
    }
    
    /// Get advance fee in basis points
    pub fn get_advance_fee_bps(&self) -> Felt {
        felt!(DEFAULT_ADVANCE_FEE_BPS)
    }
    
    /// Get APR in basis points
    pub fn get_apr_bps(&self) -> Felt {
        felt!(DEFAULT_APR_BPS)
    }
}
