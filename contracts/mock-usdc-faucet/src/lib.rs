// Voile Protocol - Mock USDC Faucet
// Fungible token faucet for testing purposes
#![no_std]

use miden::{component, felt, Felt, StorageValue, Word};

// USDC has 6 decimals, so 1 USDC = 1_000_000
const USDC_DECIMALS: u64 = 6;
const ONE_USDC: u64 = 1_000_000;

// Max supply: 1 billion USDC
const MAX_SUPPLY: u64 = 1_000_000_000 * ONE_USDC;

/// Mock USDC Faucet
/// 
/// This is a fungible token faucet that mints mock USDC for testing.
/// In production, this would be replaced with real USDC or bridged stablecoins.
#[component]
struct MockUsdcFaucet {
    /// Total supply minted
    #[storage(slot(0), description = "total supply")]
    total_supply: StorageValue,
    
    /// Max mintable per request (anti-abuse)
    #[storage(slot(1), description = "max mint per request")]
    max_mint: StorageValue,
}

#[component]
impl MockUsdcFaucet {
    // =========================================================================
    // TOKEN INFO
    // =========================================================================
    
    /// Get token symbol (USDC)
    pub fn symbol(&self) -> Felt {
        // "USDC" encoded as felt
        felt!(0x55534443) // ASCII: U=85, S=83, D=68, C=67
    }
    
    /// Get token decimals
    pub fn decimals(&self) -> Felt {
        felt!(USDC_DECIMALS)
    }
    
    /// Get total supply
    pub fn total_supply(&self) -> Felt {
        self.total_supply.get()
    }
    
    /// Get max supply
    pub fn max_supply(&self) -> Felt {
        felt!(MAX_SUPPLY)
    }
    
    // =========================================================================
    // MINTING
    // =========================================================================
    
    /// Mint USDC tokens
    /// 
    /// # Arguments
    /// * `amount` - Amount to mint (in smallest units, 6 decimals)
    ///
    /// # Returns
    /// * New total supply
    pub fn mint(&self, amount: Felt) -> Felt {
        let current_supply = self.total_supply.get();
        let new_supply = current_supply + amount;
        
        // Check max supply
        assert!(
            new_supply.as_int() <= MAX_SUPPLY,
            "Exceeds max supply"
        );
        
        // Check per-request limit
        let max_per_request = self.max_mint.get();
        if max_per_request.as_int() > 0 {
            assert!(
                amount.as_int() <= max_per_request.as_int(),
                "Exceeds max mint per request"
            );
        }
        
        self.total_supply.set(new_supply);
        new_supply
    }
    
    /// Set max mint per request (admin function)
    pub fn set_max_mint(&self, max_amount: Felt) {
        self.max_mint.set(max_amount);
    }
    
    /// Get max mint per request
    pub fn get_max_mint(&self) -> Felt {
        self.max_mint.get()
    }
    
    // =========================================================================
    // HELPERS
    // =========================================================================
    
    /// Convert USDC display amount to raw amount
    /// e.g., 100 USDC -> 100_000_000
    pub fn to_raw_amount(&self, display_amount: Felt) -> Felt {
        felt!(display_amount.as_int() * ONE_USDC)
    }
    
    /// Convert raw amount to display amount
    /// e.g., 100_000_000 -> 100 USDC
    pub fn to_display_amount(&self, raw_amount: Felt) -> Felt {
        felt!(raw_amount.as_int() / ONE_USDC)
    }
}
