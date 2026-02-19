//! Voile Protocol - Unit Tests
//! Tests the off-chain matching logic and pricing calculations

use integration::voile_helpers::{
    cooldown_end_timestamp, LpOffer, MatchingEngine, PricingCalculator, UnlockRequest,
    DEFAULT_COOLDOWN_SECONDS, LP_FEE_BPS, ONE_USDC, PROTOCOL_FEE_BPS,
};

use miden_client::account::{AccountId, AccountStorageMode, AccountType};
use miden_protocol::account::AccountIdVersion;
use rand::rngs::StdRng;
use rand::SeedableRng;

fn mock_account_id() -> AccountId {
    // Create a mock account ID for testing
    AccountId::dummy(
        [0u8; 15],
        AccountIdVersion::Version0,
        AccountType::RegularAccountImmutableCode,
        AccountStorageMode::Public,
    )
}

#[test]
fn test_pricing_calculator_fee() {
    let principal = 3000 * ONE_USDC; // $3,000

    // 5% fee = $150
    let fee = PricingCalculator::advance_fee(principal);
    assert_eq!(fee, 150 * ONE_USDC);

    // Net advance = $2,850
    let net = PricingCalculator::net_advance(principal);
    assert_eq!(net, 2850 * ONE_USDC);
}

#[test]
fn test_pricing_calculator_apr() {
    let principal = 3000 * ONE_USDC; // $3,000

    // APR interest for 14 days at 10%
    let interest = PricingCalculator::apr_interest(principal, 14);
    // = 3000 * 1000 * 14 / (10000 * 365) ≈ $11.50
    assert!(interest > 11 * ONE_USDC && interest < 12 * ONE_USDC);
}

#[test]
fn test_fee_split() {
    let total_fee = 100 * ONE_USDC;

    // LP gets 80%
    let lp_share = PricingCalculator::lp_fee_share(total_fee);
    assert_eq!(lp_share, 80 * ONE_USDC);

    // Protocol gets 20%
    let protocol_share = PricingCalculator::protocol_fee_share(total_fee);
    assert_eq!(protocol_share, 20 * ONE_USDC);
}

#[test]
fn test_unlock_request_creation() {
    let mut rng = StdRng::seed_from_u64(42);
    let account_id = mock_account_id();
    let request_amount = 10_000 * ONE_USDC;
    let cooldown_end = cooldown_end_timestamp(DEFAULT_COOLDOWN_SECONDS);

    let request = UnlockRequest::new(1, request_amount, cooldown_end, account_id, &mut rng);

    assert_eq!(request.request_id, 1);
    assert_eq!(request.amount, request_amount);
    assert_eq!(request.cooldown_end_timestamp, cooldown_end);

    // Verify fee calculations
    let expected_fee = (request_amount * 500) / 10000; // 5%
    assert_eq!(request.advance_fee(), expected_fee);
    assert_eq!(request.net_advance(), request_amount - expected_fee);
}

#[test]
fn test_lp_offer_creation() {
    let account_id = mock_account_id();

    let offer = LpOffer::new(
        1,
        account_id,
        100_000 * ONE_USDC, // max
        1_000 * ONE_USDC,   // min
        Some(900),          // 9% APR
    );

    assert_eq!(offer.offer_id, 1);
    assert_eq!(offer.max_amount, 100_000 * ONE_USDC);
    assert_eq!(offer.min_amount, 1_000 * ONE_USDC);
    assert_eq!(offer.custom_apr_bps, Some(900));
    assert!(offer.is_active);

    // Test matching criteria
    assert!(offer.can_match(50_000 * ONE_USDC));
    assert!(offer.can_match(1_000 * ONE_USDC)); // min
    assert!(offer.can_match(100_000 * ONE_USDC)); // max
    assert!(!offer.can_match(999 * ONE_USDC)); // below min
    assert!(!offer.can_match(100_001 * ONE_USDC)); // above max
}

#[test]
fn test_matching_engine() {
    let mut rng = StdRng::seed_from_u64(42);
    let user_account_id = mock_account_id();
    let lp_account_id = mock_account_id();

    // Create offers with different APRs
    let offer1 = LpOffer::new(1, lp_account_id, 100_000 * ONE_USDC, 1_000 * ONE_USDC, Some(1000)); // 10%
    let offer2 = LpOffer::new(2, lp_account_id, 100_000 * ONE_USDC, 1_000 * ONE_USDC, Some(800)); // 8%
    let offer3 = LpOffer::new(3, lp_account_id, 50_000 * ONE_USDC, 5_000 * ONE_USDC, Some(900)); // 9%

    let mut engine = MatchingEngine::new();
    engine.add_offer(offer1);
    engine.add_offer(offer2);
    engine.add_offer(offer3);

    // Create request
    let request_amount = 25_000 * ONE_USDC;
    let cooldown_end = cooldown_end_timestamp(DEFAULT_COOLDOWN_SECONDS);
    let request = UnlockRequest::new(1, request_amount, cooldown_end, user_account_id, &mut rng);

    // Find matches
    let matches = engine.find_matches(&request);
    assert_eq!(matches.len(), 3); // All three should match

    // Best match should be offer2 (lowest APR)
    assert_eq!(matches[0].offer_id, 2);
    assert_eq!(matches[0].custom_apr_bps, Some(800));

    // Match request
    let deal = engine.match_request(request.clone(), &mut rng);
    assert!(deal.is_some());

    let deal = deal.unwrap();
    assert_eq!(deal.offer.offer_id, 2); // Should match best offer
    assert_eq!(deal.advance_amount, request.net_advance());
}

#[test]
fn test_matched_deal_earnings() {
    let mut rng = StdRng::seed_from_u64(42);
    let user_account_id = mock_account_id();
    let lp_account_id = mock_account_id();

    let offer = LpOffer::new(1, lp_account_id, 100_000 * ONE_USDC, 1_000 * ONE_USDC, None);

    let request_amount = 10_000 * ONE_USDC;
    let cooldown_end = cooldown_end_timestamp(DEFAULT_COOLDOWN_SECONDS);
    let request = UnlockRequest::new(1, request_amount, cooldown_end, user_account_id, &mut rng);

    let mut engine = MatchingEngine::new();
    engine.add_offer(offer);

    let deal = engine.match_request(request, &mut rng).unwrap();

    // LP earnings
    let (lp_fee, interest) = deal.lp_earnings(14);
    let total_fee = (10_000 * ONE_USDC * 500) / 10000; // 5% = $500
    let expected_lp_fee = (total_fee * LP_FEE_BPS) / 10000; // 80% = $400
    assert_eq!(lp_fee, expected_lp_fee);

    // Protocol earnings
    let protocol_fee = deal.protocol_earnings();
    let expected_protocol_fee = (total_fee * PROTOCOL_FEE_BPS) / 10000; // 20% = $100
    assert_eq!(protocol_fee, expected_protocol_fee);
}

#[test]
fn test_no_matching_offers() {
    let mut rng = StdRng::seed_from_u64(42);
    let user_account_id = mock_account_id();
    let lp_account_id = mock_account_id();

    // Offer with small range
    let offer = LpOffer::new(1, lp_account_id, 5_000 * ONE_USDC, 1_000 * ONE_USDC, None);

    let mut engine = MatchingEngine::new();
    engine.add_offer(offer);

    // Request larger than offer max
    let request_amount = 10_000 * ONE_USDC;
    let cooldown_end = cooldown_end_timestamp(DEFAULT_COOLDOWN_SECONDS);
    let request = UnlockRequest::new(1, request_amount, cooldown_end, user_account_id, &mut rng);

    let matches = engine.find_matches(&request);
    assert!(matches.is_empty());

    let deal = engine.match_request(request, &mut rng);
    assert!(deal.is_none());
}
