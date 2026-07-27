import {
  Account,
  Asset,
  Contract,
  Keypair,
  Memo,
  Networks,
  Operation,
  TransactionBuilder,
  BASE_FEE,
  TimeoutInfinite,
  xdr,
  StrKey,
  Address,
  Horizon,
} from 'stellar-sdk';
import { SorobanRpc } from 'stellar-sdk';
import { AxiosInstance, default as axios } from 'axios';
import BigNumber from 'bignumber.js';
import {
  StellarConfig,
  ContractAddresses,
  TransactionResult,
  NetworkInfo,
  AccountInfo,
  FeeEstimate,
  ErrorInfo,
} from './types';

export class StellarClient {
  private config: StellarConfig;
  private contracts: ContractAddresses;
  private httpClient: AxiosInstance;
  private sorobanRpc: SorobanRpc.Server;

  constructor(config: StellarConfig, contracts: ContractAddresses) {
    this.config = config;
    this.contracts = contracts;
    this.httpClient = axios.create({
      baseURL: config.horizonUrl,
      timeout: 30000,
      headers: { 'Content-Type': 'application/json' },
    });
    this.sorobanRpc = new SorobanRpc.Server(config.sorobanRpcUrl);
  }

  async getNetworkInfo(): Promise<NetworkInfo> {
    try {
      const friendbotUrl =
        this.config.networkPassphrase === Networks.TESTNET
          ? `${this.config.horizonUrl}/friendbot`
          : undefined;

      return {
        horizonUrl: this.config.horizonUrl,
        sorobanRpcUrl: this.config.sorobanRpcUrl,
        networkPassphrase: this.config.networkPassphrase,
        friendbotUrl,
      };
    } catch (error) {
      throw this.handleError(error);
    }
  }

  async getAccount(accountId: string): Promise<AccountInfo> {
    try {
      const response = await this.httpClient.get(`/accounts/${accountId}`);
      const account = response.data;

      // Defensively handle ledgers where balances or flags may be absent
      const balances: any[] = Array.isArray(account.balances) ? account.balances : [];
      const nativeEntry = balances.find((b: any) => b.asset_type === 'native');
      const flags = account.flags ?? {};

      return {
        accountId: account.id,
        // Fall back to '0' when no native (XLM) balance entry is present
        balance: nativeEntry?.balance ?? '0',
        sequence: account.sequence,
        numSubentries: account.num_subentries ?? 0,
        flags: {
          authRequired: flags.auth_required ?? false,
          authRevocable: flags.auth_revocable ?? false,
          authImmutable: flags.auth_immutable ?? false,
        },
      };
    } catch (error) {
      throw this.handleError(error);
    }
  }

  async fundTestnetAccount(accountId: string): Promise<TransactionResult> {
    try {
      if (this.config.networkPassphrase !== Networks.TESTNET) {
        throw new Error('Testnet funding is only available on testnet');
      }
      const response = await this.httpClient.post('/friendbot', { addr: accountId });
      return { hash: response.data.hash, success: true, result: response.data };
    } catch (error) {
      throw this.handleError(error);
    }
  }

  async estimateFee(operations: number = 1): Promise<FeeEstimate> {
    try {
      const baseFee = new BigNumber(BASE_FEE);
      const recommendedFee = baseFee.multipliedBy(operations);
      const maxFee = recommendedFee.multipliedBy(2);
      const feeBumpFee = recommendedFee.multipliedBy(1.5);
      return {
        minFee: baseFee.toString(),
        recommendedFee: recommendedFee.toString(),
        maxFee: maxFee.toString(),
        feeBumpFee: feeBumpFee.toString(),
      };
    } catch (error) {
      throw this.handleError(error);
    }
  }

  async buildTransaction(
    sourceAccount: Account,
    operations: Operation[],
    options: {
      fee?: string;
      memo?: string;
      timeout?: number;
      feeBump?: boolean;
    } = {}
  ): Promise<TransactionBuilder> {
    const fee = options.fee ?? (options.feeBump ? '2000' : BASE_FEE);
    const timeoutSeconds = options.timeout ?? TimeoutInfinite;

    // Convert a relative timeout (seconds) into an absolute Unix maxTime.
    // TimeoutInfinite (0) means no expiry — pass 0 to disable the upper bound.
    // Any positive value is treated as seconds-from-now, which is the correct
    // semantics for transaction timebounds: minTime=0 (valid immediately),
    // maxTime=<absolute unix timestamp> (expires after N seconds).
    const maxTime =
      timeoutSeconds === TimeoutInfinite || timeoutSeconds === 0
        ? TimeoutInfinite
        : Math.floor(Date.now() / 1000) + timeoutSeconds;

    let builder = new TransactionBuilder(sourceAccount, {
      fee,
      networkPassphrase: this.config.networkPassphrase,
      timebounds: { minTime: 0, maxTime },
    });

    if (options.memo) {
      builder = builder.addMemo(Memo.text(options.memo));
    }

    // stellar-sdk v12 Contract.call() returns Operation2 — cast through any
    operations.forEach((op) => builder.addOperation(op as any));

    return builder;
  }

  async submitTransaction(
    transactionXdr: string,
    options: { skipLedgerCheck?: boolean } = {}
  ): Promise<TransactionResult> {
    try {
      const response = await this.httpClient.post('/transactions', {
        tx: transactionXdr,
        skip_ledger_check: options.skipLedgerCheck ?? false,
      });
      const result = response.data;
      return { hash: result.hash, success: result.successful, result };
    } catch (error: any) {
      const errorInfo: ErrorInfo = {
        code: error.response?.data?.code ?? 'UNKNOWN_ERROR',
        message: error.response?.data?.title ?? error.message,
        details: error.response?.data,
        transactionResult: error.response?.data?.extras?.result_codes,
      };
      return { hash: '', success: false, error: errorInfo.message };
    }
  }

  async simulateTransaction(transactionXdr: string): Promise<any> {
    try {
      return await this.sorobanRpc.simulateTransaction(transactionXdr as any);
    } catch (error) {
      throw this.handleError(error);
    }
  }

  async prepareSorobanTransaction(
    transaction: TransactionBuilder,
    signer: Keypair
  ): Promise<TransactionBuilder> {
    const tx = transaction.build();
    const prepared = await this.sorobanRpc.prepareTransaction(tx);

    if (!prepared) {
      throw new Error('Soroban prepare transaction returned no result');
    }

    const signedTx = prepared.transaction as unknown as TransactionBuilder;
    signedTx.sign(signer);
    return signedTx;
  }

  async sendSorobanTransaction(transaction: any): Promise<TransactionResult> {
    try {
      const tx = transaction.transaction || (typeof transaction.build === 'function' ? transaction.build() : transaction);
      const response = await this.sorobanRpc.sendTransaction(tx);
      return {
        hash: response.hash,
        success: response.status === 'PENDING' || response.status === 'SUCCESS',
        result: response,
      };
    } catch (error) {
      throw this.handleError(error);
    }
  }

  async getSorobanTransactionStatus(hash: string): Promise<TransactionResult> {
    try {
      const response = await this.sorobanRpc.getTransaction(hash);
      return {
        hash,
        success: response.status === 'SUCCESS',
        result: response,
      };
    } catch (error) {
      throw this.handleError(error);
    }
  }

  async getContractData(
    contractId: string,
    key: xdr.ScVal,
    durability: 'temporary' | 'persistent' = 'persistent'
  ): Promise<xdr.ScVal | null> {
    try {
      const ledgerKey = xdr.LedgerKey.contractData(
        new xdr.LedgerKeyContractData({
          contract: Address.fromString(contractId).toScAddress(),
          key,
          durability:
            durability === 'temporary'
              ? xdr.ContractDataDurability.temporary()
              : xdr.ContractDataDurability.persistent(),
        })
      );

      const result = await this.sorobanRpc.getLedgerEntries(ledgerKey);

      if (result.entries.length === 0) return null;

      // val is the raw ledger entry data; extract the contract data value
      const entry = result.entries[0] as any;
      const scVal: xdr.ScVal = entry.val?.contractData?.()?.val?.() ?? entry.val;
      return scVal;
    } catch (error) {
      throw this.handleError(error);
    }
  }

  /** Returns the raw Operation2 — callers should cast via `as unknown as Operation` */
  invokeContractMethod(
    contractId: string,
    method: string,
    args: xdr.ScVal[] = []
  ): Operation {
    const contract = new Contract(contractId);
    return contract.call(method, ...args) as unknown as Operation;
  }

  getEscrowContract(): Contract {
    return new Contract(this.contracts.escrow);
  }

  getRateOracleContract(): Contract {
    return new Contract(this.contracts.rateOracle);
  }

  getComplianceContract(): Contract {
    return new Contract(this.contracts.compliance);
  }

  getHorizon(): Horizon.Server {
    return new Horizon.Server(this.config.horizonUrl);
  }

  createKeyPair(): Keypair {
    return Keypair.random();
  }

  validateAddress(address: string): boolean {
    try {
      StrKey.decodeEd25519PublicKey(address);
      return true;
    } catch {
      try {
        // eslint-disable-next-line no-new
        new Address(address);
        return true;
      } catch {
        return false;
      }
    }
  }

  formatAmount(amount: string | number, decimals: number = 7): string {
    return new BigNumber(amount)
      .dividedBy(new BigNumber(10).pow(decimals))
      .toString();
  }

  parseAmount(amount: string, decimals: number = 7): string {
    return new BigNumber(amount)
      .multipliedBy(new BigNumber(10).pow(decimals))
      .toFixed(0);
  }

  async waitForTransaction(
    hash: string,
    timeout: number = 30000
  ): Promise<TransactionResult> {
    const startTime = Date.now();

    while (Date.now() - startTime < timeout) {
      try {
        const response = await this.httpClient.get(`/transactions/${hash}`);
        const transaction = response.data;
        if (transaction.successful !== undefined) {
          return { hash, success: transaction.successful, result: transaction };
        }
      } catch (error: any) {
        if (error.response?.status !== 404) {
          throw this.handleError(error);
        }
      }
      await new Promise((resolve) => setTimeout(resolve, 1000));
    }

    throw new Error(`Transaction ${hash} not confirmed within ${timeout}ms`);
  }

  private handleError(error: any): Error {
    if (error.response) {
      const { status, data } = error.response;
      const message = data.title ?? data.message ?? error.message;
      return new Error(`Stellar API Error (${status}): ${message}`);
    }
    if (error.request) {
      return new Error('Network error: Unable to connect to Stellar API');
    }
    return new Error(`Unexpected error: ${error.message}`);
  }

  getConfig(): StellarConfig {
    return { ...this.config };
  }

  getContracts(): ContractAddresses {
    return { ...this.contracts };
  }

  updateConfig(newConfig: Partial<StellarConfig>): void {
    this.config = { ...this.config, ...newConfig };
    if (newConfig.horizonUrl) {
      this.httpClient.defaults.baseURL = newConfig.horizonUrl;
    }
    if (newConfig.sorobanRpcUrl) {
      this.sorobanRpc = new SorobanRpc.Server(newConfig.sorobanRpcUrl);
    }
  }
}
