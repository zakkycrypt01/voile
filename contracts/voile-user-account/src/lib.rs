// Voile Protocol - User Account Contract
// Manages staked assets and generates private unlock requests
#![no_std]

use miden::{component, felt, Felt, StorageMap, StorageMapAccess, Word};

/// Voile User Account - holds staked assets and manages private unlock requests
/// 
/// Storage layout:
/// Slot 0 (unlock_requests):
///   - [request_id, 0, 0, 0] -> request commitment
///   - [request_id, 1, 0, 0] -> LP commitment (when matched)
///   - [request_id, 2, 0, 0] -> locked amount
///   - [request_id, 3, 0, 0] -> settled flag (1 = settled)
/// 
/// Slot 1 (balances):
///   - [0, 0, 0, 0] -> staked asset balance
///   - [0, 0, 0, 1] -> request counter
#[component]
struct VoileUserAccount {
    #[storage(slot(0), description = "unlock request commitments")]
    unlock_requests: StorageMap,
    
    #[storage(slot(1), description = "balances and counters")]
    balances: StorageMap,
}

#[component]
impl VoileUserAccount {
    // =========================================================================
    // BALANCE MANAGEMENT
    // =========================================================================
    
    /// Get the current staked asset balance
    pub fn get_staked_balance(&self) -> Felt {
        let key = Word::from([felt!(0), felt!(0), felt!(0), felt!(0)]);
        self.balances.get(&key)
    }
    
    /// Deposit staked assets into the account
    pub fn deposit_staked_assets(&self, amount: Felt) -> Felt {
        let key = Word::from([felt!(0), felt!(0), felt!(0), felt!(0)]);
        let current: Felt = self.balances.get(&key);
        let new_balance = current + amount;
        self.balances.set(key, new_balance);
        new_balance
    }
    
    /// Withdraw staked assets (if available)
    /// Returns new balance
    pub fn withdraw_staked_assets(&self, amount: Felt) -> Felt {
        let key = Word::from([felt!(0), felt!(0), felt!(0), felt!(0)]);
        let current: Felt = self.balances.get(&key);
        let new_balance = current - amount;
        self.balances.set(key, new_balance);
        new_balance
    }
    
    // =========================================================================
    // UNLOCK REQUEST MANAGEMENT
    // =========================================================================
    
    /// Get the current request counter
    pub fn get_request_counter(&self) -> Felt {
        let key = Word::from([felt!(0), felt!(0), felt!(0), felt!(1)]);
        self.balances.get(&key)
    }
    
    /// Create a private unlock request
    /// Stores commitment and locks assets
    /// Returns new request_id
    pub fn create_unlock_request(
        &self,
        amount: Felt,
        request_commitment: Word,
    ) -> Felt {
        // Lock assets (reduce balance)
        let balance_key = Word::from([felt!(0), felt!(0), felt!(0), felt!(0)]);
        let current_balance: Felt = self.balances.get(&balance_key);
        let new_balance = current_balance - amount;
        self.balances.set(balance_key, new_balance);
        
        // Get and increment request counter
        let counter_key = Word::from([felt!(0), felt!(0), felt!(0), felt!(1)]);
        let request_id: Felt = self.balances.get(&counter_key);
        let new_counter = request_id + felt!(1);
        self.balances.set(counter_key, new_counter);
        
        // Store the commitment
        let commitment_key = Word::from([request_id, felt!(0), felt!(0), felt!(0)]);
        self.unlock_requests.set(commitment_key, request_commitment[0]);
        
        // Store the locked amount
        let amount_key = Word::from([request_id, felt!(2), felt!(0), felt!(0)]);
        self.unlock_requests.set(amount_key, amount);
        
        request_id
    }
    
    /// Get an unlock request commitment by ID
    pub fn get_request_commitment(&self, request_id: Felt) -> Felt {
        let key = Word::from([request_id, felt!(0), felt!(0), felt!(0)]);
        self.unlock_requests.get(&key)
    }
    
    /// Get locked amount for a request
    pub fn get_request_amount(&self, request_id: Felt) -> Felt {
        let key = Word::from([request_id, felt!(2), felt!(0), felt!(0)]);
        self.unlock_requests.get(&key)
    }
    
    /// Mark request as matched by storing LP commitment
    pub fn mark_request_matched(
        &self,
        request_id: Felt,
        lp_commitment: Word,
    ) -> Felt {
        let lp_key = Word::from([request_id, felt!(1), felt!(0), felt!(0)]);
        self.unlock_requests.set(lp_key, lp_commitment[0]);
        felt!(1)
    }
    
    /// Get LP commitment for a matched request
    pub fn get_lp_commitment(&self, request_id: Felt) -> Felt {
        let key = Word::from([request_id, felt!(1), felt!(0), felt!(0)]);
        self.unlock_requests.get(&key)
    }
    
    /// Check if request is matched (LP commitment is non-zero)
    pub fn is_request_matched(&self, request_id: Felt) -> Felt {
        let lp_commitment = self.get_lp_commitment(request_id);
        if lp_commitment == felt!(0) {
            felt!(0)
        } else {
            felt!(1)
        }
    }
    
    /// Cancel an unmatched request and return assets
    pub fn cancel_request(&self, request_id: Felt) -> Felt {
        // Get locked amount
        let amount = self.get_request_amount(request_id);
        
        // Clear the request
        let commitment_key = Word::from([request_id, felt!(0), felt!(0), felt!(0)]);
        self.unlock_requests.set(commitment_key, felt!(0));
        
        let amount_key = Word::from([request_id, felt!(2), felt!(0), felt!(0)]);
        self.unlock_requests.set(amount_key, felt!(0));
        
        // Return locked assets
        let balance_key = Word::from([felt!(0), felt!(0), felt!(0), felt!(0)]);
        let current: Felt = self.balances.get(&balance_key);
        self.balances.set(balance_key, current + amount);
        
        felt!(1)
    }
    
    // =========================================================================
    // SETTLEMENT
    // =========================================================================
    
    /// Mark request as settled
    pub fn mark_settled(&self, request_id: Felt) -> Felt {
        let settled_key = Word::from([request_id, felt!(3), felt!(0), felt!(0)]);
        self.unlock_requests.set(settled_key, felt!(1));
        felt!(1)
    }
    
    /// Check if request is settled
    pub fn is_settled(&self, request_id: Felt) -> Felt {
        let settled_key = Word::from([request_id, felt!(3), felt!(0), felt!(0)]);
        self.unlock_requests.get(&settled_key)
    }
    
    // =========================================================================
    // PRICING HELPERS (using fixed values for simplicity)
    // 5% advance fee = amount * 5 / 100
    // =========================================================================
    
    /// Calculate 5% advance fee
    /// fee = amount * 5 / 100
    pub fn calculate_fee(&self, amount: Felt) -> Felt {
        // Simple calculation: (amount * 5) / 100
        // Since we can't do division directly, we approximate
        // For 5%, we do amount / 20
        amount / felt!(20)
    }
    
    /// Calculate net advance after 5% fee
    pub fn calculate_net_advance(&self, amount: Felt) -> Felt {
        let fee = self.calculate_fee(amount);
        amount - fee
    }
}
