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
      tsa_token_path, network_time, tsa_source, tsa_request_nonce
    ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17)",
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
            record.tsa_request_nonce,
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
            process_time_ms, tsa_token_path, network_time, tsa_source, tsa_request_nonce
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
            tsa_request_nonce: row.get(17)?,
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
    conn.query_row(
        "SELECT id, original_hash, file_name, created_at, duration_secs,
              resolution, watermark_uid, thumbnail_path, output_douyin,
              output_bilibili, output_xhs, is_hdr_source, hw_encoder_used,
              process_time_ms, tsa_token_path, network_time, tsa_source, tsa_request_nonce
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
                tsa_request_nonce: row.get(17)?,
            })
        },
    )
    .ok()
}

/// Find a record by watermark_uid and exact file hash prefix.
/// Returns `None` if the asset-binding hash prefix does not match.
pub fn find_by_uid_and_hash(
    conn: &Connection,
    uid: &str,
    file_hash_prefix: &[u8; 4],
) -> Option<VaultRecord> {
    let mut stmt = conn
        .prepare(
            "SELECT id, original_hash, file_name, created_at, duration_secs,
            resolution, watermark_uid, thumbnail_path, output_douyin,
            output_bilibili, output_xhs, is_hdr_source, hw_encoder_used,
            process_time_ms, tsa_token_path, network_time, tsa_source, tsa_request_nonce
     FROM vault_records WHERE watermark_uid = ?1 ORDER BY created_at DESC",
        )
        .ok()?;

    let records: Vec<VaultRecord> = stmt
        .query_map(params![uid], |row| {
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
                tsa_request_nonce: row.get(17)?,
            })
        })
        .ok()?
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

    None
}

/// Whether any vault record exists for the given watermark UID.
pub fn has_watermark_uid(conn: &Connection, uid: &str) -> bool {
    conn.query_row(
        "SELECT 1 FROM vault_records WHERE watermark_uid = ?1 LIMIT 1",
        params![uid],
        |_| Ok(()),
    )
    .is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::schema;

    fn sample_record(hash: &str, uid: &str, created_at: &str) -> VaultRecord {
        VaultRecord {
            id: 0,
            original_hash: hash.to_string(),
            file_name: "sample.mp4".to_string(),
            created_at: created_at.to_string(),
            duration_secs: 1.0,
            resolution: "1920x1080".to_string(),
            watermark_uid: uid.to_string(),
            thumbnail_path: None,
            output_douyin: None,
            output_bilibili: None,
            output_xhs: None,
            is_hdr_source: false,
            hw_encoder_used: None,
            process_time_ms: None,
            tsa_token_path: None,
            network_time: None,
            tsa_source: None,
            tsa_request_nonce: None,
        }
    }

    #[test]
    fn exact_hash_prefix_is_required_for_match() {
        let conn = Connection::open_in_memory().unwrap();
        schema::run_migrations(&conn).unwrap();

        let uid = "HS-ABCD-EF01-2345";
        insert_record(
            &conn,
            &sample_record("11223344deadbeef", uid, "2026-04-20T10:00:00Z"),
        )
        .unwrap();
        insert_record(
            &conn,
            &sample_record("55667788cafebabe", uid, "2026-04-21T10:00:00Z"),
        )
        .unwrap();

        assert!(has_watermark_uid(&conn, uid));
        assert!(find_by_uid_and_hash(&conn, uid, &[0x11, 0x22, 0x33, 0x44]).is_some());
        assert!(find_by_uid_and_hash(&conn, uid, &[0xaa, 0xbb, 0xcc, 0xdd]).is_none());
    }
}
