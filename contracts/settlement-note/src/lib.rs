// Voile Protocol - Settlement Note Script
// Executes automatic repayment when cooldown ends
// Transfers staked assets from user to LP
#![no_std]

use miden::*;

use crate::bindings::miden::voile_user_account::voile_user_account;
use crate::bindings::miden::voile_lp_pool::voile_lp_pool;

/// Settlement Note Script
/// 
/// This note is consumed by the LP pool account after the cooldown period ends.
/// It verifies the cooldown has elapsed and transfers staked assets to the LP.
///
/// Note inputs (private):
/// - [0]: request_id - The unlock request ID
/// - [1]: amount - Staked asset amount to transfer
/// - [2]: cooldown_end_timestamp - When cooldown ends
/// - [3]: fee_amount - Advance fee amount
/// - [4]: interest_amount - APR interest amount
/// - [5]: deal_id - The matched deal ID
/// - [6]: current_timestamp - Current block timestamp (provided at execution)
#[note_script]
fn run(note_inputs: Word) {
    // Extract note inputs
    // In production, these would come from encrypted note data
    let request_id = note_inputs[0];
    let amount = note_inputs[1];
    let cooldown_end_timestamp = note_inputs[2];
    let deal_id = note_inputs[3];
    
    // Get current timestamp (in production, from block header)
    // For now, we use a simple check that cooldown_end is in the past
    let current_timestamp = get_current_timestamp();
    
    // Verify cooldown has ended
    assert!(
        current_timestamp.as_int() >= cooldown_end_timestamp.as_int(),
        "Cooldown period has not ended"
    );
    
    // Authorize settlement on user account
    let authorized = voile_user_account::authorize_settlement(
        request_id,
        amount,
        current_timestamp,
        cooldown_end_timestamp,
    );
    assert!(authorized, "Settlement not authorized");
    
    // Record settlement on LP pool
    // Fee and interest would be calculated from the original deal terms
    let fee_amount = calculate_fee(amount);
    let interest_amount = calculate_interest(amount, cooldown_end_timestamp);
    
    let settled = voile_lp_pool::record_settlement(
        deal_id,
        amount,
        fee_amount,
        interest_amount,
    );
    assert!(settled, "Failed to record settlement");
    
    // Transfer staked assets to LP
    // In production, this would use Miden's asset transfer primitives
    // The assets are already locked in the user account
}

/// Get current timestamp
/// In production, this would read from block header or transaction context
fn get_current_timestamp() -> Felt {
    // Placeholder: would be injected by the Miden VM at execution time
    Felt::from_u64(0)
}

/// Calculate advance fee (5% = 500 bps)
fn calculate_fee(amount: Felt) -> Felt {
    Felt::from_u64((amount.as_int() * 500) / 10000)
}

/// Calculate interest based on cooldown duration
fn calculate_interest(amount: Felt, cooldown_end: Felt) -> Felt {
    // Simplified: 10% APR for 14 days
    // interest = amount * 0.10 * (14/365)
    let apr_bps: u64 = 1000; // 10%
    let days: u64 = 14;
    let interest = (amount.as_int() * apr_bps * days) / (10000 * 365);
    Felt::from_u64(interest)
}
