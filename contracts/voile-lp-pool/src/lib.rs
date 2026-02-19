// Voile Protocol - LP Pool Contract
// Manages stablecoin liquidity and LP offers for private matching
#![no_std]

extern crate alloc;

use miden::{component, felt, Felt, StorageMap, StorageMapAccess, StorageValue, Word};

// Storage slot indices
const SLOT_USDC_BALANCE: u8 = 0;      // Pool's USDC balance
const SLOT_ACTIVE_OFFERS: u8 = 1;     // Map of active LP offers
const SLOT_MATCHED_DEALS: u8 = 2;     // Map of matched deals pending settlement
const SLOT_SETTLED_DEALS: u8 = 3;     // Map of completed settlements
const SLOT_TOTAL_EARNED: u8 = 4;      // Total fees + interest earned
const SLOT_OFFER_COUNTER: u8 = 5;     // Counter for offer IDs

// Protocol fee split: 20% to Voile, 80% to LP
const PROTOCOL_FEE_BPS: u64 = 2000;   // 20%
const LP_FEE_BPS: u64 = 8000;         // 80%

/// LP Pool - holds USDC and manages liquidity offers
#[component]
struct VoileLpPool {
    /// USDC balance available for advances
    #[storage(slot(0), description = "USDC balance")]
    usdc_balance: StorageValue,
    
    /// Active offers map (offer_id -> offer details commitment)
    #[storage(slot(1), description = "active LP offers")]
    active_offers: StorageMap,
    
    /// Matched deals awaiting settlement
    #[storage(slot(2), description = "matched deals")]
    matched_deals: StorageMap,
    
    /// Completed settlements
    #[storage(slot(3), description = "settled deals")]
    settled_deals: StorageMap,
    
    /// Total earnings (fees + interest)
    #[storage(slot(4), description = "total earned")]
    total_earned: StorageValue,
    
    /// Offer ID counter
    #[storage(slot(5), description = "offer counter")]
    offer_counter: StorageValue,
}

#[component]
impl VoileLpPool {
    // =========================================================================
    // LIQUIDITY MANAGEMENT
    // =========================================================================
    
    /// Get current USDC balance
    pub fn get_usdc_balance(&self) -> Felt {
        self.usdc_balance.get()
    }
    
    /// Deposit USDC into the pool
    pub fn deposit_usdc(&self, amount: Felt) -> Felt {
        let current = self.usdc_balance.get();
        let new_balance = current + amount;
        self.usdc_balance.set(new_balance);
        new_balance
    }
    
    /// Withdraw USDC from the pool (only available balance)
    pub fn withdraw_usdc(&self, amount: Felt) -> bool {
        let current = self.usdc_balance.get();
        if current.as_int() < amount.as_int() {
            return false;
        }
        self.usdc_balance.set(current - amount);
        true
    }
    
    /// Get total earnings
    pub fn get_total_earned(&self) -> Felt {
        self.total_earned.get()
    }
    
    // =========================================================================
    // OFFER MANAGEMENT
    // =========================================================================
    
    /// Get current offer counter
    pub fn get_offer_counter(&self) -> Felt {
        self.offer_counter.get()
    }
    
    /// Create a new LP offer
    /// 
    /// Offer specifies how much USDC the LP is willing to advance
    /// and under what terms (stored as commitment for privacy)
    ///
    /// # Arguments
    /// * `max_amount` - Maximum USDC to advance
    /// * `min_amount` - Minimum USDC to advance  
    /// * `offer_commitment` - Hash of full offer details
    ///
    /// # Returns
    /// * Offer ID
    pub fn create_offer(
        &self,
        max_amount: Felt,
        min_amount: Felt,
        offer_commitment: Word,
    ) -> Felt {
        // Verify pool has sufficient balance
        let balance = self.usdc_balance.get();
        assert!(balance.as_int() >= max_amount.as_int(), "Insufficient balance for offer");
        
        // Get next offer ID
        let offer_id = self.offer_counter.get();
        let new_counter = offer_id + felt!(1);
        self.offer_counter.set(new_counter);
        
        // Store offer commitment
        let offer_key = Word::from([offer_id, felt!(0), felt!(0), felt!(0)]);
        self.active_offers.set(offer_key, offer_commitment[0]);
        
        // Store max amount (for quick filtering)
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
    
    /// Get offer details
    pub fn get_offer(&self, offer_id: Felt) -> (Felt, Felt, Felt, bool) {
        let offer_key = Word::from([offer_id, felt!(0), felt!(0), felt!(0)]);
        let commitment = self.active_offers.get(&offer_key);
        
        let max_key = Word::from([offer_id, felt!(1), felt!(0), felt!(0)]);
        let max_amount = self.active_offers.get(&max_key);
        
        let min_key = Word::from([offer_id, felt!(2), felt!(0), felt!(0)]);
        let min_amount = self.active_offers.get(&min_key);
        
        let active_key = Word::from([offer_id, felt!(3), felt!(0), felt!(0)]);
        let is_active = self.active_offers.get(&active_key) == felt!(1);
        
        (commitment, max_amount, min_amount, is_active)
    }
    
    /// Cancel an active offer
    pub fn cancel_offer(&self, offer_id: Felt) -> bool {
        let active_key = Word::from([offer_id, felt!(3), felt!(0), felt!(0)]);
        let is_active = self.active_offers.get(&active_key) == felt!(1);
        
        if !is_active {
            return false;
        }
        
        // Mark as inactive
        self.active_offers.set(active_key, felt!(0));
        true
    }
    
    // =========================================================================
    // MATCHING & DEAL EXECUTION
    // =========================================================================
    
    /// Accept a match with a user's unlock request
    /// 
    /// This locks USDC for the advance and records the deal
    ///
    /// # Arguments
    /// * `offer_id` - The LP offer being used
    /// * `user_request_commitment` - User's request commitment
    /// * `advance_amount` - USDC to advance (after fees)
    /// * `settlement_note_hash` - Hash of the settlement note
    /// * `deal_id` - Unique deal identifier
    pub fn accept_match(
        &self,
        offer_id: Felt,
        user_request_commitment: Word,
        advance_amount: Felt,
        settlement_note_hash: Word,
        deal_id: Word,
    ) -> bool {
        // Verify offer is active
        let active_key = Word::from([offer_id, felt!(3), felt!(0), felt!(0)]);
        let is_active = self.active_offers.get(&active_key) == felt!(1);
        if !is_active {
            return false;
        }
        
        // Verify amount is within offer bounds
        let max_key = Word::from([offer_id, felt!(1), felt!(0), felt!(0)]);
        let max_amount = self.active_offers.get(&max_key);
        let min_key = Word::from([offer_id, felt!(2), felt!(0), felt!(0)]);
        let min_amount = self.active_offers.get(&min_key);
        
        if advance_amount.as_int() > max_amount.as_int() || 
           advance_amount.as_int() < min_amount.as_int() {
            return false;
        }
        
        // Lock USDC for advance
        let balance = self.usdc_balance.get();
        if balance.as_int() < advance_amount.as_int() {
            return false;
        }
        self.usdc_balance.set(balance - advance_amount);
        
        // Record matched deal
        let deal_key = Word::from([deal_id[0], felt!(0), felt!(0), felt!(0)]);
        self.matched_deals.set(deal_key, user_request_commitment[0]);
        
        // Store advance amount
        let amount_key = Word::from([deal_id[0], felt!(1), felt!(0), felt!(0)]);
        self.matched_deals.set(amount_key, advance_amount);
        
        // Store settlement note hash
        let settle_key = Word::from([deal_id[0], felt!(2), felt!(0), felt!(0)]);
        self.matched_deals.set(settle_key, settlement_note_hash[0]);
        
        // Store offer ID used
        let offer_key = Word::from([deal_id[0], felt!(3), felt!(0), felt!(0)]);
        self.matched_deals.set(offer_key, offer_id);
        
        true
    }
    
    /// Get deal details
    pub fn get_deal(&self, deal_id: Felt) -> (Felt, Felt, Felt, Felt) {
        let deal_key = Word::from([deal_id, felt!(0), felt!(0), felt!(0)]);
        let request_commitment = self.matched_deals.get(&deal_key);
        
        let amount_key = Word::from([deal_id, felt!(1), felt!(0), felt!(0)]);
        let advance_amount = self.matched_deals.get(&amount_key);
        
        let settle_key = Word::from([deal_id, felt!(2), felt!(0), felt!(0)]);
        let settlement_hash = self.matched_deals.get(&settle_key);
        
        let offer_key = Word::from([deal_id, felt!(3), felt!(0), felt!(0)]);
        let offer_id = self.matched_deals.get(&offer_key);
        
        (request_commitment, advance_amount, settlement_hash, offer_id)
    }
    
    // =========================================================================
    // SETTLEMENT
    // =========================================================================
    
    /// Record settlement completion
    /// Called when staked assets are received from user
    ///
    /// # Arguments
    /// * `deal_id` - The deal being settled
    /// * `staked_assets_received` - Amount of staked assets received
    /// * `fee_earned` - Advance fee earned
    /// * `interest_earned` - APR interest earned
    pub fn record_settlement(
        &self,
        deal_id: Felt,
        staked_assets_received: Felt,
        fee_earned: Felt,
        interest_earned: Felt,
    ) -> bool {
        // Verify deal exists
        let deal_key = Word::from([deal_id, felt!(0), felt!(0), felt!(0)]);
        let request = self.matched_deals.get(&deal_key);
        if request == felt!(0) {
            return false;
        }
        
        // Calculate LP share of fees (80%)
        let lp_fee = felt!((fee_earned.as_int() * LP_FEE_BPS) / 10000);
        let total_lp_earnings = lp_fee + interest_earned;
        
        // Update total earned
        let current_earned = self.total_earned.get();
        self.total_earned.set(current_earned + total_lp_earnings);
        
        // Mark deal as settled
        let settled_key = Word::from([deal_id, felt!(0), felt!(0), felt!(0)]);
        self.settled_deals.set(settled_key, staked_assets_received);
        
        // Store earnings breakdown
        let fee_key = Word::from([deal_id, felt!(1), felt!(0), felt!(0)]);
        self.settled_deals.set(fee_key, lp_fee);
        
        let interest_key = Word::from([deal_id, felt!(2), felt!(0), felt!(0)]);
        self.settled_deals.set(interest_key, interest_earned);
        
        true
    }
    
    /// Check if a deal is settled
    pub fn is_deal_settled(&self, deal_id: Felt) -> bool {
        let settled_key = Word::from([deal_id, felt!(0), felt!(0), felt!(0)]);
        self.settled_deals.get(&settled_key) != felt!(0)
    }
    
    // =========================================================================
    // PROTOCOL FEE HELPERS
    // =========================================================================
    
    /// Calculate protocol's share of fees (20%)
    pub fn calculate_protocol_fee(&self, total_fee: Felt) -> Felt {
        felt!((total_fee.as_int() * PROTOCOL_FEE_BPS) / 10000)
    }
    
    /// Calculate LP's share of fees (80%)
    pub fn calculate_lp_fee(&self, total_fee: Felt) -> Felt {
        felt!((total_fee.as_int() * LP_FEE_BPS) / 10000)
    }
}
