/**
 * Voile Protocol - Demo Script
 * 
 * Demonstrates the full end-to-end flow:
 * 1. Create user and LP accounts
 * 2. LP creates offer
 * 3. User creates private unlock request
 * 4. Off-chain matching
 * 5. Instant USDC advance
 * 6. Settlement after cooldown
 */

import {
  VoileSDK,
  createVoileSDK,
  rawToUsdc,
  remainingCooldown,
  formatTimestamp,
  ONE_USDC,
} from './index';

async function main() {
  console.log('╔════════════════════════════════════════════════════════════╗');
  console.log('║           VOILE PROTOCOL - Private Early Liquidity         ║');
  console.log('║                     Demo on Miden Testnet                  ║');
  console.log('╚════════════════════════════════════════════════════════════╝');
  console.log();
  
  // Initialize SDK
  const sdk = createVoileSDK();
  await sdk.initialize();
  
  // =========================================================================
  // STEP 1: Create accounts
  // =========================================================================
  console.log('\n━━━ Step 1: Create Accounts ━━━\n');
  
  const userAccountId = await sdk.createUserAccount();
  console.log(`  User Account: ${userAccountId}`);
  
  const lpAccountId = await sdk.createLpPoolAccount(100_000);
  console.log(`  LP Pool Account: ${lpAccountId} (100,000 USDC)`);
  
  // =========================================================================
  // STEP 2: LP creates offer
  // =========================================================================
  console.log('\n━━━ Step 2: LP Creates Liquidity Offer ━━━\n');
  
  const offer = sdk.createLpOffer(
    lpAccountId,
    50_000,  // max 50,000 USDC
    500,     // min 500 USDC
    9,       // 9% APR (slightly better than default 10%)
  );
  
  // =========================================================================
  // STEP 3: User previews pricing
  // =========================================================================
  console.log('\n━━━ Step 3: User Previews Pricing ━━━\n');
  
  const previewAmount = 10_000; // $10,000
  const preview = sdk.previewUnlockRequest(previewAmount, 14);
  
  console.log(`  Principal: $${previewAmount.toLocaleString()} USDC`);
  console.log(`  Advance Fee (5%): $${preview.fee.toLocaleString()} USDC`);
  console.log(`  Net Advance: $${preview.netAdvance.toLocaleString()} USDC`);
  console.log(`  APR Interest (14 days): $${preview.interest.toFixed(2)} USDC`);
  
  // =========================================================================
  // STEP 4: User creates private unlock request
  // =========================================================================
  console.log('\n━━━ Step 4: Create Private Unlock Request ━━━\n');
  
  const request = sdk.createUnlockRequest(
    userAccountId,
    previewAmount,
    14, // 14 day cooldown
  );
  
  console.log('\n  ⚠️  This request is PRIVATE:');
  console.log('      - No public broadcast');
  console.log('      - No mempool exposure');
  console.log('      - No intent leakage');
  
  // =========================================================================
  // STEP 5: Execute unlock (match + advance)
  // =========================================================================
  console.log('\n━━━ Step 5: Execute Unlock Request ━━━\n');
  
  const deal = await sdk.executeUnlockRequest(request);
  
  if (!deal) {
    console.log('❌ Failed to execute unlock request');
    return;
  }
  
  // =========================================================================
  // STEP 6: Summary
  // =========================================================================
  console.log('\n━━━ Final Summary ━━━\n');
  
  console.log('  USER OUTCOME:');
  console.log(`    ├─ Staked assets locked: $${rawToUsdc(deal.stakedAmount).toLocaleString()} USDC equivalent`);
  console.log(`    ├─ Fee paid: $${rawToUsdc(deal.advanceFee).toLocaleString()} USDC`);
  console.log(`    └─ USDC received NOW: $${rawToUsdc(deal.advanceAmount).toLocaleString()} USDC`);
  
  console.log('\n  LP OUTCOME (after settlement):');
  console.log(`    ├─ USDC advanced: $${rawToUsdc(deal.advanceAmount).toLocaleString()} USDC`);
  console.log(`    ├─ Fee earned: $${rawToUsdc(deal.advanceFee * 8000n / 10000n).toLocaleString()} USDC`);
  console.log(`    ├─ Interest earned: $${rawToUsdc(deal.expectedInterest).toFixed(2)} USDC`);
  console.log(`    └─ Staked assets received: $${rawToUsdc(deal.stakedAmount).toLocaleString()} worth`);
  
  console.log('\n  PROTOCOL OUTCOME:');
  console.log(`    └─ Fee earned: $${rawToUsdc(deal.advanceFee * 2000n / 10000n).toLocaleString()} USDC`);
  
  console.log('\n  PRIVACY:');
  console.log('    ├─ User intent: HIDDEN');
  console.log('    ├─ Amount: ENCRYPTED');
  console.log('    ├─ Timing: PRIVATE');
  console.log('    └─ Matching: OFF-CHAIN');
  
  const remaining = remainingCooldown(deal.cooldownEndTimestamp);
  console.log(`\n  ⏰ Settlement in: ${remaining.days}d ${remaining.hours}h ${remaining.minutes}m`);
  console.log(`     (${formatTimestamp(deal.cooldownEndTimestamp)})`);
  
  // =========================================================================
  // DONE
  // =========================================================================
  console.log('\n╔════════════════════════════════════════════════════════════╗');
  console.log('║                    ✓ Demo Complete!                        ║');
  console.log('╠════════════════════════════════════════════════════════════╣');
  console.log('║  Zero Intent Leakage • Private Matching • Instant USDC    ║');
  console.log('╚════════════════════════════════════════════════════════════╝');
  
  await sdk.disconnect();
}

// Run demo
main().catch(console.error);
