# Performance Expectations

## Rate Oracle Aggregation

| Sources | Expected throughput |
|---------|-------------------|
| 5       | > 500,000 ops/sec |
| 100     | > 50,000 ops/sec  |

> Note: `bench:oracle` is not yet implemented. Validate rate aggregation with `npm run test:sdk`.

## Batch Submission (validation only, no network)

| Batch size | Expected throughput |
|------------|-------------------|
| 100        | > 100,000 rec/sec |
| 1,000      | > 80,000 rec/sec  |
| 10,000     | > 50,000 rec/sec  |

> Note: `bench:batch` is not yet implemented. Validate batch logic with `npm run test:cli`.

## Real-world network throughput
Horizon testnet typically processes 50–100 tx/sec sustained.
For large batches (> 500 payments), use the `--dry-run` flag first
to validate all records before submission.

## Re-running benchmarks
Benchmarks are deterministic (no network I/O) and should be
re-run after any changes to rate aggregation or batch validation logic.
Once `bench:*` scripts land, this document should reference them; until then
use the test commands above.
