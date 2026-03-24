import { Asset } from 'stellar-sdk';
import BigNumber from 'bignumber.js';
import { StellarClient } from './client';
import { PathPaymentService, BestPathResult } from './pathPayment';
import { StellarPayments } from './payments';
import { ExchangeRateRequest } from './types';

export interface OptimizedRate {
  venue: 'DEX' | 'Oracle' | 'External';
  rate: string;
  amount: string;
  path?: string[];
  confidence: number;
}

export class RateOptimizer {
  private client: StellarClient;
  private pathService: PathPaymentService;
  private payments: StellarPayments;

  constructor(client: StellarClient) {
    this.client = client;
    this.pathService = new PathPaymentService(client);
    this.payments = new StellarPayments(client);
  }

  /**
   * findCheapestExecution - Parallel-fetches and auto-selects cheapest execution venue
   */
  async findCheapestExecution(
    fromAsset: string,
    toAsset: string,
    amount: string
  ): Promise<OptimizedRate> {
    const fromSymbol = this.getAssetSymbol(fromAsset);
    const toSymbol = this.getAssetSymbol(toAsset);

    const [dexQuote, oracleQuote, externalQuote] = await Promise.all([
      this.getDexQuote(fromAsset, toAsset, amount),
      this.getOracleQuote(fromSymbol, toSymbol, amount),
      this.getExternalQuote(fromSymbol, toSymbol, amount),
    ]);

    const quotes: OptimizedRate[] = [];
    if (dexQuote) quotes.push(dexQuote);
    if (oracleQuote) quotes.push(oracleQuote);
    if (externalQuote) quotes.push(externalQuote);

    if (quotes.length === 0) {
      throw new Error(`No execution path found for ${fromAsset} -> ${toAsset}`);
    }

    // Sort by best rate (highest amount for destination)
    return quotes.sort((a, b) => 
      new BigNumber(b.amount).comparedTo(a.amount)
    )[0];
  }

  private async getDexQuote(from: string, to: string, amount: string): Promise<OptimizedRate | null> {
    try {
      const fromA = this.parseAsset(from);
      const toA = this.parseAsset(to);
      const result: BestPathResult = await this.pathService.findBestPath(fromA, toA, amount);
      
      return {
        venue: 'DEX',
        rate: result.rate.toString(),
        amount: result.destinationAmount,
        path: result.path,
        confidence: 95,
      };
    } catch {
      return null;
    }
  }

  private async getOracleQuote(from: string, to: number | string, amount: string): Promise<OptimizedRate | null> {
    try {
      const request: ExchangeRateRequest = {
        from_currency: from,
        to_currency: to.toString(),
      };
      const result = await this.payments.getExchangeRate(request);
      
      const rateBN = new BigNumber(result.rate).dividedBy(1_000_000);
      const destAmount = new BigNumber(amount).times(rateBN).toFixed(7);

      return {
        venue: 'Oracle',
        rate: rateBN.toString(),
        amount: destAmount,
        confidence: result.aggregated.sources_count > 0 ? 90 : 50,
      };
    } catch {
      return null;
    }
  }

  private async getExternalQuote(from: string, to: string, amount: string): Promise<OptimizedRate | null> {
    // Placeholder for External FX APIs (XE, OANDA)
    // In a real implementation, this would call axios.get(...)
    return {
      venue: 'External',
      rate: '0.92', // Mock EUR rate
      amount: new BigNumber(amount).times(0.92).toFixed(7),
      confidence: 100,
    };
  }

  private parseAsset(assetStr: string): Asset {
    if (assetStr === 'XLM' || assetStr === 'native') return Asset.native();
    const [code, issuer] = assetStr.split(':');
    return new Asset(code, issuer);
  }

  private getAssetSymbol(assetStr: string): string {
    if (assetStr === 'XLM' || assetStr === 'native') return 'XLM';
    return assetStr.split(':')[0];
  }
}
