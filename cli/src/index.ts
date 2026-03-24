#!/usr/bin/env node

import { Command } from 'commander';
import * as path from 'path';
import * as dotenv from 'dotenv';
import { executeBatch } from './commands/batch';
import { executeStatus } from './commands/status';
import { executeRetry } from './commands/retry';
import { executeReport } from './commands/report';
import { detectFormat } from './parsers';
import {
  InputFormat,
  NetworkType,
  ReportFormat,
} from './types';
import { setLogLevel, LogLevel } from './utils/logger';

dotenv.config();

const program = new Command();

program
  .name('stellar-payout')
  .description('CLI tool for batch cross-border payments on the Stellar network')
  .version('0.1.0');

// ── batch command ────────────────────────────────────────────────────
program
  .command('batch')
  .description('Process batch payments from CSV, JSON, XLSX, or SWIFT MT103 files')
  .requiredOption('-i, --input <file>', 'Input file path (CSV, JSON, XLSX, or MT103)')
  .option('-f, --format <format>', 'Input format: csv, json, xlsx, mt103 (auto-detected from extension)')
  .requiredOption('-s, --source-secret <key>', 'Source account secret key', process.env.ADMIN_SECRET_KEY)
  .option('-n, --network <network>', 'Network: testnet, mainnet, futurenet', 'testnet')
  .option('--horizon-url <url>', 'Horizon URL', process.env.HORIZON_URL || 'https://horizon-testnet.stellar.org')
  .option('--network-passphrase <passphrase>', 'Network passphrase (auto-detected from network)')
  .option('--dry-run', 'Simulate transactions without submitting', false)
  .option('--max-ops <number>', 'Maximum operations per transaction (max 100)', '100')
  .option('--max-fee <number>', 'Maximum fee in stroops', '10000')
  .option('--concurrency <number>', 'Number of concurrent transaction submissions', '5')
  .option('--fee-surge-threshold <number>', 'Fee surge threshold in stroops (pauses if exceeded)', '100')
  .option('--rate-lock-minutes <number>', 'Rate lock window in minutes', '10')
  .option('--escrow-contract <address>', 'Escrow contract address', process.env.ESCROW_CONTRACT_ADDRESS)
  .option('--rate-oracle-contract <address>', 'Rate oracle contract address', process.env.RATE_ORACLE_CONTRACT_ADDRESS)
  .option('--compliance-contract <address>', 'Compliance contract address', process.env.COMPLIANCE_CONTRACT_ADDRESS)
  .option('--db-path <path>', 'SQLite database path for crash recovery', './stellar-payout.db')
  .option('--verbose', 'Enable verbose logging', false)
  .action(async (opts) => {
    if (opts.verbose) {
      setLogLevel(LogLevel.DEBUG);
    }

    const format = opts.format
      ? (opts.format as InputFormat)
      : detectFormat(opts.input);

    await executeBatch({
      inputFile: path.resolve(opts.input),
      format,
      sourceSecret: opts.sourceSecret,
      network: opts.network as NetworkType,
      horizonUrl: opts.horizonUrl,
      networkPassphrase: opts.networkPassphrase || '',
      dryRun: opts.dryRun,
      maxOpsPerTx: Math.min(parseInt(opts.maxOps, 10), 100),
      maxFee: parseInt(opts.maxFee, 10),
      concurrency: parseInt(opts.concurrency, 10),
      feeSurgeThreshold: parseInt(opts.feeSurgeThreshold, 10),
      rateLockMinutes: parseInt(opts.rateLockMinutes, 10),
      escrowContractAddress: opts.escrowContract || '',
      rateOracleContractAddress: opts.rateOracleContract || '',
      complianceContractAddress: opts.complianceContract || '',
      dbPath: opts.dbPath,
    });
  });

// ── status command ───────────────────────────────────────────────────
program
  .command('status')
  .description('Real-time monitoring of batch payment status with Horizon streaming')
  .option('-b, --batch-id <id>', 'Batch ID to monitor (shows recent batches if omitted)')
  .option('-f, --follow', 'Stream real-time updates', false)
  .option('--horizon-url <url>', 'Horizon URL', process.env.HORIZON_URL || 'https://horizon-testnet.stellar.org')
  .option('--db-path <path>', 'SQLite database path', './stellar-payout.db')
  .option('--verbose', 'Enable verbose logging', false)
  .action(async (opts) => {
    if (opts.verbose) {
      setLogLevel(LogLevel.DEBUG);
    }

    await executeStatus({
      batchId: opts.batchId || '',
      follow: opts.follow,
      dbPath: opts.dbPath,
      horizonUrl: opts.horizonUrl,
    });
  });

// ── retry command ────────────────────────────────────────────────────
program
  .command('retry')
  .description('Automatically resubmit failed transactions with exponential backoff')
  .requiredOption('-b, --batch-id <id>', 'Batch ID to retry failed entries')
  .requiredOption('-s, --source-secret <key>', 'Source account secret key', process.env.ADMIN_SECRET_KEY)
  .option('--max-retries <number>', 'Maximum retry attempts per entry', '3')
  .option('--backoff-base <ms>', 'Base backoff delay in milliseconds', '1000')
  .option('--backoff-max <ms>', 'Maximum backoff delay in milliseconds', '30000')
  .option('--horizon-url <url>', 'Horizon URL', process.env.HORIZON_URL || 'https://horizon-testnet.stellar.org')
  .option('--network-passphrase <passphrase>', 'Network passphrase', process.env.NETWORK_PASSPHRASE || 'Test SDF Network ; September 2015')
  .option('--db-path <path>', 'SQLite database path', './stellar-payout.db')
  .option('--verbose', 'Enable verbose logging', false)
  .action(async (opts) => {
    if (opts.verbose) {
      setLogLevel(LogLevel.DEBUG);
    }

    await executeRetry({
      batchId: opts.batchId,
      maxRetries: parseInt(opts.maxRetries, 10),
      backoffBase: parseInt(opts.backoffBase, 10),
      backoffMax: parseInt(opts.backoffMax, 10),
      dbPath: opts.dbPath,
      sourceSecret: opts.sourceSecret,
      horizonUrl: opts.horizonUrl,
      networkPassphrase: opts.networkPassphrase,
    });
  });

// ── report command ───────────────────────────────────────────────────
program
  .command('report')
  .description('Generate compliance audit trail reports in PDF or CSV format')
  .requiredOption('-b, --batch-id <id>', 'Batch ID to generate report for')
  .option('--format <format>', 'Report format: pdf or csv', 'csv')
  .option('-o, --output <path>', 'Output file path (auto-generated if omitted)')
  .option('--db-path <path>', 'SQLite database path', './stellar-payout.db')
  .option('--verbose', 'Enable verbose logging', false)
  .action(async (opts) => {
    if (opts.verbose) {
      setLogLevel(LogLevel.DEBUG);
    }

    await executeReport({
      batchId: opts.batchId,
      format: opts.format as ReportFormat,
      outputPath: opts.output || '',
      dbPath: opts.dbPath,
    });
  });

program.parse();
