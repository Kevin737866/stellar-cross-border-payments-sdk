import {
  Asset,
  Operation,
  TransactionBuilder,
  Account,
} from 'stellar-sdk';
import { StellarClient } from './client';

export class TrustlineService {
  private client: StellarClient;

  constructor(client: StellarClient) {
    this.client = client;
  }

  /**
   * ensureTrustline - Checks if trustline exists, if not creates the CHANGE_TRUST operation
   */
  async ensureTrustline(
    accountId: string,
    asset: Asset,
    limit?: string
  ): Promise<Operation | null> {
    const accountInfo = await this.client.getAccount(accountId);
    
    // Check if trustline already exists
    const hasTrustline = (this.client as any).httpClient.get(`/accounts/${accountId}`)
      .then((res: any) => {
        const balances = res.data.balances || [];
        return balances.some((b: any) => 
          (asset.isNative() && b.asset_type === 'native') ||
          (b.asset_code === asset.getCode() && b.asset_issuer === asset.getIssuer())
        );
      })
      .catch(() => false);

    if (await hasTrustline) {
      return null;
    }

    // Create the CHANGE_TRUST operation
    return Operation.changeTrust({
      asset,
      limit,
    });
  }

  /**
   * createTrustlines - Batch creates multiple trustlines
   */
  async createTrustlines(
    accountId: string,
    assets: Asset[]
  ): Promise<Operation[]> {
    const operations: Operation[] = [];
    
    for (const asset of assets) {
      const op = await this.ensureTrustline(accountId, asset);
      if (op) {
        operations.push(op);
      }
    }
    
    return operations;
  }
}
