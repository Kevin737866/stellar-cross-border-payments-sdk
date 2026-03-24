import {
  Keypair,
  Account,
  TransactionBuilder,
  Operation,
  Asset,
  Memo,
} from 'stellar-sdk';
import axios from 'axios';
import { RetryOptions, BatchEntryStatus, BatchStatus, BatchPaymentEntry } from '../types';
import { BatchDatabase } from '../utils/database';
import * as logger from '../utils/logger';

export async function executeRetry(options: RetryOptions): Promise<void> {
  const db = new BatchDatabase(options.dbPath);

  logger.banner('Stellar Payout - Retry Failed Transactions');

  const batch = db.getBatch(options.batchId);
  if (!batch) {
    logger.error(`Batch not found: ${options.batchId}`);
    db.close();
    return;
  }

  const failedEntries = db.getFailedEntries(options.batchId);
  if (failedEntries.length === 0) {
    logger.success('No failed entries to retry.');
    db.close();
    return;
  }

  logger.info(`Found ${failedEntries.length} failed entries to retry`);
  logger.info(`Max retries per entry: ${options.maxRetries}`);
  logger.info(`Backoff base: ${options.backoffBase}ms`);

  const sourceKeypair = Keypair.fromSecret(options.sourceSecret);
  let retried = 0;
  let succeeded = 0;
  let permanentlyFailed = 0;

  for (const entry of failedEntries) {
    if (entry.retryCount >= options.maxRetries) {
      logger.warn(`Entry #${entry.index}: Max retries (${options.maxRetries}) exceeded. Skipping.`);
      db.updateEntryStatus(options.batchId, entry.index, BatchEntryStatus.Skipped);
      permanentlyFailed++;
      continue;
    }

    const backoff = Math.min(
      options.backoffBase * Math.pow(2, entry.retryCount),
      options.backoffMax
    );

    logger.info(`Retrying entry #${entry.index} (attempt ${entry.retryCount + 1}/${options.maxRetries}) after ${backoff}ms...`);
    await sleep(backoff);

    try {
      const result = await retryEntry(entry, sourceKeypair, options);

      if (result.success) {
        db.updateEntryStatus(options.batchId, entry.index, BatchEntryStatus.Confirmed, result.hash);
        succeeded++;
        logger.success(`Entry #${entry.index}: Success (${result.hash})`);
      } else {
        db.updateEntryStatus(options.batchId, entry.index, BatchEntryStatus.Failed, '', result.error);
        logger.warn(`Entry #${entry.index}: Failed again - ${result.error}`);
      }
    } catch (err) {
      const errorMsg = err instanceof Error ? err.message : String(err);
      db.updateEntryStatus(options.batchId, entry.index, BatchEntryStatus.Failed, '', errorMsg);
      logger.error(`Entry #${entry.index}: Error - ${errorMsg}`);
    }

    retried++;
    logger.progress(retried, failedEntries.length, 'entries retried');
  }

  // Update batch counters
  db.updateBatchCounters(options.batchId);

  // Summary
  logger.banner('Retry Summary');
  logger.table(
    ['Metric', 'Value'],
    [
      ['Total Retried', String(retried)],
      ['Succeeded', String(succeeded)],
      ['Still Failed', String(retried - succeeded - permanentlyFailed)],
      ['Permanently Failed', String(permanentlyFailed)],
    ]
  );

  if (retried - succeeded > 0) {
    logger.info(`\nRetry again with: stellar-payout retry --batch-id=${options.batchId}`);
  }

  db.close();
}

async function retryEntry(
  entry: BatchPaymentEntry,
  sourceKeypair: Keypair,
  options: RetryOptions
): Promise<{ success: boolean; hash: string; error: string }> {
  try {
    // Fetch source account
    const accountResponse = await axios.get(
      `${options.horizonUrl}/accounts/${sourceKeypair.publicKey()}`,
      { timeout: 15000 }
    );
    const sourceAccount = new Account(sourceKeypair.publicKey(), accountResponse.data.sequence);

    const asset = entry.asset === 'XLM' || entry.asset === 'native'
      ? Asset.native()
      : new Asset(entry.asset, entry.asset_issuer || sourceKeypair.publicKey());

    const builder = new TransactionBuilder(sourceAccount, {
      fee: '10000', // Higher fee for retries (fee bump)
      networkPassphrase: options.networkPassphrase,
    }).setTimeout(0);

    if (entry.memo) {
      builder.addMemo(Memo.text(entry.memo.substring(0, 28)));
    }

    builder.addOperation(
      Operation.payment({
        destination: entry.destination,
        asset,
        amount: entry.amount,
      })
    );

    const transaction = builder.build();
    transaction.sign(sourceKeypair);

    const response = await axios.post(
      `${options.horizonUrl}/transactions`,
      `tx=${encodeURIComponent(transaction.toXDR())}`,
      {
        headers: { 'Content-Type': 'application/x-www-form-urlencoded' },
        timeout: 30000,
      }
    );

    return {
      success: response.data.successful !== false,
      hash: response.data.hash || '',
      error: '',
    };
  } catch (err: unknown) {
    const axiosErr = err as { response?: { data?: Record<string, unknown> }; message?: string };
    const errorMsg = axiosErr.response?.data
      ? JSON.stringify(axiosErr.response.data)
      : (axiosErr.message || 'Unknown error');
    return { success: false, hash: '', error: errorMsg };
  }
}

function sleep(ms: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, ms));
}
