/**
 * Tests for the JSON parser.
 *
 * Uses temp files to remain self-contained without fixture files on disk.
 */

import * as fs from 'fs';
import * as os from 'os';
import * as path from 'path';
import { parseJSON } from './json-parser';

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function writeTempJSON(content: string): string {
  const tmpFile = path.join(
    os.tmpdir(),
    `json-test-${Date.now()}-${Math.random().toString(36).slice(2)}.json`,
  );
  fs.writeFileSync(tmpFile, content, 'utf-8');
  return tmpFile;
}

function parseJSONString(content: string) {
  const file = writeTempJSON(content);
  try {
    return parseJSON(file);
  } finally {
    fs.unlinkSync(file);
  }
}

// ---------------------------------------------------------------------------
// Sample inputs
// ---------------------------------------------------------------------------

const ARRAY_FORMAT = JSON.stringify([
  {
    destination: 'GBDEVU63Y6BHHYWUMAS6NHXVWUIQEJBACC7F6QXJZUCM4TBNEOUFL2',
    amount: '150',
    asset: 'USDC',
    asset_issuer: 'GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN',
    memo: 'Invoice-007',
    escrow_duration: 3600,
  },
  {
    destination: 'GCEZWKCA5VLDNRLN3RPRJMRZOX3Z6G5CHCGVA2XM9DWBF32LLVBF3',
    amount: 75,
    asset: 'XLM',
    asset_issuer: '',
    memo: '',
    escrow_duration: 0,
  },
]);

const OBJECT_WRAPPER_FORMAT = JSON.stringify({
  payments: [
    {
      destination: 'GBDEVU63Y6BHHYWUMAS6NHXVWUIQEJBACC7F6QXJZUCM4TBNEOUFL2',
      amount: '200',
      asset: 'EURC',
      memo: 'Payroll-Q1',
    },
  ],
});

const NUMERIC_AMOUNT_FORMAT = JSON.stringify([
  { destination: 'GBDEVU63Y6BHHYWUMAS6NHXVWUIQEJBACC7F6QXJZUCM4TBNEOUFL2', amount: 42 },
]);

const MISSING_OPTIONAL_FIELDS = JSON.stringify([
  { destination: 'GBDEVU63Y6BHHYWUMAS6NHXVWUIQEJBACC7F6QXJZUCM4TBNEOUFL2', amount: '10' },
]);

const EMPTY_ARRAY = JSON.stringify([]);

const EMPTY_PAYMENTS_WRAPPER = JSON.stringify({ payments: [] });

// ---------------------------------------------------------------------------
// Tests — array format
// ---------------------------------------------------------------------------

describe('parseJSON — top-level array format', () => {
  it('returns one record per array entry', () => {
    const records = parseJSONString(ARRAY_FORMAT);
    expect(records).toHaveLength(2);
  });

  it('maps destination correctly', () => {
    const [first] = parseJSONString(ARRAY_FORMAT);
    expect(first.destination).toBe('GBDEVU63Y6BHHYWUMAS6NHXVWUIQEJBACC7F6QXJZUCM4TBNEOUFL2');
  });

  it('maps string amount correctly', () => {
    const [first] = parseJSONString(ARRAY_FORMAT);
    expect(first.amount).toBe('150');
  });

  it('coerces numeric amount to string', () => {
    const [, second] = parseJSONString(ARRAY_FORMAT);
    expect(second.amount).toBe('75');
    expect(typeof second.amount).toBe('string');
  });

  it('maps asset correctly', () => {
    const [first] = parseJSONString(ARRAY_FORMAT);
    expect(first.asset).toBe('USDC');
  });

  it('maps asset_issuer correctly', () => {
    const [first] = parseJSONString(ARRAY_FORMAT);
    expect(first.asset_issuer).toBe('GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN');
  });

  it('maps memo correctly', () => {
    const [first] = parseJSONString(ARRAY_FORMAT);
    expect(first.memo).toBe('Invoice-007');
  });

  it('maps escrow_duration correctly', () => {
    const [first] = parseJSONString(ARRAY_FORMAT);
    expect(first.escrow_duration).toBe(3600);
  });
});

// ---------------------------------------------------------------------------
// Tests — { payments: [...] } wrapper format
// ---------------------------------------------------------------------------

describe('parseJSON — object wrapper { payments: [...] } format', () => {
  it('reads entries from the payments key', () => {
    const records = parseJSONString(OBJECT_WRAPPER_FORMAT);
    expect(records).toHaveLength(1);
  });

  it('maps fields from wrapped format correctly', () => {
    const [record] = parseJSONString(OBJECT_WRAPPER_FORMAT);
    expect(record.destination).toBe('GBDEVU63Y6BHHYWUMAS6NHXVWUIQEJBACC7F6QXJZUCM4TBNEOUFL2');
    expect(record.amount).toBe('200');
    expect(record.asset).toBe('EURC');
    expect(record.memo).toBe('Payroll-Q1');
  });
});

// ---------------------------------------------------------------------------
// Tests — default / fallback values
// ---------------------------------------------------------------------------

describe('parseJSON — default values for missing optional fields', () => {
  it('defaults asset to XLM', () => {
    const [record] = parseJSONString(MISSING_OPTIONAL_FIELDS);
    expect(record.asset).toBe('XLM');
  });

  it('defaults asset_issuer to empty string', () => {
    const [record] = parseJSONString(MISSING_OPTIONAL_FIELDS);
    expect(record.asset_issuer).toBe('');
  });

  it('defaults memo to empty string', () => {
    const [record] = parseJSONString(MISSING_OPTIONAL_FIELDS);
    expect(record.memo).toBe('');
  });

  it('defaults escrow_duration to 0', () => {
    const [record] = parseJSONString(MISSING_OPTIONAL_FIELDS);
    expect(record.escrow_duration).toBe(0);
  });
});

// ---------------------------------------------------------------------------
// Tests — numeric amount coercion
// ---------------------------------------------------------------------------

describe('parseJSON — numeric amount coercion', () => {
  it('converts numeric amount to string', () => {
    const [record] = parseJSONString(NUMERIC_AMOUNT_FORMAT);
    expect(record.amount).toBe('42');
    expect(typeof record.amount).toBe('string');
  });
});

// ---------------------------------------------------------------------------
// Tests — edge cases
// ---------------------------------------------------------------------------

describe('parseJSON — edge cases', () => {
  it('returns empty array for an empty top-level array', () => {
    expect(parseJSONString(EMPTY_ARRAY)).toHaveLength(0);
  });

  it('returns empty array for an empty payments wrapper', () => {
    expect(parseJSONString(EMPTY_PAYMENTS_WRAPPER)).toHaveLength(0);
  });

  it('returns all required PaymentRecord keys on every record', () => {
    const records = parseJSONString(ARRAY_FORMAT);
    for (const record of records) {
      expect(record).toHaveProperty('destination');
      expect(record).toHaveProperty('amount');
      expect(record).toHaveProperty('asset');
      expect(record).toHaveProperty('asset_issuer');
      expect(record).toHaveProperty('memo');
      expect(record).toHaveProperty('escrow_duration');
    }
  });

  it('throws on malformed JSON', () => {
    expect(() => parseJSONString('{ invalid json')).toThrow();
  });

  it('throws when the file does not exist', () => {
    expect(() => parseJSON('/nonexistent/path/file.json')).toThrow();
  });
});
