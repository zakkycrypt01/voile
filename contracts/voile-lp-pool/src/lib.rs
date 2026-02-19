// Voile Protocol - LP Pool Contract
// Manages stablecoin liquidity and LP offers for private matching
#![no_std]

use miden::{component, felt, Felt, StorageMap, StorageMapAccess, Word};

/// LP Pool - holds USDC and manages liquidity offers
/// 
/// Storage layout:
/// Slot 0 (balances):
///   - [0, 0, 0, 0] -> USDC balance
///   - [0, 0, 0, 1] -> total earned fees
///   - [0, 0, 0, 2] -> offer counter
///   - [0, 0, 0, 3] -> deal counter
/// 
/// Slot 1 (active_offers):
///   - [offer_id, 0, 0, 0] -> offer commitment
///   - [offer_id, 1, 0, 0] -> max amount
///   - [offer_id, 2, 0, 0] -> min amount
///   - [offer_id, 3, 0, 0] -> is active (1 or 0)
/// 
/// Slot 2 (matched_deals):
///   - [deal_id, 0, 0, 0] -> user request commitment
///   - [deal_id, 1, 0, 0] -> advance amount
///   - [deal_id, 2, 0, 0] -> offer id
///   - [deal_id, 3, 0, 0] -> settled flag
#[component]
struct VoileLpPool {
    #[storage(slot(0), description = "balances")]
    balances: StorageMap,
    
    #[storage(slot(1), description = "active LP offers")]
    active_offers: StorageMap,
    
    #[storage(slot(2), description = "matched deals")]
    matched_deals: StorageMap,
}

#[component]
impl VoileLpPool {
    // =========================================================================
    // LIQUIDITY MANAGEMENT
    // =========================================================================
    
    /// Get current USDC balance
    pub fn get_usdc_balance(&self) -> Felt {
        let key = Word::from([felt!(0), felt!(0), felt!(0), felt!(0)]);
        self.balances.get(&key)
    }
    
    /// Deposit USDC into the pool
    pub fn deposit_usdc(&self, amount: Felt) -> Felt {
        let key = Word::from([felt!(0), felt!(0), felt!(0), felt!(0)]);
        let current: Felt = self.balances.get(&key);
        let new_balance = current + amount;
        self.balances.set(key, new_balance);
        new_balance
    }
    
    /// Withdraw USDC from the pool
    pub fn withdraw_usdc(&self, amount: Felt) -> Felt {
        let key = Word::from([felt!(0), felt!(0), felt!(0), felt!(0)]);
        let current: Felt = self.balances.get(&key);
        let new_balance = current - amount;
        self.balances.set(key, new_balance);
        new_balance
    }
    
    /// Get total earned fees
    pub fn get_total_earned(&self) -> Felt {
        let key = Word::from([felt!(0), felt!(0), felt!(0), felt!(1)]);
        self.balances.get(&key)
    }
    
    /// Add to total earned
    pub fn add_earnings(&self, amount: Felt) -> Felt {
        let key = Word::from([felt!(0), felt!(0), felt!(0), felt!(1)]);
        let current: Felt = self.balances.get(&key);
        let new_total = current + amount;
        self.balances.set(key, new_total);
        new_total
    }
    
    // =========================================================================
    // OFFER MANAGEMENT
    // =========================================================================
    
    /// Get current offer counter
    pub fn get_offer_counter(&self) -> Felt {
        let key = Word::from([felt!(0), felt!(0), felt!(0), felt!(2)]);
        self.balances.get(&key)
    }
    
    /// Create a new LP offer
    /// Returns offer ID
    pub fn create_offer(
        &self,
        max_amount: Felt,
        min_amount: Felt,
        offer_commitment: Word,
    ) -> Felt {
        // Get and increment offer counter
        let counter_key = Word::from([felt!(0), felt!(0), felt!(0), felt!(2)]);
        let offer_id: Felt = self.balances.get(&counter_key);
        let new_counter = offer_id + felt!(1);
        self.balances.set(counter_key, new_counter);
        
        // Store offer commitment
        let commit_key = Word::from([offer_id, felt!(0), felt!(0), felt!(0)]);
        self.active_offers.set(commit_key, offer_commitment[0]);
        
        // Store max amount
        let max_key = Word::from([offer_id, felt!(1), felt!(0), felt!(0)]);
        self.active_offers.set(max_key, max_amount);
        
        // Store min amount
        let min_key = Word::from([offer_id, felt!(2), felt!(0), felt!(0)]);
        self.active_offers.set(min_key, min_amount);
        
        // Mark as active
        let active_key = Word::from([offer_id, felt!(3), felt!(0), felt!(0)]);
        self.active_offers.set(active_key, felt!(1));
        
        offer_id
    }
    
    /// Get offer max amount
    pub fn get_offer_max(&self, offer_id: Felt) -> Felt {
        let key = Word::from([offer_id, felt!(1), felt!(0), felt!(0)]);
        self.active_offers.get(&key)
    }
    
    /// Get offer min amount
    pub fn get_offer_min(&self, offer_id: Felt) -> Felt {
        let key = Word::from([offer_id, felt!(2), felt!(0), felt!(0)]);
        self.active_offers.get(&key)
    }
    
    /// Check if offer is active
    pub fn is_offer_active(&self, offer_id: Felt) -> Felt {
        let key = Word::from([offer_id, felt!(3), felt!(0), felt!(0)]);
        self.active_offers.get(&key)
    }
    
    /// Cancel an active offer
    pub fn cancel_offer(&self, offer_id: Felt) -> Felt {
        let active_key = Word::from([offer_id, felt!(3), felt!(0), felt!(0)]);
        self.active_offers.set(active_key, felt!(0));
        felt!(1)
    }
    
    // =========================================================================
    // MATCHING & DEAL EXECUTION
    // =========================================================================
    
    /// Get current deal counter
    pub fn get_deal_counter(&self) -> Felt {
        let key = Word::from([felt!(0), felt!(0), felt!(0), felt!(3)]);
        self.balances.get(&key)
    }
    
    /// Accept a match with a user's unlock request
    /// Returns deal_id
    pub fn accept_match(
        &self,
        offer_id: Felt,
        user_request_commitment: Word,
        advance_amount: Felt,
    ) -> Felt {
        // Lock USDC for advance
        let balance_key = Word::from([felt!(0), felt!(0), felt!(0), felt!(0)]);
        let balance: Felt = self.balances.get(&balance_key);
        let new_balance = balance - advance_amount;
        self.balances.set(balance_key, new_balance);
        
        // Get and increment deal counter
        let counter_key = Word::from([felt!(0), felt!(0), felt!(0), felt!(3)]);
        let deal_id: Felt = self.balances.get(&counter_key);
        let new_counter = deal_id + felt!(1);
        self.balances.set(counter_key, new_counter);
        
        // Store deal - user commitment
        let commit_key = Word::from([deal_id, felt!(0), felt!(0), felt!(0)]);
        self.matched_deals.set(commit_key, user_request_commitment[0]);
        
        // Store deal - advance amount
        let amount_key = Word::from([deal_id, felt!(1), felt!(0), felt!(0)]);
        self.matched_deals.set(amount_key, advance_amount);
        
        // Store deal - offer id
        let offer_key = Word::from([deal_id, felt!(2), felt!(0), felt!(0)]);
        self.matched_deals.set(offer_key, offer_id);
        
        deal_id
    }
    
    /// Get deal advance amount
    pub fn get_deal_amount(&self, deal_id: Felt) -> Felt {
        let key = Word::from([deal_id, felt!(1), felt!(0), felt!(0)]);
        self.matched_deals.get(&key)
    }
    
    /// Get deal offer id
    pub fn get_deal_offer(&self, deal_id: Felt) -> Felt {
        let key = Word::from([deal_id, felt!(2), felt!(0), felt!(0)]);
        self.matched_deals.get(&key)
    }
    
    // =========================================================================
    // SETTLEMENT
    // =========================================================================
    
    /// Record settlement completion
    pub fn record_settlement(
        &self,
        deal_id: Felt,
        staked_assets_received: Felt,
        fee_earned: Felt,
    ) -> Felt {
        // Add staked assets to balance (converting to USDC equivalent)
        let balance_key = Word::from([felt!(0), felt!(0), felt!(0), felt!(0)]);
        let balance: Felt = self.balances.get(&balance_key);
        let new_balance = balance + staked_assets_received;
        self.balances.set(balance_key, new_balance);
        
        // Add to earnings (LP gets 80% of fees)
        // fee_earned * 4 / 5 = 80%
        let lp_fee = fee_earned * felt!(4) / felt!(5);
        let earned_key = Word::from([felt!(0), felt!(0), felt!(0), felt!(1)]);
        let current_earned: Felt = self.balances.get(&earned_key);
        self.balances.set(earned_key, current_earned + lp_fee);
        
        // Mark deal as settled
        let settled_key = Word::from([deal_id, felt!(3), felt!(0), felt!(0)]);
        self.matched_deals.set(settled_key, felt!(1));
        
        felt!(1)
    }
    
    /// Check if a deal is settled
    pub fn is_deal_settled(&self, deal_id: Felt) -> Felt {
        let key = Word::from([deal_id, felt!(3), felt!(0), felt!(0)]);
        self.matched_deals.get(&key)
    }
}
