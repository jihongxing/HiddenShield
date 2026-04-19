use rusqlite::{params, Connection};

use crate::commands::vault::VaultRecord;
use crate::db::schema;

/// Initialize the database by running all pending migrations.
pub fn init_db(conn: &Connection) -> Result<(), rusqlite::Error> {
  schema::run_migrations(conn)
}

/// Insert a single VaultRecord into the database.
pub fn insert_record(conn: &Connection, record: &VaultRecord) -> Result<(), rusqlite::Error> {
  conn.execute(
    "INSERT INTO vault_records (
      original_hash, file_name, created_at, duration_secs, resolution,
      watermark_uid, thumbnail_path, output_douyin, output_bilibili,
      output_xhs, is_hdr_source, hw_encoder_used, process_time_ms,
      tsa_token_path, network_time, tsa_source
    ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16)",
    params![
      record.original_hash,
      record.file_name,
      record.created_at,
      record.duration_secs,
      record.resolution,
      record.watermark_uid,
      record.thumbnail_path,
      record.output_douyin,
      record.output_bilibili,
      record.output_xhs,
      record.is_hdr_source as i32,
      record.hw_encoder_used,
      record.process_time_ms.map(|v| v as i64),
      record.tsa_token_path,
      record.network_time,
      record.tsa_source,
    ],
  )?;
  Ok(())
}

/// Query all vault records ordered by created_at descending.
pub fn list_records(conn: &Connection) -> Vec<VaultRecord> {
  let mut stmt = match conn.prepare(
    "SELECT id, original_hash, file_name, created_at, duration_secs,
            resolution, watermark_uid, thumbnail_path, output_douyin,
            output_bilibili, output_xhs, is_hdr_source, hw_encoder_used,
            process_time_ms, tsa_token_path, network_time, tsa_source
     FROM vault_records ORDER BY created_at DESC",
  ) {
    Ok(s) => s,
    Err(_) => return Vec::new(),
  };

  let rows = stmt.query_map([], |row| {
    Ok(VaultRecord {
      id: row.get::<_, u32>(0)?,
      original_hash: row.get(1)?,
      file_name: row.get(2)?,
      created_at: row.get(3)?,
      duration_secs: row.get(4)?,
      resolution: row.get(5)?,
      watermark_uid: row.get(6)?,
      thumbnail_path: row.get(7)?,
      output_douyin: row.get(8)?,
      output_bilibili: row.get(9)?,
      output_xhs: row.get(10)?,
      is_hdr_source: row.get::<_, i32>(11)? != 0,
      hw_encoder_used: row.get(12)?,
      process_time_ms: row.get::<_, Option<i64>>(13)?.map(|v| v as u64),
      tsa_token_path: row.get(14)?,
      network_time: row.get(15)?,
      tsa_source: row.get(16)?,
    })
  });

  match rows {
    Ok(mapped) => mapped.filter_map(|r| r.ok()).collect(),
    Err(_) => Vec::new(),
  }
}

/// Find a single record by watermark_uid (returns first match).
#[allow(dead_code)]
pub fn find_by_watermark_uid(conn: &Connection, uid: &str) -> Option<VaultRecord> {
  conn
    .query_row(
      "SELECT id, original_hash, file_name, created_at, duration_secs,
              resolution, watermark_uid, thumbnail_path, output_douyin,
              output_bilibili, output_xhs, is_hdr_source, hw_encoder_used,
              process_time_ms, tsa_token_path, network_time, tsa_source
       FROM vault_records WHERE watermark_uid = ?1",
      params![uid],
      |row| {
        Ok(VaultRecord {
          id: row.get::<_, u32>(0)?,
          original_hash: row.get(1)?,
          file_name: row.get(2)?,
          created_at: row.get(3)?,
          duration_secs: row.get(4)?,
          resolution: row.get(5)?,
          watermark_uid: row.get(6)?,
          thumbnail_path: row.get(7)?,
          output_douyin: row.get(8)?,
          output_bilibili: row.get(9)?,
          output_xhs: row.get(10)?,
          is_hdr_source: row.get::<_, i32>(11)? != 0,
          hw_encoder_used: row.get(12)?,
          process_time_ms: row.get::<_, Option<i64>>(13)?.map(|v| v as u64),
          tsa_token_path: row.get(14)?,
          network_time: row.get(15)?,
          tsa_source: row.get(16)?,
        })
      },
    )
    .ok()
}

/// Find records by watermark_uid, then narrow down by file hash prefix.
/// Returns the best match: first tries exact hash prefix match, falls back to first record.
pub fn find_by_uid_and_hash(conn: &Connection, uid: &str, file_hash_prefix: &[u8; 4]) -> Option<VaultRecord> {
  let mut stmt = conn.prepare(
    "SELECT id, original_hash, file_name, created_at, duration_secs,
            resolution, watermark_uid, thumbnail_path, output_douyin,
            output_bilibili, output_xhs, is_hdr_source, hw_encoder_used,
            process_time_ms, tsa_token_path, network_time, tsa_source
     FROM vault_records WHERE watermark_uid = ?1 ORDER BY created_at DESC",
  ).ok()?;

  let records: Vec<VaultRecord> = stmt.query_map(params![uid], |row| {
    Ok(VaultRecord {
      id: row.get::<_, u32>(0)?,
      original_hash: row.get(1)?,
      file_name: row.get(2)?,
      created_at: row.get(3)?,
      duration_secs: row.get(4)?,
      resolution: row.get(5)?,
      watermark_uid: row.get(6)?,
      thumbnail_path: row.get(7)?,
      output_douyin: row.get(8)?,
      output_bilibili: row.get(9)?,
      output_xhs: row.get(10)?,
      is_hdr_source: row.get::<_, i32>(11)? != 0,
      hw_encoder_used: row.get(12)?,
      process_time_ms: row.get::<_, Option<i64>>(13)?.map(|v| v as u64),
      tsa_token_path: row.get(14)?,
      network_time: row.get(15)?,
      tsa_source: row.get(16)?,
    })
  }).ok()?
    .filter_map(|r| r.ok())
    .collect();

  if records.is_empty() {
    return None;
  }

  // Try to match by file hash prefix (first 4 bytes of SHA-256)
  let prefix_hex = hex::encode(file_hash_prefix);
  for record in &records {
    if record.original_hash.starts_with(&prefix_hex) {
      return Some(record.clone());
    }
  }

  // Fallback: return the most recent record with this UID
  Some(records.into_iter().next().unwrap())
}
