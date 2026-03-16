import {
  Account,
  Keypair,
  Operation,
  TransactionBuilder,
  xdr,
  Address,
  ScInt,
} from 'stellar-sdk';
import BigNumber from 'bignumber.js';
import { StellarClient } from './client';
import {
  PaymentRequest,
  PaymentOptions,
  EscrowCreationResult,
  PaymentStatus,
  ComplianceCheckResult,
  ExchangeRateResult,
  TransactionResult,
  EscrowStatus,
  ComplianceRequest,
  ExchangeRateRequest,
  Escrow,
  Dispute,
  ComplianceCheck,
  AggregatedRate,
} from './types';

export class StellarPayments {
  private client: StellarClient;

  constructor(client: StellarClient) {
    this.client = client;
  }

  async createPayment(
    request: PaymentRequest,
    options: PaymentOptions = {}
  ): Promise<EscrowCreationResult> {
    try {
      const sourceAccount = await this.getSourceAccount(request.from);
      
      const escrowContract = this.client.getEscrowContract();
      
      const releaseTime = request.release_time || 
        Math.floor(Date.now() / 1000) + (24 * 60 * 60); // 24 hours default

      const metadata = request.metadata || {};
      const metadataScVal = this.convertMetadataToScVal(metadata);

      const createEscrowOp = escrowContract.call(
        'create_escrow',
        new Address(request.from).toScVal(),
        new Address(request.to).toScVal(),
        new ScInt(request.amount).toI128(),
        new Address(request.token).toScVal(),
        new ScInt(releaseTime).toU64(),
        metadataScVal
      );

      const builder = await this.client.buildTransaction(
        sourceAccount,
        [createEscrowOp],
        {
          fee: options.feeBump ? '2000' : undefined,
          memo: options.memo,
          timeout: options.timeout,
        }
      );

      const transaction = builder.build();
      
      if (options.submit !== false) {
        const result = await this.client.submitTransaction(transaction.toXDR());
        
        if (result.success && result.result) {
          const escrowId = this.extractEscrowIdFromResult(result.result);
          return {
            ...result,
            escrowId,
          };
        }
        
        return {
          hash: result.hash,
          success: false,
          error: result.error,
          escrowId: '',
        };
      }

      return {
        hash: transaction.hash().toString('hex'),
        success: true,
        escrowId: '',
      };
    } catch (error) {
      return {
        hash: '',
        success: false,
        error: error instanceof Error ? error.message : 'Unknown error',
        escrowId: '',
      };
    }
  }

  async releaseEscrow(
    escrowId: string,
    signer: Keypair,
    options: PaymentOptions = {}
  ): Promise<TransactionResult> {
    try {
      const sourceAccount = await this.getSourceAccount(signer.publicKey());
      
      const escrowContract = this.client.getEscrowContract();
      const releaseEscrowOp = escrowContract.call(
        'release_escrow',
        xdr.ScVal.scvBytes(Buffer.from(escrowId, 'hex'))
      );

      const builder = await this.client.buildTransaction(
        sourceAccount,
        [releaseEscrowOp],
        {
          fee: options.feeBump ? '2000' : undefined,
          memo: options.memo,
          timeout: options.timeout,
        }
      );

      const transaction = builder.build();
      transaction.sign(signer);

      if (options.submit !== false) {
        return await this.client.submitTransaction(transaction.toXDR());
      }

      return {
        hash: transaction.hash().toString('hex'),
        success: true,
      };
    } catch (error) {
      return {
        hash: '',
        success: false,
        error: error instanceof Error ? error.message : 'Unknown error',
      };
    }
  }

  async refundEscrow(
    escrowId: string,
    signer: Keypair,
    options: PaymentOptions = {}
  ): Promise<TransactionResult> {
    try {
      const sourceAccount = await this.getSourceAccount(signer.publicKey());
      
      const escrowContract = this.client.getEscrowContract();
      const refundEscrowOp = escrowContract.call(
        'refund_escrow',
        xdr.ScVal.scvBytes(Buffer.from(escrowId, 'hex'))
      );

      const builder = await this.client.buildTransaction(
        sourceAccount,
        [refundEscrowOp],
        {
          fee: options.feeBump ? '2000' : undefined,
          memo: options.memo,
          timeout: options.timeout,
        }
      );

      const transaction = builder.build();
      transaction.sign(signer);

      if (options.submit !== false) {
        return await this.client.submitTransaction(transaction.toXDR());
      }

      return {
        hash: transaction.hash().toString('hex'),
        success: true,
      };
    } catch (error) {
      return {
        hash: '',
        success: false,
        error: error instanceof Error ? error.message : 'Unknown error',
      };
    }
  }

  async disputeEscrow(
    escrowId: string,
    challenger: string,
    reason: string,
    evidence: Uint8Array,
    signer: Keypair,
    options: PaymentOptions = {}
  ): Promise<TransactionResult> {
    try {
      const sourceAccount = await this.getSourceAccount(signer.publicKey());
      
      const escrowContract = this.client.getEscrowContract();
      const disputeEscrowOp = escrowContract.call(
        'dispute_escrow',
        xdr.ScVal.scvBytes(Buffer.from(escrowId, 'hex')),
        new Address(challenger).toScVal(),
        xdr.ScVal.scvSymbol(reason),
        xdr.ScVal.scvBytes(evidence)
      );

      const builder = await this.client.buildTransaction(
        sourceAccount,
        [disputeEscrowOp],
        {
          fee: options.feeBump ? '2000' : undefined,
          memo: options.memo,
          timeout: options.timeout,
        }
      );

      const transaction = builder.build();
      transaction.sign(signer);

      if (options.submit !== false) {
        return await this.client.submitTransaction(transaction.toXDR());
      }

      return {
        hash: transaction.hash().toString('hex'),
        success: true,
      };
    } catch (error) {
      return {
        hash: '',
        success: false,
        error: error instanceof Error ? error.message : 'Unknown error',
      };
    }
  }

  async getExchangeRate(request: ExchangeRateRequest): Promise<ExchangeRateResult> {
    try {
      const rateOracleContract = this.client.getRateOracleContract();
      
      const rateKey = xdr.ScVal.scvVec([
        xdr.ScVal.scvSymbol(request.from_currency),
        xdr.ScVal.scvSymbol(request.to_currency),
      ]);

      const rateData = await this.client.getContractData(
        this.client.getContracts().rateOracle,
        rateKey
      );

      if (!rateData) {
        throw new Error(`Exchange rate not found for ${request.from_currency}/${request.to_currency}`);
      }

      const aggregatedRate = this.parseAggregatedRate(rateData);
      
      const sources = await this.getRateSources(request.from_currency, request.to_currency);

      return {
        rate: aggregatedRate.rate,
        timestamp: aggregatedRate.last_updated,
        sources,
        aggregated: aggregatedRate,
      };
    } catch (error) {
      throw new Error(`Failed to get exchange rate: ${error instanceof Error ? error.message : 'Unknown error'}`);
    }
  }

  async checkCompliance(request: ComplianceRequest): Promise<ComplianceCheckResult> {
    try {
      const complianceContract = this.client.getComplianceContract();
      
      const transactionId = this.generateTransactionId();
      
      const complianceOp = complianceContract.call(
        'check_transaction_compliance',
        xdr.ScVal.scvBytes(Buffer.from(transactionId, 'hex')),
        new Address(request.from_user).toScVal(),
        new Address(request.to_user).toScVal(),
        new ScInt(request.amount).toI128(),
        xdr.ScVal.scvSymbol(request.currency),
        xdr.ScVal.scvSymbol(request.jurisdiction_from),
        xdr.ScVal.scvSymbol(request.jurisdiction_to)
      );

      const result = await this.client.simulateTransaction(complianceOp);
      
      if (!result.results || result.results.length === 0) {
        throw new Error('Compliance check returned no results');
      }

      const complianceCheck = this.parseComplianceCheck(result.results[0]);

      return {
        hash: transactionId,
        success: true,
        approved: complianceCheck.approved,
        reason: complianceCheck.reason,
        rulesTriggered: complianceCheck.rules_triggered,
      };
    } catch (error) {
      return {
        hash: '',
        success: false,
        approved: false,
        reason: error instanceof Error ? error.message : 'Compliance check failed',
        rulesTriggered: [],
      };
    }
  }

  async getPaymentStatus(escrowId: string): Promise<PaymentStatus> {
    try {
      const escrow = await this.getEscrow(escrowId);
      const currentTime = Math.floor(Date.now() / 1000);
      
      return {
        escrowId: escrow.id,
        status: escrow.status,
        amount: escrow.amount,
        sender: escrow.sender,
        receiver: escrow.receiver,
        created_at: escrow.created_at,
        release_time: escrow.release_time,
        can_release: escrow.status === EscrowStatus.Pending && currentTime >= escrow.release_time,
        can_refund: escrow.status === EscrowStatus.Pending,
      };
    } catch (error) {
      throw new Error(`Failed to get payment status: ${error instanceof Error ? error.message : 'Unknown error'}`);
    }
  }

  async getEscrow(escrowId: string): Promise<Escrow> {
    try {
      const escrowContract = this.client.getEscrowContract();
      const escrowKey = xdr.ScVal.scvBytes(Buffer.from(escrowId, 'hex'));
      
      const escrowData = await this.client.getContractData(
        this.client.getContracts().escrow,
        escrowKey
      );

      if (!escrowData) {
        throw new Error(`Escrow not found: ${escrowId}`);
      }

      return this.parseEscrow(escrowData);
    } catch (error) {
      throw new Error(`Failed to get escrow: ${error instanceof Error ? error.message : 'Unknown error'}`);
    }
  }

  async getUserEscrows(userAddress: string): Promise<string[]> {
    try {
      const escrowContract = this.client.getEscrowContract();
      const userKey = xdr.ScVal.scvSymbol(`USER_ESCROWS_${userAddress}`);
      
      const escrowData = await this.client.getContractData(
        this.client.getContracts().escrow,
        userKey
      );

      if (!escrowData) {
        return [];
      }

      return this.parseEscrowList(escrowData);
    } catch (error) {
      throw new Error(`Failed to get user escrows: ${error instanceof Error ? error.message : 'Unknown error'}`);
    }
  }

  private async getSourceAccount(accountId: string): Promise<Account> {
    const accountInfo = await this.client.getAccount(accountId);
    return new Account(accountId, accountInfo.sequence);
  }

  private convertMetadataToScVal(metadata: Record<string, Uint8Array>): xdr.ScVal {
    const entries = Object.entries(metadata).map(([key, value]) => 
      xdr.ScVal.scvMap([
        new xdr.ScMapEntry({
          key: xdr.ScVal.scvSymbol(key),
          val: xdr.ScVal.scvBytes(value),
        }),
      ])
    );

    return xdr.ScVal.scvMap(entries.flat());
  }

  private extractEscrowIdFromResult(result: any): string {
    if (result.result_xdr) {
      const transactionResult = xdr.TransactionResult.fromXDR(result.result_xdr, 'base64');
      const results = transactionResult.result().results();
      if (results && results.length > 0) {
        const scVal = results[0].value();
        if (scVal.switch() === xdr.ScValType.scvBytes()) {
          return Buffer.from(scVal.bytes()).toString('hex');
        }
      }
    }
    return '';
  }

  private parseAggregatedRate(scVal: xdr.ScVal): AggregatedRate {
    if (scVal.switch() !== xdr.ScValType.scvMap()) {
      throw new Error('Invalid aggregated rate format');
    }

    const map = scVal.map();
    const result: any = {};

    for (const entry of map) {
      const key = entry.key().sym().toString();
      const value = entry.val();
      
      switch (key) {
        case 'rate':
          result.rate = new BigNumber(value.u128().toString()).toString();
          break;
        case 'weighted_average':
          result.weighted_average = new BigNumber(value.u128().toString()).toString();
          break;
        case 'sources_count':
          result.sources_count = value.u32();
          break;
        case 'last_updated':
          result.last_updated = value.u64();
          break;
        case 'deviation_threshold':
          result.deviation_threshold = value.u32();
          break;
      }
    }

    return result as AggregatedRate;
  }

  private parseComplianceCheck(scVal: xdr.ScVal): ComplianceCheck {
    if (scVal.switch() !== xdr.ScValType.scvMap()) {
      throw new Error('Invalid compliance check format');
    }

    const map = scVal.map();
    const result: any = {};

    for (const entry of map) {
      const key = entry.key().sym().toString();
      const value = entry.val();
      
      switch (key) {
        case 'transaction_id':
          result.transaction_id = Buffer.from(value.bytes()).toString('hex');
          break;
        case 'from_user':
          result.from_user = Address.fromScVal(value).toString();
          break;
        case 'to_user':
          result.to_user = Address.fromScVal(value).toString();
          break;
        case 'amount':
          result.amount = new BigNumber(value.i128().toString()).toString();
          break;
        case 'currency':
          result.currency = value.sym().toString();
          break;
        case 'jurisdiction_from':
          result.jurisdiction_from = value.sym().toString();
          break;
        case 'jurisdiction_to':
          result.jurisdiction_to = value.sym().toString();
          break;
        case 'timestamp':
          result.timestamp = value.u64();
          break;
        case 'approved':
          result.approved = value.b();
          break;
        case 'reason':
          result.reason = value.sym().toString();
          break;
        case 'rules_triggered':
          result.rules_triggered = this.parseStringArray(value);
          break;
      }
    }

    return result as ComplianceCheck;
  }

  private parseEscrow(scVal: xdr.ScVal): Escrow {
    if (scVal.switch() !== xdr.ScValType.scvMap()) {
      throw new Error('Invalid escrow format');
    }

    const map = scVal.map();
    const result: any = {};

    for (const entry of map) {
      const key = entry.key().sym().toString();
      const value = entry.val();
      
      switch (key) {
        case 'id':
          result.id = Buffer.from(value.bytes()).toString('hex');
          break;
        case 'sender':
          result.sender = Address.fromScVal(value).toString();
          break;
        case 'receiver':
          result.receiver = Address.fromScVal(value).toString();
          break;
        case 'amount':
          result.amount = new BigNumber(value.i128().toString()).toString();
          break;
        case 'token':
          result.token = Address.fromScVal(value).toString();
          break;
        case 'status':
          result.status = this.parseEscrowStatus(value);
          break;
        case 'release_time':
          result.release_time = value.u64();
          break;
        case 'created_at':
          result.created_at = value.u64();
          break;
        case 'metadata':
          result.metadata = this.parseMetadata(value);
          break;
      }
    }

    return result as Escrow;
  }

  private parseEscrowStatus(scVal: xdr.ScVal): EscrowStatus {
    const status = scVal.sym().toString();
    switch (status) {
      case 'Pending': return EscrowStatus.Pending;
      case 'Completed': return EscrowStatus.Completed;
      case 'Refunded': return EscrowStatus.Refunded;
      case 'Disputed': return EscrowStatus.Disputed;
      default: throw new Error(`Unknown escrow status: ${status}`);
    }
  }

  private parseMetadata(scVal: xdr.ScVal): Record<string, Uint8Array> {
    if (scVal.switch() !== xdr.ScValType.scvMap()) {
      return {};
    }

    const map = scVal.map();
    const result: Record<string, Uint8Array> = {};

    for (const entry of map) {
      const key = entry.key().sym().toString();
      const value = entry.val();
      if (value.switch() === xdr.ScValType.scvBytes()) {
        result[key] = value.bytes();
      }
    }

    return result;
  }

  private parseStringArray(scVal: xdr.ScVal): string[] {
    if (scVal.switch() !== xdr.ScValType.scvVec()) {
      return [];
    }

    const vec = scVal.vec();
    const result: string[] = [];

    for (const item of vec) {
      if (item.switch() === xdr.ScValType.scvBytes()) {
        result.push(Buffer.from(item.bytes()).toString('hex'));
      }
    }

    return result;
  }

  private parseEscrowList(scVal: xdr.ScVal): string[] {
    if (scVal.switch() !== xdr.ScValType.scvVec()) {
      return [];
    }

    const vec = scVal.vec();
    const result: string[] = [];

    for (const item of vec) {
      if (item.switch() === xdr.ScValType.scvBytes()) {
        result.push(Buffer.from(item.bytes()).toString('hex'));
      }
    }

    return result;
  }

  private async getRateSources(fromCurrency: string, toCurrency: string): Promise<any[]> {
    // This would typically query the rate oracle for individual source rates
    // For now, return an empty array as this is a placeholder
    return [];
  }

  private generateTransactionId(): string {
    return Buffer.from(Math.random().toString(36).substring(2, 15)).toString('hex');
  }
}
