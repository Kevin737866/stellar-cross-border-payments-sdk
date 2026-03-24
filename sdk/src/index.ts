export { StellarClient } from './client';
export { StellarPayments } from './payments';
export { PathPaymentService } from './pathPayment';
export { RateOptimizer } from './rateOptimizer';
export { TrustlineService } from './trustlines';
export * from './types';

import { StellarClient } from './client';
import { StellarPayments } from './payments';
import { PathPaymentService } from './pathPayment';
import { RateOptimizer } from './rateOptimizer';
import { TrustlineService } from './trustlines';
import { StellarConfig, ContractAddresses } from './types';

export class StellarCrossBorderSDK {
  private client: StellarClient;
  private payments: StellarPayments;
  private pathPayment: PathPaymentService;
  private rateOptimizer: RateOptimizer;
  private trustlines: TrustlineService;

  constructor(config: StellarConfig, contracts: ContractAddresses) {
    this.client = new StellarClient(config, contracts);
    this.payments = new StellarPayments(this.client);
    this.pathPayment = new PathPaymentService(this.client);
    this.rateOptimizer = new RateOptimizer(this.client);
    this.trustlines = new TrustlineService(this.client);
  }

  get clientInstance(): StellarClient {
    return this.client;
  }

  get paymentsInstance(): StellarPayments {
    return this.payments;
  }

  get pathPaymentInstance(): PathPaymentService {
    return this.pathPayment;
  }

  get rateOptimizerInstance(): RateOptimizer {
    return this.rateOptimizer;
  }

  get trustlinesInstance(): TrustlineService {
    return this.trustlines;
  }

  static createTestnetConfig(horizonUrl?: string, sorobanRpcUrl?: string): StellarConfig {
    return {
      horizonUrl: horizonUrl || 'https://horizon-testnet.stellar.org',
      sorobanRpcUrl: sorobanRpcUrl || 'https://soroban-testnet.stellar.org',
      networkPassphrase: 'Test SDF Network ; September 2015',
      defaultTimeout: 30000,
    };
  }

  static createMainnetConfig(horizonUrl?: string, sorobanRpcUrl?: string): StellarConfig {
    return {
      horizonUrl: horizonUrl || 'https://horizon.stellar.org',
      sorobanRpcUrl: sorobanRpcUrl || 'https://soroban.stellar.org',
      networkPassphrase: 'Public Global Stellar Network ; September 2015',
      defaultTimeout: 30000,
    };
  }

  static createFuturenetConfig(horizonUrl?: string, sorobanRpcUrl?: string): StellarConfig {
    return {
      horizonUrl: horizonUrl || 'https://horizon-futurenet.stellar.org',
      sorobanRpcUrl: sorobanRpcUrl || 'https://soroban-futurenet.stellar.org',
      networkPassphrase: 'Test SDF Future Network ; October 2022',
      defaultTimeout: 30000,
    };
  }
}

export default StellarCrossBorderSDK;
