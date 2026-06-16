import { existsSync } from 'node:fs';
import { spawnSync } from 'node:child_process';

const endpoint = process.env.HIDDENSHIELD_SYNC_URL ?? 'http://127.0.0.1:47219';
const pairingCode = process.env.HIDDENSHIELD_PAIRING_CODE?.trim();
const dbPath = process.env.HIDDENSHIELD_VAULT_DB;
const eventKind = process.env.HIDDENSHIELD_SYNC_EVENT ?? 'vault';
const queueId = `script-${Date.now()}`;

if (!pairingCode) {
  console.error(
    'Set HIDDENSHIELD_PAIRING_CODE to the code shown in the desktop settings page.',
  );
  process.exit(1);
}

if (!['vault', 'evidence', 'batch'].includes(eventKind)) {
  console.error('HIDDENSHIELD_SYNC_EVENT must be "vault", "evidence", or "batch".');
  process.exit(1);
}

const recordId = `record-${Date.now()}`;
const item =
  eventKind === 'evidence'
    ? {
        queueId,
        recordId,
        operation: 'upsertEvidenceRecord',
        payloadType: 'evidence_record',
        payload: {
          id: recordId,
          kind: 'image',
          title: 'script-evidence-check.png',
          watermark_uid: 'script-evidence-check',
          revision: 1,
          extracted_timestamp: Math.floor(Date.now() / 1000),
          extracted_device_id_hex: '090a0b0c',
          extracted_file_hash_hex: 'abcd',
          source: 'verify',
          sync_status: 'pending',
          created_at: new Date().toISOString(),
        },
      }
    : {
        queueId,
        recordId,
        operation: 'upsertVaultRecord',
        payloadType: 'vault_record',
        payload: {
          id: recordId,
          kind: 'image',
          title: 'script-sync-check.png',
          watermark_uid: 'script-sync-check',
          revision: 1,
          source: 'write',
          sync_status: 'pending',
          created_at: new Date().toISOString(),
        },
      };
const payload =
  eventKind === 'batch'
    ? {
        items: [
          item,
          {
            queueId: `${queueId}-evidence`,
            recordId: `${recordId}-evidence`,
            operation: 'upsertEvidenceRecord',
            payloadType: 'evidence_record',
            payload: {
              id: `${recordId}-evidence`,
              kind: 'image',
              title: 'script-evidence-check.png',
              watermark_uid: 'script-sync-check',
              revision: 1,
              extracted_timestamp: Math.floor(Date.now() / 1000),
              extracted_device_id_hex: '090a0b0c',
              extracted_file_hash_hex: 'abcd',
              source: 'verify',
              sync_status: 'pending',
              created_at: new Date().toISOString(),
            },
          },
        ],
      }
    : { items: [item] };

const response = await fetch(`${endpoint}/api/mobile-sync/v1/queue-batch`, {
  method: 'POST',
  headers: {
    'content-type': 'application/json',
    'x-hiddenshield-pairing-code': pairingCode,
  },
  body: JSON.stringify(payload),
});

const body = await response.text();
console.log(`POST ${response.status}: ${body}`);

if (!response.ok) {
  process.exit(1);
}

if (!dbPath) {
  console.log('Set HIDDENSHIELD_VAULT_DB to also verify SQLite persistence.');
  process.exit(0);
}

if (!existsSync(dbPath)) {
  console.error(`Database not found: ${dbPath}`);
  process.exit(1);
}

const escapedQueueId = queueId.replaceAll("'", "''");
const escapedRecordId = recordId.replaceAll("'", "''");
const escapedBatchEvidenceQueueId = `${queueId}-evidence`.replaceAll("'", "''");
const escapedBatchEvidenceRecordId = `${recordId}-evidence`.replaceAll("'", "''");
const query =
  eventKind === 'batch'
    ? `SELECT e.queue_id, e.operation, r.mobile_record_id FROM sync_events e JOIN sync_evidence_records r ON r.mobile_record_id = '${escapedBatchEvidenceRecordId}' WHERE e.queue_id = '${escapedBatchEvidenceQueueId}' LIMIT 1;`
    : eventKind === 'evidence'
    ? `SELECT e.queue_id, e.operation, r.mobile_record_id FROM sync_events e JOIN sync_evidence_records r ON r.mobile_record_id = '${escapedRecordId}' WHERE e.queue_id = '${escapedQueueId}' LIMIT 1;`
    : `SELECT e.queue_id, e.operation, r.watermark_uid FROM sync_events e JOIN vault_records r ON r.watermark_uid = 'script-sync-check' WHERE e.queue_id = '${escapedQueueId}' LIMIT 1;`;
const sqlite = spawnSync('sqlite3', [dbPath, query], {
  encoding: 'utf8',
  shell: false,
});

if (sqlite.error) {
  console.error(`sqlite3 unavailable: ${sqlite.error.message}`);
  console.log(`Queue id to check manually: ${queueId}`);
  process.exit(0);
}

if (sqlite.status !== 0) {
  console.error(sqlite.stderr.trim());
  process.exit(sqlite.status ?? 1);
}

const output = sqlite.stdout.trim();
if (!output) {
  console.error(`sync_events row not found for ${queueId}`);
  process.exit(1);
}

console.log(`SQLite row: ${output}`);
