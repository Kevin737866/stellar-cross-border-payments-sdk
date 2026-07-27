import * as fs from 'fs';
import { parse } from 'csv-parse/sync';
import { PaymentRecord } from '../types';

/** Required CSV column names for a valid payment batch */
const REQUIRED_COLUMNS = ['destination', 'amount'] as const;

export function parseCSV(filePath: string): PaymentRecord[] {
  const content = fs.readFileSync(filePath, 'utf-8');

  // Parse header row first to validate required columns before processing data
  const headerLine = content.split('\n')[0] ?? '';
  const headerColumns = headerLine
    .split(',')
    .map((col) => col.trim().toLowerCase());

  const missingColumns = REQUIRED_COLUMNS.filter(
    (required) => !headerColumns.includes(required)
  );

  if (missingColumns.length > 0) {
    const columnList = missingColumns.map((c) => `"${c}"`).join(', ');
    throw new Error(
      `Missing required CSV columns: ${columnList}. ` +
      `The CSV file must contain at least the following columns: ${REQUIRED_COLUMNS.map((c) => `"${c}"`).join(', ')}. ` +
      `Found columns: ${headerColumns.map((c) => `"${c}"`).join(', ') || '(none)'}.`
    );
  }

  const records = parse(content, {
    columns: true,
    skip_empty_lines: true,
    trim: true,
    cast: (value: string, context: { column: string | number }) => {
      if (context.column === 'escrow_duration') {
        return parseInt(value, 10) || 0;
      }
      return value;
    },
  }) as Array<Record<string, string | number>>;

  return records.map((row) => ({
    destination: String(row.destination || ''),
    amount: String(row.amount || '0'),
    asset: String(row.asset || 'XLM'),
    asset_issuer: String(row.asset_issuer || ''),
    memo: String(row.memo || ''),
    escrow_duration: Number(row.escrow_duration) || 0,
  }));
}
