// Voile Protocol - Advance Note Script
// Transfers USDC from LP pool to user
#![no_std]

use miden::*;

/// Advance Note Script
///
/// Note inputs:
/// - [0]: advance_amount
/// - [1]: deal_id
/// - [2]: offer_id
/// - [3]: user_commitment
///
/// This note is created when an LP matches a user's unlock request.
/// Consuming this note transfers the USDC advance to the user.
#[note_script]
fn run(note_inputs: Word) {
    let _advance_amount = note_inputs[0];
    let _deal_id = note_inputs[1];
    let _offer_id = note_inputs[2];
    let _user_commitment = note_inputs[3];
    
    // In a full implementation, this would:
    // 1. Verify the deal exists in LP pool
    // 2. Verify amounts match
    // 3. The USDC assets attached to this note are automatically
    //    transferred to the consuming account
    //
    // For now, we just validate the note can be consumed
}
