import * as XLSX from 'xlsx';
import { PaymentRecord } from '../types';

interface XLSXRow {
  destination?: string;
  amount?: string | number;
  asset?: string;
  asset_issuer?: string;
  memo?: string;
  escrow_duration?: string | number;
}

export function parseXLSX(filePath: string): PaymentRecord[] {
  const workbook = XLSX.readFile(filePath);
  const sheetName = workbook.SheetNames[0];
  const sheet = workbook.Sheets[sheetName];
  const rows = XLSX.utils.sheet_to_json<XLSXRow>(sheet);

  return rows.map((row) => ({
    destination: String(row.destination || ''),
    amount: String(row.amount || '0'),
    asset: String(row.asset || 'XLM'),
    asset_issuer: String(row.asset_issuer || ''),
    memo: String(row.memo || ''),
    escrow_duration: Number(row.escrow_duration) || 0,
  }));
}
