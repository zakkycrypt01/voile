//! Voile Protocol - Testnet Demo Script
//! 
//! Demonstrates the full end-to-end flow on Miden testnet:
//! 1. Create user and LP accounts
//! 2. Mint mock USDC
//! 3. LP creates offer
//! 4. User creates private unlock request
//! 5. Off-chain matching
//! 6. Instant USDC advance
//! 7. Settlement after cooldown

use integration::helpers::{
    build_project_in_dir, create_account_from_package, create_basic_wallet_account,
    create_note_from_package, setup_client, AccountCreationConfig, ClientSetup, NoteCreationConfig,
};
use integration::voile_helpers::{
    UnlockRequest, LpOffer, MatchedDeal, MatchingEngine, PricingCalculator,
    DEFAULT_COOLDOWN_SECONDS, ONE_USDC, USDC_DECIMALS,
    current_timestamp, cooldown_end_timestamp,
};

use anyhow::{Context, Result};
use miden_client::{
    account::StorageMap,
    note::{NoteType, NoteAssets},
    transaction::{OutputNote, TransactionRequestBuilder},
    Felt, Word,
};
use rand::{rngs::StdRng, SeedableRng, RngCore};
use std::{path::Path, sync::Arc};

fn print_header(text: &str) {
    println!("\n{'═'.repeat(60)}");
    println!("║ {:<56} ║", text);
    println!("{'═'.repeat(60)}");
}

fn print_step(num: u32, text: &str) {
    println!("\n━━━ Step {}: {} ━━━\n", num, text);
}

#[tokio::main]
async fn main() -> Result<()> {
    println!("╔════════════════════════════════════════════════════════════╗");
    println!("║           VOILE PROTOCOL - Miden Testnet Demo              ║");
    println!("║              Private Early Liquidity System                ║");
    println!("╚════════════════════════════════════════════════════════════╝");
    
    let mut rng = StdRng::seed_from_u64(42);
    
    // =========================================================================
    // STEP 1: Connect to testnet and setup client
    // =========================================================================
    print_step(1, "Connecting to Miden Testnet");
    
    let ClientSetup { mut client, keystore } = setup_client().await?;
    
    let sync_summary = client.sync_state().await?;
    println!("✓ Connected to Miden Testnet");
    println!("  Latest block: {}", sync_summary.block_num);
    
    // =========================================================================
    // STEP 2: Build Voile contracts
    // =========================================================================
    print_step(2, "Building Voile Contracts");
    
    let user_account_package = Arc::new(
        build_project_in_dir(Path::new("../contracts/voile-user-account"), true)
            .context("Failed to build voile-user-account")?
    );
    println!("✓ Built voile-user-account");
    
    let lp_pool_package = Arc::new(
        build_project_in_dir(Path::new("../contracts/voile-lp-pool"), true)
            .context("Failed to build voile-lp-pool")?
    );
    println!("✓ Built voile-lp-pool");
    
    let settlement_note_package = Arc::new(
        build_project_in_dir(Path::new("../contracts/settlement-note"), true)
            .context("Failed to build settlement-note")?
    );
    println!("✓ Built settlement-note");
    
    let advance_note_package = Arc::new(
        build_project_in_dir(Path::new("../contracts/advance-note"), true)
            .context("Failed to build advance-note")?
    );
    println!("✓ Built advance-note");
    
    // =========================================================================
    // STEP 3: Create accounts
    // =========================================================================
    print_step(3, "Creating Accounts");
    
    // Initial balances
    let initial_staked_balance = 100_000 * ONE_USDC;
    let initial_usdc_balance = 500_000 * ONE_USDC;
    
    // Create user account
    let staked_balance_key = Word::from([Felt::new(0), Felt::new(0), Felt::new(0), Felt::new(1)]);
    let user_cfg = AccountCreationConfig {
        storage_slots: vec![miden_client::account::StorageSlot::Map(
            StorageMap::with_entries([(
                staked_balance_key,
                Word::from([Felt::new(initial_staked_balance), Felt::new(0), Felt::new(0), Felt::new(0)]),
            )])?,
        )],
        ..Default::default()
    };
    
    let user_account = create_account_from_package(&mut client, user_account_package.clone(), user_cfg).await?;
    println!("✓ User account created: {:?}", user_account.id().to_hex());
    println!("  Staked balance: {} tokens", initial_staked_balance / ONE_USDC);
    
    // Create LP pool
    let usdc_balance_key = Word::from([Felt::new(0), Felt::new(0), Felt::new(0), Felt::new(0)]);
    let lp_cfg = AccountCreationConfig {
        storage_slots: vec![miden_client::account::StorageSlot::Map(
            StorageMap::with_entries([(
                usdc_balance_key,
                Word::from([Felt::new(initial_usdc_balance), Felt::new(0), Felt::new(0), Felt::new(0)]),
            )])?,
        )],
        ..Default::default()
    };
    
    let lp_account = create_account_from_package(&mut client, lp_pool_package.clone(), lp_cfg).await?;
    println!("✓ LP pool created: {:?}", lp_account.id().to_hex());
    println!("  USDC balance: {} USDC", initial_usdc_balance / ONE_USDC);
    
    // Create sender wallet (for publishing notes)
    let sender_account = create_basic_wallet_account(&mut client, keystore.clone(), AccountCreationConfig::default()).await?;
    println!("✓ Sender wallet: {:?}", sender_account.id().to_hex());
    
    // =========================================================================
    // STEP 4: LP creates offer
    // =========================================================================
    print_step(4, "LP Creates Liquidity Offer");
    
    let lp_offer = LpOffer::new(
        1, // offer_id
        lp_account.id(),
        100_000 * ONE_USDC, // max $100,000
        1_000 * ONE_USDC,   // min $1,000
        Some(900), // 9% APR (better than default 10%)
    );
    
    println!("✓ LP offer created");
    println!("  Range: {} - {} USDC", 
        lp_offer.min_amount / ONE_USDC,
        lp_offer.max_amount / ONE_USDC
    );
    println!("  APR: 9%");
    
    // =========================================================================
    // STEP 5: User creates private unlock request
    // =========================================================================
    print_step(5, "User Creates Private Unlock Request");
    
    let request_amount = 25_000 * ONE_USDC; // $25,000
    let cooldown_end = cooldown_end_timestamp(DEFAULT_COOLDOWN_SECONDS);
    
    let unlock_request = UnlockRequest::new(
        1,
        request_amount,
        cooldown_end,
        user_account.id(),
        &mut rng,
    );
    
    println!("✓ Unlock request created (PRIVATE - not broadcast)");
    println!("  Request ID: {:?}", unlock_request.request_id);
    println!("  Amount: {} USDC", request_amount / ONE_USDC);
    println!("  Cooldown: 14 days");
    println!();
    println!("  ⚠️  This request is PRIVATE:");
    println!("      - No on-chain broadcast");
    println!("      - No mempool exposure");  
    println!("      - No intent leakage");
    
    // Show pricing
    let fee = unlock_request.advance_fee();
    let net = unlock_request.net_advance();
    let interest = unlock_request.apr_interest(14);
    
    println!();
    println!("  Pricing:");
    println!("  ├─ Advance fee (5%): {} USDC", fee / ONE_USDC);
    println!("  ├─ Net advance: {} USDC", net / ONE_USDC);
    println!("  └─ APR interest (14d): {} USDC", interest / ONE_USDC);
    
    // =========================================================================
    // STEP 6: Off-chain matching
    // =========================================================================
    print_step(6, "Off-Chain Matching");
    
    let mut matching_engine = MatchingEngine::new();
    matching_engine.add_offer(lp_offer.clone());
    
    println!("🔍 Finding best LP match...");
    
    let matched_deal = matching_engine.match_request(unlock_request.clone(), &mut rng)
        .expect("Failed to match request");
    
    println!("✓ Match found!");
    println!("  Deal ID: {:?}", matched_deal.deal_id[0]);
    println!("  LP: {:?}", lp_offer.lp_account_id.to_hex());
    println!("  Advance: {} USDC", matched_deal.advance_amount / ONE_USDC);
    
    let (lp_fee_share, lp_interest) = matched_deal.lp_earnings(14);
    println!();
    println!("  LP earnings:");
    println!("  ├─ Fee share (80%): {} USDC", lp_fee_share / ONE_USDC);
    println!("  └─ Interest: {} USDC", lp_interest / ONE_USDC);
    
    // =========================================================================
    // STEP 7: Create and publish settlement note
    // =========================================================================
    print_step(7, "Creating Settlement Note");
    
    let settlement_config = NoteCreationConfig {
        note_type: NoteType::Private, // ENCRYPTED
        inputs: vec![
            Felt::new(unlock_request.request_id),
            Felt::new(unlock_request.amount),
            Felt::new(unlock_request.cooldown_end_timestamp),
            matched_deal.deal_id[0],
        ],
        ..Default::default()
    };
    
    let settlement_note = create_note_from_package(
        &mut client,
        settlement_note_package.clone(),
        user_account.id(),
        settlement_config,
    )?;
    
    println!("✓ Settlement note created");
    println!("  Note ID: {:?}", settlement_note.id().to_hex());
    println!("  Type: Private (encrypted)");
    println!("  Executes: After cooldown ends");
    
    // Publish note
    let publish_request = TransactionRequestBuilder::new()
        .own_output_notes(vec![OutputNote::Full(settlement_note.clone())])
        .build()?;
    
    let publish_tx = client
        .submit_new_transaction(sender_account.id(), publish_request)
        .await?;
    
    println!("✓ Settlement note published: {:?}", publish_tx.to_hex());
    
    // =========================================================================
    // STEP 8: Create and publish advance note (USDC transfer)
    // =========================================================================
    print_step(8, "Creating Advance Note (USDC Transfer)");
    
    let advance_config = NoteCreationConfig {
        note_type: NoteType::Private, // ENCRYPTED
        inputs: vec![
            Felt::new(matched_deal.advance_amount),
            matched_deal.deal_id[0],
            Felt::new(matched_deal.offer.offer_id),
            unlock_request.commitment[0],
        ],
        ..Default::default()
    };
    
    let advance_note = create_note_from_package(
        &mut client,
        advance_note_package.clone(),
        lp_account.id(),
        advance_config,
    )?;
    
    println!("✓ Advance note created");
    println!("  Note ID: {:?}", advance_note.id().to_hex());
    println!("  USDC amount: {} USDC", matched_deal.advance_amount / ONE_USDC);
    println!("  Recipient: User account");
    
    // Publish note
    let publish_request = TransactionRequestBuilder::new()
        .own_output_notes(vec![OutputNote::Full(advance_note.clone())])
        .build()?;
    
    let publish_tx = client
        .submit_new_transaction(sender_account.id(), publish_request)
        .await?;
    
    println!("✓ Advance note published: {:?}", publish_tx.to_hex());
    
    // =========================================================================
    // SUMMARY
    // =========================================================================
    print_header("DEMO COMPLETE");
    
    println!("\n  USER OUTCOME:");
    println!("  ├─ Staked assets locked: {} tokens", request_amount / ONE_USDC);
    println!("  ├─ Fee paid: {} USDC", fee / ONE_USDC);
    println!("  └─ USDC received NOW: {} USDC", net / ONE_USDC);
    
    println!("\n  LP OUTCOME (after 14 days):");
    println!("  ├─ USDC advanced: {} USDC", net / ONE_USDC);
    println!("  ├─ Fee earned: {} USDC", lp_fee_share / ONE_USDC);
    println!("  ├─ Interest earned: {} USDC", lp_interest / ONE_USDC);
    println!("  └─ Staked assets received: {} tokens", request_amount / ONE_USDC);
    
    println!("\n  PROTOCOL OUTCOME:");
    println!("  └─ Fee earned: {} USDC", matched_deal.protocol_earnings() / ONE_USDC);
    
    println!("\n  PRIVACY:");
    println!("  ├─ User intent: HIDDEN ✓");
    println!("  ├─ Amount: ENCRYPTED ✓");
    println!("  ├─ Timing: PRIVATE ✓");
    println!("  └─ Matching: OFF-CHAIN ✓");
    
    let remaining_seconds = unlock_request.cooldown_end_timestamp - current_timestamp();
    let remaining_days = remaining_seconds / (24 * 60 * 60);
    let remaining_hours = (remaining_seconds % (24 * 60 * 60)) / (60 * 60);
    
    println!("\n  ⏰ Settlement in: {}d {}h", remaining_days, remaining_hours);
    
    println!("\n╔════════════════════════════════════════════════════════════╗");
    println!("║  Zero Intent Leakage • Private Matching • Instant USDC    ║");
    println!("╚════════════════════════════════════════════════════════════╝\n");
    
    Ok(())
}
