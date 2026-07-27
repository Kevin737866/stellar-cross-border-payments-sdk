import * as XLSX from 'xlsx';
import { PaymentRecord } from '../types';

const HEADER_ALIASES: Record<string, string> = {
  destination: 'destination',
  dest: 'destination',
  recipient: 'destination',
  'recipient address': 'destination',
  'wallet address': 'destination',
  address: 'destination',
  'stellar address': 'destination',
  amount: 'amount',
  amt: 'amount',
  value: 'amount',
  'payment amount': 'amount',
  asset: 'asset',
  currency: 'asset',
  token: 'asset',
  'asset code': 'asset',
  coin: 'asset',
  asset_issuer: 'asset_issuer',
  'asset issuer': 'asset_issuer',
  issuer: 'asset_issuer',
  'issuer address': 'asset_issuer',
  memo: 'memo',
  note: 'memo',
  notes: 'memo',
  reference: 'memo',
  description: 'memo',
  ref: 'memo',
  escrow_duration: 'escrow_duration',
  'escrow duration': 'escrow_duration',
  escrow: 'escrow_duration',
  'lock duration': 'escrow_duration',
  'lock time': 'escrow_duration',
  duration: 'escrow_duration',
};

const REQUIRED_COLUMNS = ['destination', 'amount'] as const;

export interface XLSXRowError {
  row: number;
  errors: string[];
}

export interface XLSXParseResult {
  records: PaymentRecord[];
  errors: XLSXRowError[];
}

export function parseXLSX(filePath: string): XLSXParseResult {
  const workbook = XLSX.readFile(filePath);
  const sheetName = workbook.SheetNames[0];
  const sheet = workbook.Sheets[sheetName];

  const rawRows = XLSX.utils.sheet_to_json<Record<string, unknown>>(sheet, {
    raw: true,
    defval: null,
  });

  if (rawRows.length === 0) {
    return { records: [], errors: [] };
  }

  const rawHeaders = Object.keys(rawRows[0]);
  const headerMap = buildHeaderMap(rawHeaders);

  const missingRequired = REQUIRED_COLUMNS.filter((col) => !headerMap.has(col));
  if (missingRequired.length > 0) {
    throw new Error(
      `XLSX file is missing required column(s): ${missingRequired.join(', ')}. ` +
        `Found headers: ${rawHeaders.join(', ')}`,
    );
  }

  const records: PaymentRecord[] = [];
  const errors: XLSXRowError[] = [];

  for (let i = 0; i < rawRows.length; i++) {
    const rowNumber = i + 2;
    const rawRow = rawRows[i];
    const row = remapRow(rawRow, headerMap);
    const rowErrors = validateRow(row, rowNumber);
    if (rowErrors.length > 0) {
      errors.push({ row: rowNumber, errors: rowErrors });
      continue;
    }
    records.push(buildRecord(row));
  }

  return { records, errors };
}

function buildHeaderMap(rawHeaders: string[]): Map<string, string> {
  const map = new Map<string, string>();
  for (const raw of rawHeaders) {
    const normalised = String(raw).trim().toLowerCase().replace(/\s+/g, ' ');
    const canonical = HEADER_ALIASES[normalised];
    if (canonical && !map.has(canonical)) {
      map.set(canonical, raw as string);
    }
  }
  return map;
}

function remapRow(
  rawRow: Record<string, unknown>,
  headerMap: Map<string, string>,
): Record<string, unknown> {
  const out: Record<string, unknown> = {};
  for (const [canonical, rawHeader] of headerMap.entries()) {
    const value = rawRow[rawHeader as string];
    if (value === null || value === undefined) {
      out[canonical] = '';
    } else if (typeof value === 'number') {
      out[canonical] = String(value);
    } else {
      out[canonical] = String(value).trim();
    }
  }
  return out;
}

function validateRow(row: Record<string, unknown>, rowNumber: number): string[] {
  const errors: string[] = [];

  const destination = row.destination as string;
  if (!destination || destination.trim().length === 0) {
    errors.push(`Row ${rowNumber}: missing required field "destination"`);
  } else if (destination.length < 10) {
    errors.push(`Row ${rowNumber}: "destination" appears too short for a valid address`);
  }

  const rawAmount = row.amount as string;
  if (!rawAmount || rawAmount.trim().length === 0) {
    errors.push(`Row ${rowNumber}: missing required field "amount"`);
  } else {
    const cleanAmount = rawAmount.replace(/[$,€£\s]/g, '');
    const num = Number(cleanAmount);
    if (isNaN(num)) {
      errors.push(`Row ${rowNumber}: "amount" is not a valid number ("${rawAmount}")`);
    } else if (num <= 0) {
      errors.push(`Row ${rowNumber}: "amount" must be greater than zero (got ${num})`);
    }
  }

  const rawEscrow = row.escrow_duration as string;
  if (rawEscrow && rawEscrow.trim().length > 0) {
    const escrowNum = Number(rawEscrow);
    if (isNaN(escrowNum)) {
      errors.push(
        `Row ${rowNumber}: "escrow_duration" is not a valid number ("${rawEscrow}")`,
      );
    } else if (!Number.isInteger(escrowNum) || escrowNum < 0) {
      errors.push(
        `Row ${rowNumber}: "escrow_duration" must be a non-negative integer (got ${escrowNum})`,
      );
    }
  }

  return errors;
}

function buildRecord(row: Record<string, unknown>): PaymentRecord {
  return {
    destination: (row.destination as string) || '',
    amount: (row.amount as string) || '0',
    asset: (row.asset as string) || 'XLM',
    asset_issuer: (row.asset_issuer as string) || '',
    memo: (row.memo as string) || '',
    escrow_duration: parseEscrowDuration(row.escrow_duration),
  };
}

function parseEscrowDuration(value: unknown): number {
  if (value === undefined || value === null || value === '') return 0;
  const n = Number(value);
  return Number.isFinite(n) && n >= 0 ? Math.floor(n) : 0;
}