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
- [Testing](#testing)
- [Deployment](#deployment)
- [Contributing](#contributing)

## 🛠 Installation

### Prerequisites
- Node.js 18+ 
- Rust 1.70+ (for contract compilation)
- Soroban CLI (for contract deployment)

### Install SDK

```bash
# Clone the repository
git clone https://github.com/stellar-cross-border/stellar-cross-border-payments-sdk.git
cd stellar-cross-border-payments-sdk

# Install dependencies
npm install

# Install Rust dependencies
cd src && cargo build
cd ..
```

### Environment Setup

```bash
# Copy environment template
cp .env.example .env

# Edit with your configuration
nano .env
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

React hook for payment state management:

```typescript
import { useStellarPayment } from '@stellar-cross-border/ui';

const MyComponent = () => {
  const {
    loading,
    error,
    paymentStatus,
    createPayment,
    releaseEscrow,
    refreshStatus,
  } = useStellarPayment(sdk, escrowId, { autoRefresh: true });

  return (
    <div>
      {loading && <div>Loading...</div>}
      {error && <div>Error: {error}</div>}
      {paymentStatus && (
        <div>Status: {paymentStatus.status}</div>
      )}
    </div>
  );
};
```

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

### Contract Deployment

```bash
# Build contracts
cd src
cargo build --target wasm32-unknown-unknown --release

# Deploy escrow contract
soroban contract deploy \
  --wasm target/wasm32-unknown-unknown/release/stellar_cross_border_payments_sdk.wasm \
  --source ADMIN_SECRET_KEY \
  --network testnet

# Deploy other contracts similarly
```

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

### Run Contract Tests

```bash
cd src
cargo test
```

### Run SDK Tests

```bash
cd sdk
npm test
```

### Run UI Tests

```bash
cd ui
npm test
```

### Integration Tests

```bash
# Run all examples as tests
npm run test:examples
```

## 🚀 Deployment

### Deploy to Testnet

```bash
# 1. Deploy contracts
npm run deploy:testnet

# 2. Update environment variables
cp .env.example .env.testnet
# Edit with testnet contract addresses

# 3. Deploy UI
cd ui
npm run build
npm run start
```

### Deploy to Mainnet

```bash
# 1. Deploy contracts to mainnet
npm run deploy:mainnet

# 2. Update production environment
cp .env.example .env.production
# Edit with mainnet contract addresses

# 3. Deploy UI to production
cd ui
npm run build
npm run start:prod
```

## 🔧 Development

### Build Contracts

```bash
cd src
cargo build --target wasm32-unknown-unknown --release
```

### Build SDK

```bash
cd sdk
npm run build
```

### Build UI

```bash
cd ui
npm run build
```

### Local Development

```bash
# Start local Soroban RPC
soroban rpc start

# Run contracts in local mode
npm run dev:local

# Start UI development server
cd ui
npm run dev
```

## 🤝 Contributing

We welcome contributions! Please see our [Contributing Guide](CONTRIBUTING.md) for details.

### Development Workflow

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Add tests
5. Submit a pull request

### Code Style

- Rust: Use `rustfmt` and `clippy`
- TypeScript: Use ESLint and Prettier
- React: Follow React best practices

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
