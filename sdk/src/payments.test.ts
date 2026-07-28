import {
  metadataToScVal, scValToMetadata,
  toScValAddress, toScValOption,
  StellarPayments,
} from './payments';

test('converts flat metadata to SCVal map and back', () => {
  const meta = { ref: 'INV-001', amount: 42, verified: true };
  const scVal  = metadataToScVal(meta);
  const result = scValToMetadata(scVal);
  expect(result.ref).toBe('INV-001');
  expect(result.verified).toBe(true);
});

test('throws on non-finite number in metadata', () => {
  expect(() => metadataToScVal({ bad: Infinity })).toThrow('finite number');
});

test('throws on invalid Stellar address', () => {
  expect(() => toScValAddress('not-an-address')).toThrow('SCVal address');
});

test('toScValOption returns void for null', () => {
  const opt = toScValOption(null);
  expect(opt.switch().name).toBe('scvVoid');
});

test('toScValOption wraps a value in Some', () => {
  const inner = toScValOption(null); // void as placeholder
  const opt   = toScValOption(inner);
  expect(opt.switch().name).toBe('scvVec');
});

test('convertMetadataToScVal handles empty, optional, and different types', () => {
  const payments = new StellarPayments(null as any);
  
  // 1. Empty/missing metadata
  const emptyVal1 = (payments as any).convertMetadataToScVal();
  const emptyVal2 = (payments as any).convertMetadataToScVal({});
  const emptyVal3 = (payments as any).convertMetadataToScVal(null as any);
  
  expect(emptyVal1.switch().name).toBe('scvMap');
  expect(emptyVal1.map() || []).toHaveLength(0);
  expect(emptyVal2.map() || []).toHaveLength(0);
  expect(emptyVal3.map() || []).toHaveLength(0);

  // 2. Optional keys (null/undefined) and different types
  const data = {
    strKey: 'hello',
    numKey: 42,
    bigKey: 100n,
    boolKey: true,
    bytesKey: new Uint8Array([1, 2, 3]),
    optNull: null,
    optUndef: undefined,
  };

  const scVal = (payments as any).convertMetadataToScVal(data);
  expect(scVal.switch().name).toBe('scvMap');
  
  const entries = scVal.map() || [];
  expect(entries).toHaveLength(5); // strKey, numKey, bigKey, boolKey, bytesKey (optNull and optUndef are omitted)

  const findEntry = (key: string) => entries.find(e => e.key().sym().toString() === key);
  
  expect(findEntry('optNull')).toBeUndefined();
  expect(findEntry('optUndef')).toBeUndefined();

  expect(findEntry('strKey')?.val().bytes().toString()).toBe(Buffer.from('hello').toString());
  expect(findEntry('numKey')?.val().bytes().toString()).toBe(Buffer.from('42').toString());
  expect(findEntry('bigKey')?.val().bytes().toString()).toBe(Buffer.from('100').toString());
  expect(findEntry('boolKey')?.val().bytes().toString()).toBe(Buffer.from('true').toString());
  expect(findEntry('bytesKey')?.val().bytes().toString()).toBe(Buffer.from([1, 2, 3]).toString());
});

test('convertMetadataToScVal throws on unsupported types', () => {
  const payments = new StellarPayments(null as any);
  expect(() => {
    (payments as any).convertMetadataToScVal({ bad: { nested: 1 } });
  }).toThrow('unsupported value type');
});