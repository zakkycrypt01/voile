// Voile Protocol - Mock USDC Faucet
// Fungible token faucet for testing purposes
#![no_std]

use miden::{component, felt, Felt, StorageMap, StorageMapAccess, Word};

/// Mock USDC Faucet
/// 
/// Storage layout:
/// Slot 0 (state):
///   - [0, 0, 0, 0] -> total supply
///   - [0, 0, 0, 1] -> max mint per request
#[component]
struct MockUsdcFaucet {
    #[storage(slot(0), description = "faucet state")]
    state: StorageMap,
}

#[component]
impl MockUsdcFaucet {
    // =========================================================================
    // TOKEN INFO
    // =========================================================================
    
    /// Get total supply
    pub fn total_supply(&self) -> Felt {
        let key = Word::from([felt!(0), felt!(0), felt!(0), felt!(0)]);
        self.state.get(&key)
    }
    
    // =========================================================================
    // MINTING
    // =========================================================================
    
    /// Mint USDC tokens
    /// Returns new total supply
    pub fn mint(&self, amount: Felt) -> Felt {
        let supply_key = Word::from([felt!(0), felt!(0), felt!(0), felt!(0)]);
        let current_supply: Felt = self.state.get(&supply_key);
        let new_supply = current_supply + amount;
        self.state.set(supply_key, new_supply);
        new_supply
    }
    
    /// Burn USDC tokens
    /// Returns new total supply
    pub fn burn(&self, amount: Felt) -> Felt {
        let supply_key = Word::from([felt!(0), felt!(0), felt!(0), felt!(0)]);
        let current_supply: Felt = self.state.get(&supply_key);
        let new_supply = current_supply - amount;
        self.state.set(supply_key, new_supply);
        new_supply
    }
    
    /// Set max mint per request
    pub fn set_max_mint(&self, max_amount: Felt) -> Felt {
        let key = Word::from([felt!(0), felt!(0), felt!(0), felt!(1)]);
        self.state.set(key, max_amount);
        felt!(1)
    }
    
    /// Get max mint per request
    pub fn get_max_mint(&self) -> Felt {
        let key = Word::from([felt!(0), felt!(0), felt!(0), felt!(1)]);
        self.state.get(&key)
    }
}
