use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalBatchJob {
    pub id: String,
    pub status: String,
    pub created_at: String,
    pub updated_at: String,
    pub entitlement_plan_code: String,
    pub entitlement_status: String,
    pub items: Vec<LocalBatchItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalBatchItem {
    pub id: String,
    pub job_id: String,
    pub input_ref: String,
    pub file_name: String,
    pub media_kind: String,
    pub status: String,
    pub attempts: u32,
    pub last_error: Option<String>,
    pub output_ref: Option<String>,
    pub vault_record_id: Option<i64>,
    pub write_verification_status: Option<String>,
    pub write_verification_message: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

pub fn list_local_batch_jobs(conn: &Connection) -> Result<Vec<LocalBatchJob>, rusqlite::Error> {
    let mut stmt = conn.prepare(
        "SELECT id, status, created_at, updated_at, entitlement_plan_code, entitlement_status
         FROM local_batch_jobs
         ORDER BY updated_at DESC",
    )?;
    let mut jobs = stmt
        .query_map([], |row| {
            Ok(LocalBatchJob {
                id: row.get(0)?,
                status: row.get(1)?,
                created_at: row.get(2)?,
                updated_at: row.get(3)?,
                entitlement_plan_code: row.get(4)?,
                entitlement_status: row.get(5)?,
                items: Vec::new(),
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    for job in &mut jobs {
        job.items = list_items_for_job(conn, &job.id)?;
    }
    Ok(jobs)
}

pub fn save_local_batch_job(
    conn: &mut Connection,
    job: &LocalBatchJob,
) -> Result<(), rusqlite::Error> {
    let tx = conn.transaction()?;
    tx.execute(
        "INSERT INTO local_batch_jobs (
            id, status, created_at, updated_at, entitlement_plan_code, entitlement_status
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)
         ON CONFLICT(id) DO UPDATE SET
            status = excluded.status,
            updated_at = excluded.updated_at,
            entitlement_plan_code = excluded.entitlement_plan_code,
            entitlement_status = excluded.entitlement_status",
        params![
            &job.id,
            &job.status,
            &job.created_at,
            &job.updated_at,
            &job.entitlement_plan_code,
            &job.entitlement_status,
        ],
    )?;
    tx.execute(
        "DELETE FROM local_batch_items WHERE job_id = ?1",
        params![job.id],
    )?;
    for item in &job.items {
        tx.execute(
            "INSERT INTO local_batch_items (
                id, job_id, input_ref, file_name, media_kind, status, attempts,
                last_error, output_ref, vault_record_id, write_verification_status,
                write_verification_message, created_at, updated_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
            params![
                &item.id,
                &job.id,
                &item.input_ref,
                &item.file_name,
                &item.media_kind,
                &item.status,
                item.attempts as i64,
                &item.last_error,
                &item.output_ref,
                item.vault_record_id,
                &item.write_verification_status,
                &item.write_verification_message,
                &item.created_at,
                &item.updated_at,
            ],
        )?;
    }
    tx.commit()
}

fn list_items_for_job(
    conn: &Connection,
    job_id: &str,
) -> Result<Vec<LocalBatchItem>, rusqlite::Error> {
    let mut stmt = conn.prepare(
        "SELECT id, job_id, input_ref, file_name, media_kind, status, attempts,
                last_error, output_ref, vault_record_id, write_verification_status,
                write_verification_message, created_at, updated_at
         FROM local_batch_items
         WHERE job_id = ?1
         ORDER BY updated_at ASC",
    )?;
    let items = stmt
        .query_map(params![job_id], |row| {
            Ok(LocalBatchItem {
                id: row.get(0)?,
                job_id: row.get(1)?,
                input_ref: row.get(2)?,
                file_name: row.get(3)?,
                media_kind: row.get(4)?,
                status: row.get(5)?,
                attempts: row.get::<_, i64>(6)? as u32,
                last_error: row.get(7)?,
                output_ref: row.get(8)?,
                vault_record_id: row.get(9)?,
                write_verification_status: row.get(10)?,
                write_verification_message: row.get(11)?,
                created_at: row.get(12)?,
                updated_at: row.get(13)?,
            })
        })?
        .collect();
    items
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::schema;

    #[test]
    fn local_batch_job_round_trips_with_items() {
        let mut conn = Connection::open_in_memory().unwrap();
        schema::run_migrations(&conn).unwrap();

        let job = LocalBatchJob {
            id: "batch-1".to_string(),
            status: "queued".to_string(),
            created_at: "2026-06-18T00:00:00Z".to_string(),
            updated_at: "2026-06-18T00:00:01Z".to_string(),
            entitlement_plan_code: "creator".to_string(),
            entitlement_status: "active".to_string(),
            items: vec![LocalBatchItem {
                id: "item-1".to_string(),
                job_id: "batch-1".to_string(),
                input_ref: "cover.png".to_string(),
                file_name: "cover.png".to_string(),
                media_kind: "image".to_string(),
                status: "queued".to_string(),
                attempts: 0,
                last_error: None,
                output_ref: None,
                vault_record_id: None,
                write_verification_status: None,
                write_verification_message: None,
                created_at: "2026-06-18T00:00:00Z".to_string(),
                updated_at: "2026-06-18T00:00:00Z".to_string(),
            }],
        };

        save_local_batch_job(&mut conn, &job).unwrap();
        let loaded = list_local_batch_jobs(&conn).unwrap();

        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].id, "batch-1");
        assert_eq!(loaded[0].items.len(), 1);
        assert_eq!(loaded[0].items[0].file_name, "cover.png");
    }

    #[test]
    fn local_batch_job_replace_updates_items() {
        let mut conn = Connection::open_in_memory().unwrap();
        schema::run_migrations(&conn).unwrap();

        let mut job = LocalBatchJob {
            id: "batch-1".to_string(),
            status: "queued".to_string(),
            created_at: "2026-06-18T00:00:00Z".to_string(),
            updated_at: "2026-06-18T00:00:01Z".to_string(),
            entitlement_plan_code: "creator".to_string(),
            entitlement_status: "active".to_string(),
            items: vec![],
        };
        save_local_batch_job(&mut conn, &job).unwrap();

        job.status = "paused".to_string();
        job.items.push(LocalBatchItem {
            id: "item-2".to_string(),
            job_id: "batch-1".to_string(),
            input_ref: "song.wav".to_string(),
            file_name: "song.wav".to_string(),
            media_kind: "audio".to_string(),
            status: "failed".to_string(),
            attempts: 1,
            last_error: Some("test failure".to_string()),
            output_ref: None,
            vault_record_id: None,
            write_verification_status: None,
            write_verification_message: None,
            created_at: "2026-06-18T00:00:00Z".to_string(),
            updated_at: "2026-06-18T00:00:02Z".to_string(),
        });
        save_local_batch_job(&mut conn, &job).unwrap();

        let loaded = list_local_batch_jobs(&conn).unwrap();
        assert_eq!(loaded[0].status, "paused");
        assert_eq!(loaded[0].items.len(), 1);
        assert_eq!(
            loaded[0].items[0].last_error.as_deref(),
            Some("test failure")
        );
    }
}
