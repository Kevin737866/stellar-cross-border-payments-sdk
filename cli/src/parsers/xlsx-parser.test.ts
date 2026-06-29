/**
 * Tests for the XLSX parser.
 *
 * Uses the `xlsx` library to programmatically build workbooks in memory and
 * write them to temp files, so no fixture files are needed on disk.
 */

import * as fs from 'fs';
import * as os from 'os';
import * as path from 'path';
import * as XLSX from 'xlsx';
import { parseXLSX } from './xlsx-parser';

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

type SheetRow = Record<string, string | number | undefined>;

/** Build a single-sheet .xlsx temp file from an array of row objects. */
function writeTempXLSX(rows: SheetRow[]): string {
  const tmpFile = path.join(
    os.tmpdir(),
    `xlsx-test-${Date.now()}-${Math.random().toString(36).slice(2)}.xlsx`,
  );
  const ws = XLSX.utils.json_to_sheet(rows);
  const wb = XLSX.utils.book_new();
  XLSX.utils.book_append_sheet(wb, ws, 'Sheet1');
  XLSX.writeFile(wb, tmpFile);
  return tmpFile;
}

function parseRows(rows: SheetRow[]) {
  const file = writeTempXLSX(rows);
  try {
    return parseXLSX(file);
  } finally {
    fs.unlinkSync(file);
  }
}

// ---------------------------------------------------------------------------
// Sample row data
// ---------------------------------------------------------------------------

const FULL_ROWS: SheetRow[] = [
  {
    destination: 'GBDEVU63Y6BHHYWUMAS6NHXVWUIQEJBACC7F6QXJZUCM4TBNEOUFL2',
    amount: '100',
    asset: 'USDC',
    asset_issuer: 'GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN',
    memo: 'Invoice-001',
    escrow_duration: 3600,
  },
  {
    destination: 'GCEZWKCA5VLDNRLN3RPRJMRZOX3Z6G5CHCGVA2XM9DWBF32LLVBF3',
    amount: 50,
    asset: 'XLM',
    asset_issuer: '',
    memo: '',
    escrow_duration: 0,
  },
];

const MINIMAL_ROWS: SheetRow[] = [
  { destination: 'GBDEVU63Y6BHHYWUMAS6NHXVWUIQEJBACC7F6QXJZUCM4TBNEOUFL2', amount: '200' },
];

const NUMERIC_AMOUNT_ROW: SheetRow[] = [
  { destination: 'GBDEVU63Y6BHHYWUMAS6NHXVWUIQEJBACC7F6QXJZUCM4TBNEOUFL2', amount: 77 },
];

const INVALID_ESCROW_ROW: SheetRow[] = [
  {
    destination: 'GBDEVU63Y6BHHYWUMAS6NHXVWUIQEJBACC7F6QXJZUCM4TBNEOUFL2',
    amount: '10',
    escrow_duration: 'bad',
  },
];

// ---------------------------------------------------------------------------
// Tests — valid inputs
// ---------------------------------------------------------------------------

describe('parseXLSX — valid inputs', () => {
  it('returns one record per data row', () => {
    expect(parseRows(FULL_ROWS)).toHaveLength(2);
  });

  it('maps destination correctly', () => {
    const [first] = parseRows(FULL_ROWS);
    expect(first.destination).toBe('GBDEVU63Y6BHHYWUMAS6NHXVWUIQEJBACC7F6QXJZUCM4TBNEOUFL2');
  });

  it('maps string amount correctly', () => {
    const [first] = parseRows(FULL_ROWS);
    expect(first.amount).toBe('100');
  });

  it('coerces numeric amount to string', () => {
    const [, second] = parseRows(FULL_ROWS);
    expect(second.amount).toBe('50');
    expect(typeof second.amount).toBe('string');
  });

  it('maps asset correctly', () => {
    const [first] = parseRows(FULL_ROWS);
    expect(first.asset).toBe('USDC');
  });

  it('maps asset_issuer correctly', () => {
    const [first] = parseRows(FULL_ROWS);
    expect(first.asset_issuer).toBe('GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN');
  });

  it('maps memo correctly', () => {
    const [first] = parseRows(FULL_ROWS);
    expect(first.memo).toBe('Invoice-001');
  });

  it('maps escrow_duration as a number', () => {
    const [first] = parseRows(FULL_ROWS);
    expect(first.escrow_duration).toBe(3600);
  });

  it('maps second row independently', () => {
    const [, second] = parseRows(FULL_ROWS);
    expect(second.destination).toBe('GCEZWKCA5VLDNRLN3RPRJMRZOX3Z6G5CHCGVA2XM9DWBF32LLVBF3');
    expect(second.amount).toBe('50');
    expect(second.asset).toBe('XLM');
    expect(second.escrow_duration).toBe(0);
  });
});

// ---------------------------------------------------------------------------
// Tests — default / fallback values
// ---------------------------------------------------------------------------

describe('parseXLSX — default values for missing optional columns', () => {
  it('defaults asset to XLM', () => {
    const [record] = parseRows(MINIMAL_ROWS);
    expect(record.asset).toBe('XLM');
  });

  it('defaults asset_issuer to empty string', () => {
    const [record] = parseRows(MINIMAL_ROWS);
    expect(record.asset_issuer).toBe('');
  });

  it('defaults memo to empty string', () => {
    const [record] = parseRows(MINIMAL_ROWS);
    expect(record.memo).toBe('');
  });

  it('defaults escrow_duration to 0', () => {
    const [record] = parseRows(MINIMAL_ROWS);
    expect(record.escrow_duration).toBe(0);
  });
});

// ---------------------------------------------------------------------------
// Tests — edge cases
// ---------------------------------------------------------------------------

describe('parseXLSX — edge cases', () => {
  it('returns empty array for a sheet with no data rows', () => {
    const records = parseRows([]);
    expect(records).toHaveLength(0);
  });

  it('coerces numeric cell amount to string', () => {
    const [record] = parseRows(NUMERIC_AMOUNT_ROW);
    expect(record.amount).toBe('77');
    expect(typeof record.amount).toBe('string');
  });

  it('treats non-numeric escrow_duration as 0', () => {
    const [record] = parseRows(INVALID_ESCROW_ROW);
    expect(record.escrow_duration).toBe(0);
  });

  it('uses only the first sheet when the workbook has multiple sheets', () => {
    const tmpFile = path.join(
      os.tmpdir(),
      `xlsx-multi-${Date.now()}.xlsx`,
    );
    const ws1 = XLSX.utils.json_to_sheet([
      { destination: 'GBDEVU63Y6BHHYWUMAS6NHXVWUIQEJBACC7F6QXJZUCM4TBNEOUFL2', amount: '10' },
    ]);
    const ws2 = XLSX.utils.json_to_sheet([
      { destination: 'GCEZWKCA5VLDNRLN3RPRJMRZOX3Z6G5CHCGVA2XM9DWBF32LLVBF3', amount: '20' },
      { destination: 'GCEZWKCA5VLDNRLN3RPRJMRZOX3Z6G5CHCGVA2XM9DWBF32LLVBF3', amount: '30' },
    ]);
    const wb = XLSX.utils.book_new();
    XLSX.utils.book_append_sheet(wb, ws1, 'Payments');
    XLSX.utils.book_append_sheet(wb, ws2, 'Other');
    XLSX.writeFile(wb, tmpFile);
    try {
      const records = parseXLSX(tmpFile);
      expect(records).toHaveLength(1);
      expect(records[0].amount).toBe('10');
    } finally {
      fs.unlinkSync(tmpFile);
    }
  });

  it('returns all required PaymentRecord keys on every record', () => {
    const records = parseRows(FULL_ROWS);
    for (const record of records) {
      expect(record).toHaveProperty('destination');
      expect(record).toHaveProperty('amount');
      expect(record).toHaveProperty('asset');
      expect(record).toHaveProperty('asset_issuer');
      expect(record).toHaveProperty('memo');
      expect(record).toHaveProperty('escrow_duration');
    }
  });

  it('throws when the file does not exist', () => {
    expect(() => parseXLSX('/nonexistent/path/file.xlsx')).toThrow();
  });
});
