// Voile Protocol - Advance Note Script
// Transfers USDC from LP pool to user (instant stablecoin payout)
#![no_std]

use miden::*;

use crate::bindings::miden::voile_lp_pool::voile_lp_pool;

/// Advance Note Script
///
/// This note is created by the LP and consumed by the user account.
/// It transfers USDC to the user after a successful match.
///
/// The note is PRIVATE - only the user can see the amount and terms.
///
/// Note inputs (encrypted/private):
/// - [0]: advance_amount - Net USDC to transfer (after fees)
/// - [1]: deal_id - The matched deal identifier
/// - [2]: offer_id - The LP offer that was matched
/// - [3]: user_commitment - User's request commitment (for verification)
#[note_script]
fn run(note_inputs: Word) {
    // Extract note inputs
    let advance_amount = note_inputs[0];
    let deal_id = note_inputs[1];
    let offer_id = note_inputs[2];
    let user_commitment = note_inputs[3];
    
    // Verify the deal exists and matches
    let (stored_commitment, stored_amount, _, stored_offer) = voile_lp_pool::get_deal(deal_id);
    
    // Verify deal details match
    assert_eq!(stored_commitment, user_commitment, "Deal commitment mismatch");
    assert_eq!(stored_amount, advance_amount, "Advance amount mismatch");
    assert_eq!(stored_offer, offer_id, "Offer ID mismatch");
    
    // Transfer USDC to user
    // In production, this would:
    // 1. Verify the note recipient matches the user
    // 2. Use Miden's native asset transfer to move USDC
    // 3. The assets are attached to this note and released on consumption
    
    // The note's assets (USDC) are automatically transferred to the
    // consuming account (user) when this script executes successfully
}
