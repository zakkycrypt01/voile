# Voile Protocol - TypeScript Web Client

Private early-liquidity SDK for Miden.

## Installation

```bash
npm install @voile/web-client
```

## Quick Start

```typescript
import { VoileSDK, createVoileSDK } from '@voile/web-client';

async function main() {
  // Initialize SDK
  const sdk = createVoileSDK();
  await sdk.initialize();

  // Create accounts
  const userAccount = await sdk.createUserAccount();
  const lpAccount = await sdk.createLpPoolAccount(100_000);

  // LP creates offer
  sdk.createLpOffer(lpAccount, 50_000, 500, 9); // 9% APR

  // User creates private unlock request
  const request = sdk.createUnlockRequest(userAccount, 10_000, 14);

  // Execute (find match + receive advance)
  const deal = await sdk.executeUnlockRequest(request);
  
  console.log(`Received: $${deal.advanceAmount} USDC`);
}
```

## API Reference

### VoileSDK

Main SDK class for interacting with Voile Protocol.

#### User Methods

```typescript
// Create user account
createUserAccount(): Promise<AccountId>

// Preview pricing (without submitting)
previewUnlockRequest(amountUsdc: number, cooldownDays?: number): PricingPreview

// Create unlock request (private, stays on device)
createUnlockRequest(userAccountId: AccountId, amountUsdc: number, cooldownDays?: number): UnlockRequest

// Execute full flow (match + advance)
executeUnlockRequest(request: UnlockRequest): Promise<MatchedDeal | null>
```

#### LP Methods

```typescript
// Create LP pool with USDC
createLpPoolAccount(initialUsdcAmount: number): Promise<AccountId>

// Create liquidity offer
createLpOffer(
  lpAccountId: AccountId,
  maxAmountUsdc: number,
  minAmountUsdc?: number,
  customAprPercent?: number
): LpOffer

// Cancel offer
cancelLpOffer(offerId: bigint): void

// Execute settlement (after cooldown)
executeSettlement(deal: MatchedDeal): Promise<boolean>
```

### Pricing Calculator

```typescript
import { calculatePricingBreakdown, estimatePricing } from '@voile/web-client';

// Quick estimate
const estimate = estimatePricing(10_000, 14);
console.log(estimate.fee);        // 500 (5%)
console.log(estimate.netAdvance); // 9500
console.log(estimate.interest);   // ~38

// Full breakdown
const breakdown = calculatePricingBreakdown(10_000_000_000n, 14n);
console.log(breakdown.effectiveApy); // ~142%
```

### Matching Engine

```typescript
import { MatchingEngine, buildUnlockRequest, buildLpOffer } from '@voile/web-client';

const engine = new MatchingEngine();

// Add LP offers
engine.addOffer(buildLpOffer('lp1', 100_000n, 1_000n, 9));
engine.addOffer(buildLpOffer('lp2', 50_000n, 500n, 10));

// Create request
const request = buildUnlockRequest('user1', 25_000n, 14);

// Find matches (sorted by best APR)
const matches = engine.findMatches(request);

// Match with best offer
const deal = engine.matchRequest(request);
```

### Crypto Utilities

```typescript
import {
  generateNullifierSecret,
  computeRequestCommitment,
  currentTimestamp,
  remainingCooldown,
} from '@voile/web-client';

// Generate random nullifier
const secret = generateNullifierSecret();

// Compute commitment (for zk-proof)
const commitment = computeRequestCommitment(
  amount,
  cooldownEnd,
  secret,
  userAccountId
);

// Check cooldown status
const remaining = remainingCooldown(cooldownEnd);
console.log(`${remaining.days}d ${remaining.hours}h remaining`);
```

## Configuration

Default parameters:

| Parameter | Value |
|-----------|-------|
| Advance Fee | 5% (500 bps) |
| APR | 10% (1000 bps) |
| Cooldown | 14 days |
| Protocol Fee | 20% |
| LP Fee | 80% |

Override in config:

```typescript
import { DEFAULT_ADVANCE_FEE_BPS, DEFAULT_APR_BPS } from '@voile/web-client';
```

## Privacy

All operations are designed for privacy:

- **Unlock requests** stay on-device until matched
- **Matching** happens off-chain
- **Notes** are encrypted (NoteType.Private)
- **Commitments** reveal nothing about underlying data

## Development

```bash
# Install dependencies
npm install

# Build
npm run build

# Run demo
npm run demo

# Run tests
npm test
```

## License

MIT
