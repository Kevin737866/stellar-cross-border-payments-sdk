import { detectFormat } from './index';
import { InputFormat } from '../types';

describe('detectFormat — case-insensitive file extension detection', () => {
  // CSV
  test('detects lowercase .csv', () => {
    expect(detectFormat('payments.csv')).toBe(InputFormat.CSV);
  });
  test('detects uppercase .CSV', () => {
    expect(detectFormat('PAYMENTS.CSV')).toBe(InputFormat.CSV);
  });

  // JSON
  test('detects lowercase .json', () => {
    expect(detectFormat('data.json')).toBe(InputFormat.JSON);
  });
  test('detects mixed-case .Json', () => {
    expect(detectFormat('data.Json')).toBe(InputFormat.JSON);
  });

  // XLSX
  test('detects lowercase .xlsx', () => {
    expect(detectFormat('book.xlsx')).toBe(InputFormat.XLSX);
  });
  test('detects uppercase .XLSX', () => {
    expect(detectFormat('book.XLSX')).toBe(InputFormat.XLSX);
  });
  test('detects uppercase .XLS alias', () => {
    expect(detectFormat('book.XLS')).toBe(InputFormat.XLSX);
  });

  // MT103
  test('detects lowercase .mt103', () => {
    expect(detectFormat('wire.mt103')).toBe(InputFormat.MT103);
  });
  test('detects uppercase .MT103', () => {
    expect(detectFormat('wire.MT103')).toBe(InputFormat.MT103);
  });
  test('detects uppercase .SWIFT alias', () => {
    expect(detectFormat('wire.SWIFT')).toBe(InputFormat.MT103);
  });

  // Path + default behaviour
  test('detects uppercase extension within a mixed-case path', () => {
    expect(detectFormat('/Data/Q1/Payments.CSV')).toBe(InputFormat.CSV);
  });
  test('defaults to CSV for an unknown extension', () => {
    expect(detectFormat('file.unknown')).toBe(InputFormat.CSV);
  });

  // Edge cases
  test('handles multiple dots in filename', () => {
    expect(detectFormat('data.payments.csv')).toBe(InputFormat.CSV);
  });
  test('handles file without extension', () => {
    expect(detectFormat('README')).toBe(InputFormat.CSV);
  });
  test('handles just an extension string', () => {
    expect(detectFormat('.json')).toBe(InputFormat.JSON);
  });
  test('handles empty string', () => {
    expect(detectFormat('')).toBe(InputFormat.CSV);
  });
  test('handles XLSX with mixed case', () => {
    expect(detectFormat('report.XlSx')).toBe(InputFormat.XLSX);
  });
  test('handles SWIFT extension with mixed case', () => {
    expect(detectFormat('wire.Swift')).toBe(InputFormat.MT103);
  });
});
