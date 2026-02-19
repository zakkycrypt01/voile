//! Voile Protocol - Testnet Demo Script
//!
//! Demonstrates the full end-to-end flow on Miden testnet:
//! 1. Connect to testnet
//! 2. Create user and LP accounts
//! 3. User creates private unlock request
//! 4. Off-chain matching
//! 5. Instant USDC advance

use integration::helpers::{
    create_basic_wallet_account, setup_client, AccountCreationConfig, ClientSetup,
};
use integration::voile_helpers::{
    cooldown_end_timestamp, current_timestamp, LpOffer, MatchingEngine, UnlockRequest,
    DEFAULT_COOLDOWN_SECONDS, ONE_USDC,
};

use anyhow::Result;
use rand::{rngs::StdRng, SeedableRng};

fn print_header(text: &str) {
    println!("\n{}", "═".repeat(60));
    println!("║ {:<56} ║", text);
    println!("{}", "═".repeat(60));
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

    let ClientSetup {
        mut client,
        keystore,
    } = setup_client().await?;

    let sync_summary = client.sync_state().await?;
    println!("✓ Connected to Miden Testnet");
    println!("  Latest block: {}", sync_summary.block_num);

    // =========================================================================
    // STEP 2: Create accounts
    // =========================================================================
    print_step(2, "Creating Accounts");

    // Create user wallet
    let user_account = create_basic_wallet_account(
        &mut client,
        keystore.clone(),
        AccountCreationConfig::default(),
    )
    .await?;
    println!("✓ User wallet created: {:?}", user_account.id());

    // Create LP wallet
    let lp_account = create_basic_wallet_account(
        &mut client,
        keystore.clone(),
        AccountCreationConfig::default(),
    )
    .await?;
    println!("✓ LP wallet created: {:?}", lp_account.id());

    // =========================================================================
    // STEP 3: LP creates offer
    // =========================================================================
    print_step(3, "LP Creates Liquidity Offer");

    let lp_offer = LpOffer::new(
        1, // offer_id
        lp_account.id(),
        100_000 * ONE_USDC, // max $100,000
        1_000 * ONE_USDC,   // min $1,000
        Some(900),          // 9% APR (better than default 10%)
    );

    println!("✓ LP offer created");
    println!(
        "  Range: {} - {} USDC",
        lp_offer.min_amount / ONE_USDC,
        lp_offer.max_amount / ONE_USDC
    );
    println!("  APR: 9%");

    // =========================================================================
    // STEP 4: User creates private unlock request
    // =========================================================================
    print_step(4, "User Creates Private Unlock Request");

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
    // STEP 5: Off-chain matching
    // =========================================================================
    print_step(5, "Off-Chain Matching");

    let mut matching_engine = MatchingEngine::new();
    matching_engine.add_offer(lp_offer.clone());

    println!("🔍 Finding best LP match...");

    let matched_deal = matching_engine
        .match_request(unlock_request.clone(), &mut rng)
        .expect("Failed to match request");

    println!("✓ Match found!");
    println!("  Deal ID: {:?}", matched_deal.deal_id[0]);
    println!("  LP: {:?}", lp_offer.lp_account_id);
    println!(
        "  Advance: {} USDC",
        matched_deal.advance_amount / ONE_USDC
    );

    let (lp_fee_share, lp_interest) = matched_deal.lp_earnings(14);
    println!();
    println!("  LP earnings:");
    println!("  ├─ Fee share (80%): {} USDC", lp_fee_share / ONE_USDC);
    println!("  └─ Interest: {} USDC", lp_interest / ONE_USDC);

    // =========================================================================
    // SUMMARY
    // =========================================================================
    print_header("DEMO COMPLETE");

    println!("\n  USER OUTCOME:");
    println!(
        "  ├─ Staked assets locked: {} tokens",
        request_amount / ONE_USDC
    );
    println!("  ├─ Fee paid: {} USDC", fee / ONE_USDC);
    println!("  └─ USDC received NOW: {} USDC", net / ONE_USDC);

    println!("\n  LP OUTCOME (after 14 days):");
    println!("  ├─ USDC advanced: {} USDC", net / ONE_USDC);
    println!("  ├─ Fee earned: {} USDC", lp_fee_share / ONE_USDC);
    println!("  ├─ Interest earned: {} USDC", lp_interest / ONE_USDC);
    println!(
        "  └─ Staked assets received: {} tokens",
        request_amount / ONE_USDC
    );

    println!("\n  PROTOCOL OUTCOME:");
    println!(
        "  └─ Fee earned: {} USDC",
        matched_deal.protocol_earnings() / ONE_USDC
    );

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
