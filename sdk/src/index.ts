export { StellarClient } from './client';
export { StellarPayments } from './payments';
export * from './types';

import { StellarClient } from './client';
import { StellarPayments } from './payments';
import { StellarConfig, ContractAddresses } from './types';

export class StellarCrossBorderSDK {
  private client: StellarClient;
  private payments: StellarPayments;

  constructor(config: StellarConfig, contracts: ContractAddresses) {
    this.client = new StellarClient(config, contracts);
    this.payments = new StellarPayments(this.client);
  }

  get clientInstance(): StellarClient {
    return this.client;
  }

  get paymentsInstance(): StellarPayments {
    return this.payments;
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
