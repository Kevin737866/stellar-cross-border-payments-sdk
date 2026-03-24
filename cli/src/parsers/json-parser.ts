import * as fs from 'fs';
import { PaymentRecord } from '../types';

interface JSONPaymentInput {
  destination?: string;
  amount?: string | number;
  asset?: string;
  asset_issuer?: string;
  memo?: string;
  escrow_duration?: number;
}

export function parseJSON(filePath: string): PaymentRecord[] {
  const content = fs.readFileSync(filePath, 'utf-8');
  const data = JSON.parse(content);

  const payments: JSONPaymentInput[] = Array.isArray(data) ? data : data.payments || [];

  return payments.map((entry) => ({
    destination: entry.destination || '',
    amount: String(entry.amount || '0'),
    asset: entry.asset || 'XLM',
    asset_issuer: entry.asset_issuer || '',
    memo: entry.memo || '',
    escrow_duration: entry.escrow_duration || 0,
  }));
}
