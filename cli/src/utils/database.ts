import Database from 'better-sqlite3';
import {
  BatchState,
  BatchStatus,
  BatchPaymentEntry,
  BatchEntryStatus,
  TransactionGroup,
  NetworkType,
} from '../types';

export class BatchDatabase {
  private db: Database.Database;

  constructor(dbPath: string) {
    this.db = new Database(dbPath);
    this.db.pragma('journal_mode = WAL');
    this.db.pragma('synchronous = NORMAL');
    this.initialize();
  }

  private initialize(): void {
    this.db.exec(`
      CREATE TABLE IF NOT EXISTS batches (
        batch_id TEXT PRIMARY KEY,
        total_payments INTEGER NOT NULL DEFAULT 0,
        processed_payments INTEGER NOT NULL DEFAULT 0,
        successful_payments INTEGER NOT NULL DEFAULT 0,
        failed_payments INTEGER NOT NULL DEFAULT 0,
        skipped_payments INTEGER NOT NULL DEFAULT 0,
        started_at INTEGER NOT NULL,
        completed_at INTEGER,
        status TEXT NOT NULL DEFAULT 'created',
        source_account TEXT NOT NULL DEFAULT '',
        network TEXT NOT NULL DEFAULT 'testnet',
        dry_run INTEGER NOT NULL DEFAULT 0
      );

      CREATE TABLE IF NOT EXISTS payment_entries (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        batch_id TEXT NOT NULL,
        entry_index INTEGER NOT NULL,
        destination TEXT NOT NULL,
        amount TEXT NOT NULL,
        asset TEXT NOT NULL,
        asset_issuer TEXT NOT NULL DEFAULT '',
        memo TEXT NOT NULL DEFAULT '',
        escrow_duration INTEGER NOT NULL DEFAULT 0,
        status TEXT NOT NULL DEFAULT 'pending',
        tx_hash TEXT NOT NULL DEFAULT '',
        error TEXT NOT NULL DEFAULT '',
        retry_count INTEGER NOT NULL DEFAULT 0,
        submitted_at INTEGER NOT NULL DEFAULT 0,
        completed_at INTEGER NOT NULL DEFAULT 0,
        batch_group INTEGER NOT NULL DEFAULT 0,
        FOREIGN KEY (batch_id) REFERENCES batches(batch_id)
      );

      CREATE TABLE IF NOT EXISTS transaction_groups (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        batch_id TEXT NOT NULL,
        group_index INTEGER NOT NULL,
        tx_hash TEXT NOT NULL DEFAULT '',
        status TEXT NOT NULL DEFAULT 'pending',
        fee TEXT NOT NULL DEFAULT '0',
        submitted_at INTEGER NOT NULL DEFAULT 0,
        confirmed_at INTEGER NOT NULL DEFAULT 0,
        FOREIGN KEY (batch_id) REFERENCES batches(batch_id)
      );

      CREATE INDEX IF NOT EXISTS idx_entries_batch ON payment_entries(batch_id);
      CREATE INDEX IF NOT EXISTS idx_entries_status ON payment_entries(status);
      CREATE INDEX IF NOT EXISTS idx_groups_batch ON transaction_groups(batch_id);
    `);
  }

  createBatch(batchId: string, totalPayments: number, sourceAccount: string, network: NetworkType, dryRun: boolean): void {
    this.db.prepare(`
      INSERT INTO batches (batch_id, total_payments, started_at, status, source_account, network, dry_run)
      VALUES (?, ?, ?, 'created', ?, ?, ?)
    `).run(batchId, totalPayments, Date.now(), sourceAccount, network, dryRun ? 1 : 0);
  }

  updateBatchStatus(batchId: string, status: BatchStatus): void {
    const updates: Record<string, unknown> = { status };
    if (status === BatchStatus.Completed || status === BatchStatus.Failed || status === BatchStatus.Cancelled) {
      updates.completed_at = Date.now();
    }
    this.db.prepare(`
      UPDATE batches SET status = ?, completed_at = ? WHERE batch_id = ?
    `).run(status, updates.completed_at || null, batchId);
  }

  updateBatchCounters(batchId: string): void {
    const stats = this.db.prepare(`
      SELECT
        COUNT(*) as total,
        SUM(CASE WHEN status IN ('submitted','confirmed') THEN 1 ELSE 0 END) as processed,
        SUM(CASE WHEN status = 'confirmed' THEN 1 ELSE 0 END) as successful,
        SUM(CASE WHEN status = 'failed' THEN 1 ELSE 0 END) as failed,
        SUM(CASE WHEN status = 'skipped' THEN 1 ELSE 0 END) as skipped
      FROM payment_entries WHERE batch_id = ?
    `).get(batchId) as Record<string, number>;

    this.db.prepare(`
      UPDATE batches SET
        processed_payments = ?,
        successful_payments = ?,
        failed_payments = ?,
        skipped_payments = ?
      WHERE batch_id = ?
    `).run(stats.processed, stats.successful, stats.failed, stats.skipped, batchId);
  }

  getBatch(batchId: string): BatchState | null {
    const row = this.db.prepare('SELECT * FROM batches WHERE batch_id = ?').get(batchId) as Record<string, unknown> | undefined;
    if (!row) return null;
    return {
      batchId: row.batch_id as string,
      totalPayments: row.total_payments as number,
      processedPayments: row.processed_payments as number,
      successfulPayments: row.successful_payments as number,
      failedPayments: row.failed_payments as number,
      skippedPayments: row.skipped_payments as number,
      startedAt: row.started_at as number,
      completedAt: row.completed_at as number | null,
      status: row.status as BatchStatus,
      sourceAccount: row.source_account as string,
      network: row.network as NetworkType,
      dryRun: (row.dry_run as number) === 1,
    };
  }

  getRecentBatches(limit: number = 10): BatchState[] {
    const rows = this.db.prepare('SELECT * FROM batches ORDER BY started_at DESC LIMIT ?').all(limit) as Record<string, unknown>[];
    return rows.map((row) => ({
      batchId: row.batch_id as string,
      totalPayments: row.total_payments as number,
      processedPayments: row.processed_payments as number,
      successfulPayments: row.successful_payments as number,
      failedPayments: row.failed_payments as number,
      skippedPayments: row.skipped_payments as number,
      startedAt: row.started_at as number,
      completedAt: row.completed_at as number | null,
      status: row.status as BatchStatus,
      sourceAccount: row.source_account as string,
      network: row.network as NetworkType,
      dryRun: (row.dry_run as number) === 1,
    }));
  }

  insertEntry(batchId: string, entry: BatchPaymentEntry): void {
    this.db.prepare(`
      INSERT INTO payment_entries (batch_id, entry_index, destination, amount, asset, asset_issuer, memo, escrow_duration, status, batch_group)
      VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
    `).run(batchId, entry.index, entry.destination, entry.amount, entry.asset, entry.asset_issuer, entry.memo, entry.escrow_duration, entry.status, entry.batchGroup);
  }

  insertEntries(batchId: string, entries: BatchPaymentEntry[]): void {
    const insert = this.db.prepare(`
      INSERT INTO payment_entries (batch_id, entry_index, destination, amount, asset, asset_issuer, memo, escrow_duration, status, batch_group)
      VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
    `);
    const transaction = this.db.transaction((items: BatchPaymentEntry[]) => {
      for (const entry of items) {
        insert.run(batchId, entry.index, entry.destination, entry.amount, entry.asset, entry.asset_issuer, entry.memo, entry.escrow_duration, entry.status, entry.batchGroup);
      }
    });
    transaction(entries);
  }

  updateEntryStatus(batchId: string, index: number, status: BatchEntryStatus, txHash?: string, errorMsg?: string): void {
    const now = Date.now();
    if (status === BatchEntryStatus.Submitted) {
      this.db.prepare(`
        UPDATE payment_entries SET status = ?, tx_hash = ?, submitted_at = ? WHERE batch_id = ? AND entry_index = ?
      `).run(status, txHash || '', now, batchId, index);
    } else if (status === BatchEntryStatus.Confirmed) {
      this.db.prepare(`
        UPDATE payment_entries SET status = ?, tx_hash = ?, completed_at = ? WHERE batch_id = ? AND entry_index = ?
      `).run(status, txHash || '', now, batchId, index);
    } else if (status === BatchEntryStatus.Failed) {
      this.db.prepare(`
        UPDATE payment_entries SET status = ?, error = ?, retry_count = retry_count + 1 WHERE batch_id = ? AND entry_index = ?
      `).run(status, errorMsg || '', batchId, index);
    } else {
      this.db.prepare(`
        UPDATE payment_entries SET status = ? WHERE batch_id = ? AND entry_index = ?
      `).run(status, batchId, index);
    }
  }

  getEntries(batchId: string): BatchPaymentEntry[] {
    const rows = this.db.prepare('SELECT * FROM payment_entries WHERE batch_id = ? ORDER BY entry_index').all(batchId) as Record<string, unknown>[];
    return rows.map((row) => ({
      index: row.entry_index as number,
      destination: row.destination as string,
      amount: row.amount as string,
      asset: row.asset as string,
      asset_issuer: (row.asset_issuer as string) || '',
      memo: row.memo as string,
      escrow_duration: row.escrow_duration as number,
      status: row.status as BatchEntryStatus,
      txHash: row.tx_hash as string,
      error: row.error as string,
      retryCount: row.retry_count as number,
      submittedAt: row.submitted_at as number,
      completedAt: row.completed_at as number,
      batchGroup: row.batch_group as number,
    }));
  }

  getFailedEntries(batchId: string): BatchPaymentEntry[] {
    const rows = this.db.prepare(
      "SELECT * FROM payment_entries WHERE batch_id = ? AND status = 'failed' ORDER BY entry_index"
    ).all(batchId) as Record<string, unknown>[];
    return rows.map((row) => ({
      index: row.entry_index as number,
      destination: row.destination as string,
      amount: row.amount as string,
      asset: row.asset as string,
      asset_issuer: (row.asset_issuer as string) || '',
      memo: row.memo as string,
      escrow_duration: row.escrow_duration as number,
      status: row.status as BatchEntryStatus,
      txHash: row.tx_hash as string,
      error: row.error as string,
      retryCount: row.retry_count as number,
      submittedAt: row.submitted_at as number,
      completedAt: row.completed_at as number,
      batchGroup: row.batch_group as number,
    }));
  }

  insertGroup(batchId: string, group: TransactionGroup): void {
    this.db.prepare(`
      INSERT INTO transaction_groups (batch_id, group_index, tx_hash, status, fee, submitted_at, confirmed_at)
      VALUES (?, ?, ?, ?, ?, ?, ?)
    `).run(batchId, group.groupIndex, group.txHash, group.status, group.fee, group.submittedAt, group.confirmedAt);
  }

  updateGroupStatus(batchId: string, groupIndex: number, status: BatchEntryStatus, txHash?: string): void {
    const now = Date.now();
    this.db.prepare(`
      UPDATE transaction_groups SET status = ?, tx_hash = ?, confirmed_at = ? WHERE batch_id = ? AND group_index = ?
    `).run(status, txHash || '', now, batchId, groupIndex);
  }

  getGroups(batchId: string): TransactionGroup[] {
    const rows = this.db.prepare('SELECT * FROM transaction_groups WHERE batch_id = ? ORDER BY group_index').all(batchId) as Record<string, unknown>[];
    return rows.map((row) => ({
      groupIndex: row.group_index as number,
      entries: [],
      txHash: row.tx_hash as string,
      status: row.status as BatchEntryStatus,
      fee: row.fee as string,
      submittedAt: row.submitted_at as number,
      confirmedAt: row.confirmed_at as number,
    }));
  }

  close(): void {
    this.db.close();
  }
}
