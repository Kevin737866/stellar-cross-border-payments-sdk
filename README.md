# Stellar Cross-Border Payments SDK

A comprehensive SDK for building cross-border payment applications on the Stellar network, featuring time-locked escrow, on-chain exchange rate oracles, and built-in compliance checks.

## 🚀 Features

### Core Features
- **Time-Locked Escrow**: Secure cross-border settlements with automatic release
- **Exchange Rate Oracle**: On-chain aggregated rates for USD/EUR/MXN pairs
- **Compliance Engine**: KYC/AML hooks and regulatory compliance checks
- **Dispute Resolution**: Automated and manual dispute handling
- **Fee Bump Support**: Reliable transactions for cross-border senders

### SDK Components
- **Soroban Contracts**: Rust smart contracts for Stellar
- **TypeScript SDK**: High-level API for contract interaction
- **React Components**: Pre-built UI components for payments
- **Examples**: Complete implementation patterns

## 📋 Table of Contents

- [Installation](#installation)
- [Quick Start](#quick-start)
- [Architecture](#architecture)
- [Contracts](#contracts)
- [TypeScript SDK](#typescript-sdk)
- [React Components](#react-components)
- [Examples](#examples)
- [Configuration](#configuration)
- [API Reference](#api-reference)
- [Building](#building)
- [Testing](#testing)
- [Deployment](#deployment)
- [Contributing](#contributing)

## 🛠 Installation

### Prerequisites
- Node.js 18+ 
- Rust 1.78+ (for contract compilation)
- Stellar CLI 21+ (for contract deployment)

### Install SDK

```bash
# Clone the repository
git clone https://github.com/stellar-cross-border/stellar-cross-border-payments-sdk.git
cd stellar-cross-border-payments-sdk

# Install all workspace dependencies (sdk + cli + ui) in one step
npm install

# Install Rust wasm target
rustup target add wasm32-unknown-unknown
```

### Environment Setup

```bash
cp .env.example .env
# Edit .env with your contract addresses and keys — see docs/deployment.md
```

## ⚡ Quick Start

### 1. Initialize the SDK

```typescript
import { StellarCrossBorderSDK } from '@stellar-cross-border/sdk';

// Configure for testnet
const config = StellarCrossBorderSDK.createTestnetConfig();
const contracts = {
  escrow: 'YOUR_ESCROW_CONTRACT_ADDRESS',
  rateOracle: 'YOUR_RATE_ORACLE_CONTRACT_ADDRESS',
  compliance: 'YOUR_COMPLIANCE_CONTRACT_ADDRESS',
};

const sdk = new StellarCrossBorderSDK(config, contracts);
```

### 2. Create a Cross-Border Payment

```typescript
import { Keypair } from 'stellar-sdk';

// Generate keypairs
const sender = Keypair.random();
const receiver = Keypair.random();

// Create payment
const paymentRequest = {
  from: sender.publicKey(),
  to: receiver.publicKey(),
  amount: '1000', // $1000 USD
  token: 'USDC',
  release_time: Math.floor(Date.now() / 1000) + (24 * 60 * 60), // 24 hours
  metadata: {
    purpose: new TextEncoder().encode('remittance'),
    reference: new TextEncoder().encode('US-MX-2024-001'),
  },
};

const result = await sdk.paymentsInstance.createPayment(paymentRequest, {
  feeBump: true,
  memo: 'Cross-border payment',
});

console.log(`Payment created: ${result.escrowId}`);
```

### 3. Check Exchange Rates

```typescript
const rateResult = await sdk.paymentsInstance.getExchangeRate({
  from_currency: 'USD',
  to_currency: 'MXN',
});

console.log(`1 USD = ${rateResult.rate} MXN`);
```

### 4. Verify Compliance

```typescript
const complianceResult = await sdk.paymentsInstance.checkCompliance({
  from_user: sender.publicKey(),
  to_user: receiver.publicKey(),
  amount: '1000',
  currency: 'USD',
  jurisdiction_from: 'US',
  jurisdiction_to: 'MX',
});

if (complianceResult.approved) {
  console.log('Payment is compliant');
} else {
  console.log(`Compliance check failed: ${complianceResult.reason}`);
}
```

## 🏗 Architecture

```
stellar-cross-border-payments-sdk/
├── src/                    # Soroban contracts (Rust)
│   ├── escrow.rs          # Time-locked escrow logic
│   ├── rate_oracle.rs     # Exchange rate aggregation
│   ├── compliance.rs      # KYC/AML compliance checks
│   └── lib.rs            # Contract exports
├── sdk/                   # TypeScript SDK
│   ├── src/
│   │   ├── client.ts     # Stellar client wrapper
│   │   ├── payments.ts   # High-level payment API
│   │   ├── types.ts      # TypeScript interfaces
│   │   └── index.ts      # Barrel exports
│   └── package.json
├── ui/                    # React components
│   ├── src/
│   │   ├── components/   # Payment UI components
│   │   └── hooks/        # React hooks
│   └── package.json
├── cli/                   # CLI tool (stellar-payout)
│   ├── src/
│   │   ├── commands/     # batch, status, retry, report
│   │   ├── parsers/      # CSV, JSON, XLSX, MT103
│   │   ├── utils/        # Database, validation, logger
│   │   ├── types.ts      # CLI type definitions
│   │   └── index.ts      # CLI entry point
│   └── package.json
├── examples/              # Usage examples
│   ├── usd-to-mxn.ts     # US to Mexico remittance
│   ├── eur-to-usd.ts     # Europe to US business payment
│   ├── escrow-dispute.ts # Dispute resolution
│   ├── payroll-batch.csv  # 50-employee payroll sample
│   └── aid-disbursement.ts # UNHCR-style rapid response
└── README.md
```

## 🔒 Contracts

### Escrow Contract

The escrow contract provides time-locked payment protection:

```rust
// Create escrow
pub fn create_escrow(
    env: Env,
    sender: Address,
    receiver: Address,
    amount: i128,
    token: Address,
    release_time: u64,
    metadata: Map<Symbol, Vec<u8>>,
) -> BytesN<32>

// Release funds
pub fn release_escrow(env: Env, escrow_id: BytesN<32>) -> bool

// Refund payment
pub fn refund_escrow(env: Env, escrow_id: BytesN<32>) -> bool

// Open dispute
pub fn dispute_escrow(
    env: Env,
    escrow_id: BytesN<32>,
    challenger: Address,
    reason: Symbol,
    evidence: Vec<u8>,
) -> bool
```

### Rate Oracle Contract

Aggregates exchange rates from multiple sources:

```rust
// Submit rate
pub fn submit_rate(
    env: Env,
    source: Address,
    from_currency: Symbol,
    to_currency: Symbol,
    rate: u128,
    confidence: u8,
) -> bool

// Get aggregated rate
pub fn get_rate(env: Env, from_currency: Symbol, to_currency: Symbol) -> AggregatedRate
```

### Compliance Contract

Handles KYC/AML checks and regulatory compliance:

```rust
// Check transaction compliance
pub fn check_transaction_compliance(
    env: Env,
    transaction_id: BytesN<32>,
    from_user: Address,
    to_user: Address,
    amount: i128,
    currency: Symbol,
    jurisdiction_from: Symbol,
    jurisdiction_to: Symbol,
) -> ComplianceCheck
```

## 📚 TypeScript SDK

### StellarClient

Low-level Stellar network interaction:

```typescript
const client = new StellarClient(config, contracts);

// Get account info
const account = await client.getAccount('GD...');

// Submit transaction
const result = await client.submitTransaction(transactionXdr);

// Get contract data
const data = await client.getContractData(contractId, key);
```

### StellarPayments

High-level payment operations:

```typescript
const payments = new StellarPayments(client);

// Create payment
const result = await payments.createPayment(request, options);

// Release escrow
await payments.releaseEscrow(escrowId, signer);

// Get exchange rate
const rate = await payments.getExchangeRate({ from_currency: 'USD', to_currency: 'MXN' });

// Check compliance
const compliance = await payments.checkCompliance(request);
```

## ⚛️ React Components

### PaymentForm

Complete payment creation form:

```typescript
import { PaymentForm } from '@stellar-cross-border/ui';

<PaymentForm 
  sdk={sdk}
  onSuccess={(result) => console.log('Payment created:', result)}
  onError={(error) => console.error('Payment failed:', error)}
/>
```

### EscrowStatus

Real-time escrow monitoring:

```typescript
import { EscrowStatusComponent } from '@stellar-cross-border/ui';

<EscrowStatusComponent
  sdk={sdk}
  escrowId="ESCROW_ID_HERE"
  onStatusChange={(status) => console.log('Status changed:', status)}
  showActions={true}
/>
```

### ExchangeRateDisplay

Live exchange rate display:

```typescript
import { ExchangeRateDisplay } from '@stellar-cross-border/ui';

<ExchangeRateDisplay
  sdk={sdk}
  fromCurrency="USD"
  toCurrency="MXN"
  amount="1000"
  autoRefresh={true}
/>
```

### useStellarPayment Hook
A powerful React hook for managing the entire lifecycle of a Stellar cross-border payment.

#### Usage
```typescript
import { useStellarPayment } from '@stellar-cross-border/ui';

const MyComponent = ({ sdk, escrowId }) => {
  const {
    loading,
    error,
    paymentStatus,
    createPayment,
    releaseEscrow,
    refreshStatus,
  } = useStellarPayment(sdk, escrowId, { autoRefresh: true });

  if (loading) return <div>Loading...</div>;
  if (error) return <div>Error: {error}</div>;

  return (
    <div>
      <p>Status: {paymentStatus?.status}</p>
      <button onClick={() => releaseEscrow(escrowId, signer)}>Release Payment</button>
    </div>
  );
};
```

#### API Reference

**Inputs:**
- `sdk`: An instance of `StellarCrossBorderSDK`.
- `escrowId` (optional): The ID of the escrow to monitor.
- `options` (optional):
  - `autoRefresh`: Automatically polls for status updates (default: `true`).
  - `refreshInterval`: Time between polls in milliseconds (default: `30000`).

**Returns:**
An object containing:

**State:**
- `loading`: `boolean` - Indicates if an operation is in progress.
- `error`: `string | null` - Contains error message if an operation fails.
- `paymentStatus`: `PaymentStatus | null` - The current status of the payment.
- `exchangeRate`: `ExchangeRateResult | null` - The latest exchange rate information.
- `complianceCheck`: `ComplianceCheckResult | null` - The result of the compliance check.

**Actions:**
- `createPayment(request, options)`: Initiates a new payment.
- `releaseEscrow(escrowId, signer, options)`: Releases funds from an escrow.
- `refundEscrow(escrowId, signer, options)`: Refunds an escrow.
- `disputeEscrow(escrowId, challenger, reason, evidence, signer, options)`: Opens a dispute.
- `getExchangeRate(request)`: Fetches the latest exchange rate.
- `checkCompliance(request)`: Performs a compliance check.
- `getPaymentStatus(escrowId)`: Fetches the status of a specific escrow.
- `refreshStatus()`: Manually triggers a status refresh.
- `clearError()`: Resets the error state.


## 📖 Examples

### US to Mexico Remittance

```bash
# Run the example
npx ts-node examples/usd-to-mxn.ts
```

This example demonstrates:
- Creating a cross-border remittance
- Exchange rate conversion
- Compliance checking
- Time-locked escrow
- Payment release

### Europe to US Business Payment

```bash
# Run the example
npx ts-node examples/eur-to-usd.ts
```

This example demonstrates:
- B2B payment workflows
- Enhanced compliance for large amounts
- Business metadata handling
- Multi-step approval process

### Escrow Dispute Resolution

```bash
# Run the example
npx ts-node examples/escrow-dispute.ts
```

This example demonstrates:
- Dispute creation
- Evidence collection
- Admin resolution
- Refund processing

## 💻 CLI Tool (stellar-payout)

A purpose-built CLI for processing batch cross-border payments, designed for humanitarian aid organizations, global payroll providers, and neobanks.

### Installation

```bash
# Install globally via npm
cd cli
npm install
npm run build
npm link

# Or run directly
npx stellar-payout --help
```

### Commands

#### `stellar-payout batch` - Process Batch Payments

```bash
# Process payments from CSV
stellar-payout batch --input payments.csv --source-secret $SECRET_KEY --network testnet

# Dry-run mode (simulate without submitting)
stellar-payout batch --input payments.csv --source-secret $SECRET_KEY --dry-run

# Process from JSON
stellar-payout batch --input payments.json --format json --source-secret $SECRET_KEY

# Process from Excel
stellar-payout batch --input payments.xlsx --format xlsx --source-secret $SECRET_KEY

# Process SWIFT MT103 messages
stellar-payout batch --input transfers.mt103 --format mt103 --source-secret $SECRET_KEY

# Advanced options
stellar-payout batch --input payments.csv --source-secret $SECRET_KEY \
  --max-ops 100 \
  --concurrency 5 \
  --fee-surge-threshold 100 \
  --network testnet
```

#### `stellar-payout status` - Real-Time Monitoring

```bash
# Show recent batches
stellar-payout status

# Monitor specific batch
stellar-payout status --batch-id <batch_id>

# Stream real-time updates via Horizon
stellar-payout status --batch-id <batch_id> --follow
```

#### `stellar-payout retry` - Retry Failed Transactions

```bash
# Retry with exponential backoff
stellar-payout retry --batch-id <batch_id> --source-secret $SECRET_KEY

# Custom retry parameters
stellar-payout retry --batch-id <batch_id> --source-secret $SECRET_KEY \
  --max-retries 5 \
  --backoff-base 2000 \
  --backoff-max 60000
```

#### `stellar-payout report` - Compliance Audit Trail

```bash
# Generate CSV report
stellar-payout report --batch-id <batch_id> --format csv

# Generate PDF report
stellar-payout report --batch-id <batch_id> --format pdf

# Custom output path
stellar-payout report --batch-id <batch_id> --format pdf --output audit-report.pdf
```

### Input File Format (CSV)

```csv
destination,amount,asset,memo,escrow_duration
GBDEVU63Y6...,1500.00,USDC,payroll-001,86400
GCFONE23AB...,1200.00,EURC,payroll-002,86400
```

### Key Features

- **Transaction Batching**: Groups up to 100 payments per ledger transaction (Stellar's 100 op limit)
- **Fee Optimization**: Uses FEE_BUMP transactions for sender abstraction
- **Parallel Submission**: Concurrent channels for independent destination corridors
- **Smart Queuing**: Pauses if network congestion (fee surge pricing >100 stroops)
- **Crash Recovery**: SQLite-backed state persistence for interrupted batches
- **Emergency Stop**: SIGINT handling with graceful pause and state preservation
- **Address Validation**: Checks destination exists + trustline before submission
- **Dry-Run Mode**: Simulate all transactions without submission
- **Multi-Format Input**: CSV, JSON, Excel (.xlsx), SWIFT MT103
- **Compliance Reports**: PDF and CSV audit trail generation

### Crash Recovery & Resume

The CLI persists all batch state to a local SQLite database (default: `./stellar-payout.db`).
Atomicity guarantees ensure the database is never left in a half-written state:

| Operation | Atomicity guarantee |
|---|---|
| Batch initialisation | `initBatch()` creates the row **and** sets status `running` in one transaction — no stuck `created` batches |
| Entry seeding | All payment-entry rows inserted in one transaction — either all exist or none do |
| Group confirmation | Group row + every entry row set to `confirmed` together — a crash after Horizon confirms but before the write completes leaves entries in `submitted`, not a false `confirmed` |
| Group failure | Group row + every entry row set to `failed` together |
| Graceful pause | `SIGINT`/`SIGTERM` atomically sets status `paused` and refreshes all counters |

#### Detecting and resuming stale batches

A batch in `running` or `paused` status at startup indicates an interrupted run:

```ts
const stale = db.getBatchesNeedingResume();
// Returns both 'paused' (graceful SIGINT) and 'running' (hard crash) batches
```

To resume:

```ts
db.resumeBatch(batchId);                           // status → running, counters refreshed
const groups = db.getIncompleteGroups(batchId);    // groups not yet confirmed
for (const group of groups) {
  const pending = db.getPendingEntriesByGroup(batchId, group.groupIndex);
  // re-submit pending / submitted entries only — confirmed entries are skipped
}
```

From the CLI, resume failed entries with:

```bash
stellar-payout retry --batch-id <id> --source-secret $SECRET_KEY
```

The `retry` command re-submits every entry whose status is `failed`.
For entries stuck in `submitted` (submitted to Horizon but not yet confirmed),
re-running the batch command with the same `--db-path` will detect the incomplete
groups via `getIncompleteGroups` and re-check Horizon for their status.

## ⚙️ Configuration

### Environment Variables

```bash
# Stellar Network
HORIZON_URL=https://horizon-testnet.stellar.org
SOROBAN_RPC_URL=https://soroban-testnet.stellar.org
NETWORK_PASSPHRASE=Test SDF Network ; September 2015

# Contract Addresses
ESCROW_CONTRACT_ADDRESS=YOUR_ESCROW_CONTRACT_ADDRESS
RATE_ORACLE_CONTRACT_ADDRESS=YOUR_RATE_ORACLE_CONTRACT_ADDRESS
COMPLIANCE_CONTRACT_ADDRESS=YOUR_COMPLIANCE_CONTRACT_ADDRESS

# Admin Configuration
ADMIN_SECRET_KEY=YOUR_ADMIN_SECRET_KEY
ADMIN_PUBLIC_KEY=YOUR_ADMIN_PUBLIC_KEY
```

See [docs/deployment.md](docs/deployment.md) for the full list of environment variables and their defaults.

## 📖 API Reference

### PaymentRequest

```typescript
interface PaymentRequest {
  from: string;
  to: string;
  amount: string;
  token: string;
  release_time?: number;
  metadata?: Record<string, Uint8Array>;
}
```

### PaymentOptions

```typescript
interface PaymentOptions {
  feeBump?: boolean;
  timeout?: number;
  memo?: string;
  submit?: boolean;
}
```

### EscrowCreationResult

```typescript
interface EscrowCreationResult extends TransactionResult {
  escrowId: string;
}
```

### ExchangeRateResult

```typescript
interface ExchangeRateResult {
  rate: string;
  timestamp: number;
  sources: ExchangeRate[];
  aggregated: AggregatedRate;
}
```

### ComplianceCheckResult

```typescript
interface ComplianceCheckResult extends TransactionResult {
  approved: boolean;
  reason: string;
  rulesTriggered: string[];
}
```

## 🧪 Testing

### Run contract tests

```bash
cargo test
# or via root script
npm run test:contracts
```

### Run SDK tests

```bash
npm run test:sdk
# or directly
cd sdk && npm test
```

### Run CLI tests

```bash
npm run test:cli
# or directly
cd cli && npm test
```

### Run all tests

```bash
npm test
```

## Type Safety & Validation

All payment requests should be validated before submission:

```ts
import { validatePaymentRequest } from './src/types';

const request = validatePaymentRequest({
  sender:   'GABC...',
  receiver: 'GDEF...',
  amount:   '100.50',
  asset:    'USDC',
});
```

Branded types (`StellarAddress`, `AmountString`) prevent raw strings from
being passed where validated values are expected.
Metadata must be flat key-value with primitive values only.

## 🚀 Deployment

See **[docs/deployment.md](docs/deployment.md)** for the complete guide, including:

- Compiling the Soroban contracts to WASM
- Deploying to testnet and mainnet with Stellar CLI
- Initialising contract state (admin, restricted jurisdictions, rate sources, compliance records)
- Wiring contract addresses into the SDK and CLI
- Example testnet addresses for quick testing
- Troubleshooting common errors

Quick-start (testnet):

```bash
# 1. Compile contracts
npm run build:contracts

# 2. Generate and fund an admin keypair
stellar keys generate admin --network testnet
stellar keys fund admin --network testnet

# 3. Deploy all three contracts
WASM=target/wasm32-unknown-unknown/release/stellar_cross_border_payments_sdk.wasm
ESCROW=$(stellar contract deploy --wasm "$WASM" --source admin --network testnet)
ORACLE=$(stellar contract deploy --wasm "$WASM" --source admin --network testnet)
COMPLIANCE=$(stellar contract deploy --wasm "$WASM" --source admin --network testnet)

# 4. Write addresses to .env
echo "ESCROW_CONTRACT_ADDRESS=$ESCROW"        >> .env
echo "RATE_ORACLE_CONTRACT_ADDRESS=$ORACLE"   >> .env
echo "COMPLIANCE_CONTRACT_ADDRESS=$COMPLIANCE" >> .env

# 5. Build TypeScript packages
npm run build

# 6. Run a test batch payment
stellar-payout batch --input examples/payroll-batch.csv \
  --source-secret "$ADMIN_SECRET_KEY" --network testnet --dry-run
```

## 🔧 Development

### Watch mode

```bash
npm run dev:sdk   # tsc --watch in sdk/
npm run dev:cli   # tsc --watch in cli/
npm run dev:ui    # next dev in ui/
```

### Code style

- Rust: `cargo fmt` and `cargo clippy`
- TypeScript: ESLint (`npm run lint`) and Prettier
- React: Follow React best practices

## 🏗 Building

The repo uses npm workspaces. All packages can be built from the root.

### Build all packages

```bash
# Full build: Rust contracts → SDK → CLI → UI
npm run build:full

# Or build each layer separately
npm run build:contracts   # cargo build --release --target wasm32-unknown-unknown
npm run build:sdk         # tsc in sdk/
npm run build:cli         # tsc in cli/
npm run build:ui          # next build in ui/
```

### Build scripts (with output summaries)

```bash
# Linux / macOS / CI
./scripts/build.sh

# Windows PowerShell
.\scripts\build.ps1

# Type-check only (no emit)
./scripts/build.sh --type-check
.\scripts\build.ps1 -TypeCheck

# Skip contracts, only build TypeScript
./scripts/build.sh --ts-only
```

### Build order

The three TypeScript packages must be built in this order:

1. **`sdk/`** — compiled first because the CLI workspace resolves the SDK from `sdk/dist`.
2. **`cli/`** — depends on `sdk/dist` being present.
3. **`ui/`** — Next.js build, independent of cli and sdk dist (uses workspace package resolution).

The Rust contracts must be compiled before deploying, but the TypeScript
packages do not depend on the WASM artefacts at compile time.  Contract
addresses are supplied at runtime via environment variables.

### Type-check without building

```bash
npm run type-check          # all packages
npm run type-check:sdk
npm run type-check:cli
npm run type-check:ui
```

## 🤝 Contributing

We welcome contributions! Please see our [Contributing Guide](CONTRIBUTING.md) for details.

1. Fork the repository
2. Create a feature branch (`git checkout -b feat/my-change`)
3. Make your changes and add tests
4. Run the full build and test suite: `npm run build:full && npm test && cargo test`
5. Submit a pull request

## 📄 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## 🆘 Support

- **Documentation**: [https://docs.stellar-cross-border.com](https://docs.stellar-cross-border.com)
- **Issues**: [GitHub Issues](https://github.com/stellar-cross-border/stellar-cross-border-payments-sdk/issues)
- **Discord**: [Stellar Discord](https://discord.gg/stellar)
- **Twitter**: [@StellarOrg](https://twitter.com/StellarOrg)

## 🙏 Acknowledgments

- [Stellar Development Foundation](https://stellar.org/) for the amazing platform
- [Soroban](https://soroban.stellar.org/) for smart contract support
- The Stellar community for feedback and contributions

## 📊 Roadmap

### v0.2.0 (Q2 2024)
- [ ] Multi-signature support
- [ ] Advanced dispute resolution
- [ ] Mobile SDK
- [ ] More fiat currency pairs

### v0.3.0 (Q3 2024)
- [ ] DeFi integration
- [ ] Advanced analytics
- [ ] Compliance automation
- [ ] Enterprise features

### v1.0.0 (Q4 2024)
- [ ] Production audit
- [ ] SLA guarantees
- [ ] 24/7 support
- [ ] Global compliance framework

---

**Built with ❤️ for the Stellar ecosystem**
