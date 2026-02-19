# Voile Protocol

**Private Early Liquidity on Miden**

Voile Protocol enables users to access liquidity from staked/locked assets before the cooldown period ends, with complete privacy. Built on Miden's zero-knowledge architecture.

## 🎯 Problem

In DeFi today:
- Unstake/redemption requests are **public**
- Bots can **predict** user behavior
- Cooldowns **delay** access to funds (1-20+ days)
- Large users risk **price impact** and **copy-trading**

## ✨ Solution

Voile enables:
1. **Private Unlock Requests** - Created locally with zk-proofs
2. **Off-chain Matching** - No public broadcast of intent
3. **Instant USDC Advance** - LP provides stablecoins immediately
4. **Automatic Settlement** - Repayment after cooldown via notes

**Zero intent leakage.** No one knows who is unlocking, how much, or when.

## 🏗️ Architecture

```
┌──────────────────────────────────────────────────────────────────┐
│                         USER DEVICE                               │
│  ┌─────────────────┐    ┌─────────────────┐    ┌──────────────┐  │
│  │ Create Request  │───▶│ Generate Proof  │───▶│ Local Match  │  │
│  │   (Private)     │    │    (ZK)         │    │  (Off-chain) │  │
│  └─────────────────┘    └─────────────────┘    └──────────────┘  │
└──────────────────────────────────────────────────────────────────┘
                                   │
                                   ▼
┌──────────────────────────────────────────────────────────────────┐
│                       MIDEN NETWORK                               │
│  ┌─────────────────┐    ┌─────────────────┐    ┌──────────────┐  │
│  │  Settlement     │    │  Advance Note   │    │   LP Pool    │  │
│  │     Note        │    │  (USDC→User)    │    │   Account    │  │
│  └─────────────────┘    └─────────────────┘    └──────────────┘  │
└──────────────────────────────────────────────────────────────────┘
```

## **Installation**


Before getting started, ensure you have the following prerequisites:

1. **Install Rust** - Make sure you have Rust installed on your system. If not, install it from [rustup.rs](https://rustup.rs/)

2. **Install midenup toolchain** - Follow the installation instructions at: <https://github.com/0xMiden/midenup>

## **Project Structure**

```text
voile/
├── contracts/
│   ├── voile-user-account/      # User account with unlock requests
│   ├── voile-lp-pool/           # LP pool holding USDC
│   ├── settlement-note/         # Auto-repayment after cooldown
│   ├── advance-note/            # USDC transfer to user
│   ├── mock-usdc-faucet/        # Mock USDC for testing
│   ├── counter-account/         # Example counter contract
│   └── increment-note/          # Example note script
├── integration/
│   ├── src/
│   │   ├── bin/
│   │   │   ├── voile_demo.rs    # Testnet demo script
│   │   │   └── increment_count.rs
│   │   ├── helpers.rs           # Miden helpers
│   │   ├── voile_helpers.rs     # Voile-specific helpers
│   │   └── lib.rs
│   └── tests/
│       ├── voile_e2e_test.rs    # End-to-end tests
│       └── counter_test.rs
├── web-client/                  # TypeScript SDK
│   └── src/
│       ├── index.ts             # Main SDK
│       ├── matching.ts          # Off-chain matching
│       ├── pricing.ts           # Fee calculations
│       ├── crypto.ts            # Cryptographic utilities
│       └── demo.ts              # Browser demo
├── Cargo.toml
└── voile.txt                    # PRD document
```

## 💰 **Pricing Model**

| Component | Rate | Example ($3,000) |
|-----------|------|------------------|
| Advance Fee | 5% | $150 |
| APR (14 days) | 10% | $11.50 |
| **Net Advance** | | **$2,850** |

**Fee Split:** LP gets 80%, Protocol gets 20%

## 🚀 **Quick Start**

### Build Contracts

```bash
# Build all Voile contracts
cargo miden build --manifest-path contracts/voile-user-account/Cargo.toml
cargo miden build --manifest-path contracts/voile-lp-pool/Cargo.toml
cargo miden build --manifest-path contracts/settlement-note/Cargo.toml
cargo miden build --manifest-path contracts/advance-note/Cargo.toml
```

### Run E2E Tests

```bash
cd integration
cargo test test_voile_e2e_flow -- --nocapture
```

### Run Testnet Demo

```bash
cd integration
cargo run --bin voile_demo
```

### TypeScript Client

```bash
cd web-client
npm install
npm run demo
```

## 📚 **Usage Example (TypeScript)**

```typescript
import { VoileSDK, createVoileSDK } from '@voile/web-client';

const sdk = createVoileSDK();
await sdk.initialize();

// Create accounts
const user = await sdk.createUserAccount();
const lp = await sdk.createLpPoolAccount(100_000);

// LP creates offer
sdk.createLpOffer(lp, 50_000, 500, 9); // 9% APR

// User creates PRIVATE unlock request
const request = sdk.createUnlockRequest(user, 10_000, 14);

// Execute (find match + receive USDC)
const deal = await sdk.executeUnlockRequest(request);
console.log(`Received: $${deal.advanceAmount} USDC immediately!`);
```

## 🔐 **Privacy Guarantees**

| What | Visibility |
|------|------------|
| Who is unlocking | **HIDDEN** |
| Unlock amount | **ENCRYPTED** |
| Timing | **PRIVATE** |
| LP matching | **OFF-CHAIN** |
| Settlement | **ZK-VERIFIED** |

## **Design Philosophy**


This workspace follows a clean separation of concerns:

### **Contracts Folder - Miden Development**

The `contracts/` folder is your primary working directory when writing Miden smart contract code. Each contract is organized as its own individual crate, allowing for:

- Independent versioning and dependencies
- Clear isolation between different contracts
- Easy contract management and modularization

When you're working on Miden Rust code (writing smart contracts), you'll be working in the `contracts/` directory.

### **Integration Crate - Scripts and Testing**

The `integration/` crate is your working directory for interacting with compiled contracts. All on-chain interactions, scripts, and tests are housed within this single crate. This includes:

- **Binaries** (`src/bin/`): Rust executables for deploying and interacting with your contracts on-chain
- **Tests** (`tests/`): Integration tests for validating your contract behavior

This structure provides flexibility as your application grows, allowing you to add custom dependencies, sophisticated tooling, and independent configuration specific to your deployment and testing needs.

> **Important Note**: The `helpers.rs` file inside the `integration/` crate is temporary and exists only to facilitate current development workflows. **Do not modify this file unless you know what you are doing!** It will be removed in future versions.

## **Adding New Contracts**

To create a new contract crate, run the following command from the workspace root:

```bash
miden cargo-miden new --account contracts/my-account
```

This will scaffold a new contract crate inside the `contracts/` directory with all the necessary boilerplate.

## **Adding Binaries for On-Chain Interactions**

Binaries are used for deploying contracts and performing on-chain interactions. To add a new binary:

1. Create a new `.rs` file in `integration/src/bin/` (e.g., `deploy_contract.rs`)
2. Write your binary code as a standard Rust executable with a `main()` function
3. Run the binary using the commands shown below

## **Testing Your Contracts**

Tests are located in `integration/tests/`. To add a new test:

1. Create a new test file in `integration/tests/` (e.g., `my_contract_test.rs`)
2. Write your test functions using the standard Rust testing framework
3. Run tests using the commands shown below

## **Commands**

### Compile a Contract

```bash
# Compile a specific contract
miden cargo-miden build --manifest-path contracts/counter-account/Cargo.toml

# Or navigate to the contract directory
cd contracts/counter-account
miden cargo-miden build
```

### Run a Binary

```bash
# Navigate to integration crate and run a binary
cd integration
cargo run --bin increment_count
```

### Run Tests

```bash
# Navigate to integration crate and run tests
cd integration
cargo test                      # Run all tests
cargo test counter_test         # Run specific test file
```

## **Extending the Workspace**

If you need to extend the workspace with new crates (for example, to add libraries or additional tools), it is recommended to add these new crates in the root of the project directory. This helps keep the project structure clean and makes it easier to manage dependencies and workspace configuration.

To add a new crate to the workspace:

1. From the project root, run:
   ```bash
   cargo new my-new-crate
   ```
2. Then add the crate path (e.g., `my-new-crate`) to the `[workspace].members` section of your `Cargo.toml`.

**Note:** Avoid adding new crates as subdirectories under `contracts/` or `integration/`, unless they are intended to be contract crates or part of integration specifically. Keeping new crates at the root makes the project easier to understand and maintain.
