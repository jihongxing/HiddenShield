use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use rusqlite::Connection;
use tauri::{AppHandle, Manager};

use crate::config;
use crate::db::queries;

use super::storage::{
    build_changes_response, build_error_response, build_health_response,
    build_queue_batch_response, build_queue_item_response, init_sync_storage, pairing_code_matches,
    record_sync_event, MobileSyncBatchRequest, MobileSyncQueueItem,
};
pub fn start_sync_server(app_handle: AppHandle) {
    let listen_port = config::load_system_config().lan_debug_port;
    let app_data_dir = match app_handle.path().app_data_dir() {
        Ok(path) => path,
        Err(err) => {
            log::warn!("sync server skipped: {err}");
            return;
        }
    };

    thread::spawn(move || {
        let db_path = app_data_dir.join("vault.db");
        if let Err(err) = run_sync_server(db_path, app_data_dir, ("0.0.0.0", listen_port), None) {
            log::warn!("sync server stopped: {err}");
        }
    });
}

#[cfg(test)]
fn start_test_sync_server(
    db_path: std::path::PathBuf,
    app_data_dir: std::path::PathBuf,
) -> Result<
    (
        std::net::SocketAddr,
        Arc<AtomicBool>,
        thread::JoinHandle<()>,
    ),
    String,
> {
    let stop = Arc::new(AtomicBool::new(false));
    let listener = TcpListener::bind(("127.0.0.1", 0)).map_err(|e| e.to_string())?;
    let addr = listener.local_addr().map_err(|e| e.to_string())?;
    let stop_for_thread = Arc::clone(&stop);
    let handle = thread::spawn(move || {
        let _ =
            run_sync_server_with_listener(db_path, app_data_dir, listener, Some(stop_for_thread));
    });
    Ok((addr, stop, handle))
}

fn run_sync_server<A: std::net::ToSocketAddrs>(
    db_path: std::path::PathBuf,
    app_data_dir: std::path::PathBuf,
    addr: A,
    stop: Option<Arc<AtomicBool>>,
) -> Result<(), String> {
    let listener = TcpListener::bind(addr).map_err(|e| format!("bind failed: {e}"))?;
    run_sync_server_with_listener(db_path, app_data_dir, listener, stop)
}

fn run_sync_server_with_listener(
    db_path: std::path::PathBuf,
    app_data_dir: std::path::PathBuf,
    listener: TcpListener,
    stop: Option<Arc<AtomicBool>>,
) -> Result<(), String> {
    let conn = match Connection::open(&db_path) {
        Ok(conn) => conn,
        Err(err) => {
            return Err(format!("db open failed: {err}"));
        }
    };
    if let Err(err) = queries::init_db(&conn) {
        return Err(format!("db init failed: {err}"));
    }
    if let Err(err) = init_sync_storage(&conn) {
        return Err(format!("storage init failed: {err}"));
    }

    let conn = Arc::new(std::sync::Mutex::new(conn));

    let _ = listener.set_nonblocking(true);
    let addr = listener
        .local_addr()
        .map(|addr| addr.to_string())
        .unwrap_or_else(|_| "0.0.0.0:unknown".to_string());
    log::info!("desktop sync stub listening on http://{addr}");

    loop {
        if stop
            .as_ref()
            .map(|flag| flag.load(Ordering::SeqCst))
            .unwrap_or(false)
        {
            return Ok(());
        }
        match listener.accept() {
            Ok((stream, _)) => {
                let conn = Arc::clone(&conn);
                let app_data_dir = app_data_dir.clone();
                thread::spawn(move || handle_client(stream, conn, app_data_dir));
            }
            Err(err) if err.kind() == std::io::ErrorKind::WouldBlock => {
                thread::sleep(Duration::from_millis(100));
            }
            Err(err) => {
                log::warn!("sync server accept failed: {err}");
                thread::sleep(Duration::from_millis(200));
            }
        }
    }
}

fn handle_client(
    mut stream: TcpStream,
    conn: Arc<std::sync::Mutex<Connection>>,
    app_data_dir: std::path::PathBuf,
) {
    let _ = stream.set_read_timeout(Some(Duration::from_secs(3)));
    let request = match read_http_request(&mut stream) {
        Ok(text) => text,
        Err(error) => {
            let _ = write_response(&mut stream, 400, build_error_response(&error));
            return;
        }
    };

    let (method, path, headers, body) = match parse_http_request(&request) {
        Some(parsed) => parsed,
        None => {
            let _ = write_response(&mut stream, 400, build_error_response("invalid_request"));
            return;
        }
    };

    if method == "GET" && path == "/api/mobile-sync/v1/health" {
        let _ = write_response(&mut stream, 200, build_health_response());
        return;
    }

    if method == "GET" && path.starts_with("/api/mobile-sync/v1/changes") {
        let pairing_code = headers
            .get("x-hiddenshield-pairing-code")
            .map(String::as_str)
            .unwrap_or("");
        if pairing_code.trim().is_empty() {
            let _ = write_response(
                &mut stream,
                401,
                build_error_response("pairing_code_missing"),
            );
            return;
        }
        if !pairing_code_matches(&app_data_dir, pairing_code) {
            let _ = write_response(
                &mut stream,
                403,
                build_error_response("pairing_code_invalid"),
            );
            return;
        }

        let since = query_param(&path, "since");
        let changes = conn
            .lock()
            .ok()
            .and_then(|guard| build_changes_response(&guard, since.as_deref()).ok());
        let Some(changes) = changes else {
            let _ = write_response(
                &mut stream,
                500,
                build_error_response("changes_query_failed"),
            );
            return;
        };
        let _ = write_response(&mut stream, 200, changes);
        return;
    }

    if method == "POST"
        && (path == "/api/mobile-sync/v1/queue-item" || path == "/api/mobile-sync/v1/queue-batch")
    {
        let pairing_code = headers
            .get("x-hiddenshield-pairing-code")
            .map(String::as_str)
            .unwrap_or("");
        if pairing_code.trim().is_empty() {
            let _ = write_response(
                &mut stream,
                401,
                build_error_response("pairing_code_missing"),
            );
            return;
        }
        if !pairing_code_matches(&app_data_dir, pairing_code) {
            let _ = write_response(
                &mut stream,
                403,
                build_error_response("pairing_code_invalid"),
            );
            return;
        }

        if path == "/api/mobile-sync/v1/queue-batch" {
            let batch: MobileSyncBatchRequest = match serde_json::from_str(&body) {
                Ok(batch) => batch,
                Err(_) => {
                    let _ = write_response(&mut stream, 400, build_error_response("invalid_json"));
                    return;
                }
            };
            if batch.items.is_empty() {
                let _ = write_response(&mut stream, 400, build_error_response("empty_sync_batch"));
                return;
            }
            if batch.items.len() > 100 {
                let _ = write_response(
                    &mut stream,
                    400,
                    build_error_response("sync_batch_too_large"),
                );
                return;
            }

            let db_result = conn.lock().ok().and_then(|guard| {
                for item in &batch.items {
                    if record_sync_event(&guard, item).is_err() {
                        return None;
                    }
                }
                Some(())
            });
            if db_result.is_none() {
                let _ = write_response(
                    &mut stream,
                    500,
                    build_error_response("sync_batch_persist_failed"),
                );
                return;
            }

            let _ = write_response(&mut stream, 200, build_queue_batch_response(&batch.items));
            return;
        }

        let item: MobileSyncQueueItem = match serde_json::from_str(&body) {
            Ok(item) => item,
            Err(_) => {
                let _ = write_response(&mut stream, 400, build_error_response("invalid_json"));
                return;
            }
        };

        let db_result = conn
            .lock()
            .ok()
            .and_then(|guard| record_sync_event(&guard, &item).ok());
        if db_result.is_none() {
            let _ = write_response(
                &mut stream,
                500,
                build_error_response("sync_event_persist_failed"),
            );
            return;
        }

        let _ = write_response(&mut stream, 200, build_queue_item_response(&item.queue_id));
        return;
    }

    let _ = write_response(&mut stream, 404, build_error_response("not_found"));
}

fn read_http_request(stream: &mut TcpStream) -> Result<String, String> {
    let mut reader = BufReader::new(stream);
    let mut head = String::new();

    loop {
        let mut line = String::new();
        let bytes_read = reader
            .read_line(&mut line)
            .map_err(|_| "request_read_failed".to_string())?;
        if bytes_read == 0 {
            return Err("request_closed".to_string());
        }
        head.push_str(&line);
        if line == "\r\n" || line == "\n" {
            break;
        }
        if head.len() > 16 * 1024 {
            return Err("request_headers_too_large".to_string());
        }
    }

    let content_length = header_content_length(&head).unwrap_or(0);
    if content_length > 512 * 1024 {
        return Err("request_body_too_large".to_string());
    }

    let mut body = vec![0_u8; content_length];
    if content_length > 0 {
        reader
            .read_exact(&mut body)
            .map_err(|_| "request_body_read_failed".to_string())?;
    }

    let body = String::from_utf8(body).map_err(|_| "invalid_utf8".to_string())?;
    Ok(format!("{head}{body}"))
}

fn header_content_length(head: &str) -> Option<usize> {
    head.lines().find_map(|line| {
        let (name, value) = line.split_once(':')?;
        if name.trim().eq_ignore_ascii_case("content-length") {
            value.trim().parse::<usize>().ok()
        } else {
            None
        }
    })
}

fn parse_http_request(
    request: &str,
) -> Option<(
    String,
    String,
    std::collections::HashMap<String, String>,
    String,
)> {
    let mut parts = request.splitn(2, "\r\n\r\n");
    let head = parts.next()?;
    let body = parts.next().unwrap_or("").to_string();

    let mut lines = head.lines();
    let request_line = lines.next()?;
    let mut request_parts = request_line.split_whitespace();
    let method = request_parts.next()?.to_string();
    let path = request_parts.next()?.to_string();

    let mut headers = std::collections::HashMap::new();
    for line in lines {
        if let Some((name, value)) = line.split_once(':') {
            headers.insert(name.trim().to_lowercase(), value.trim().to_string());
        }
    }

    Some((method, path, headers, body))
}

fn query_param(path: &str, key: &str) -> Option<String> {
    let query = path.split_once('?')?.1;
    query.split('&').find_map(|pair| {
        let (name, value) = pair.split_once('=')?;
        if name == key {
            Some(value.to_string())
        } else {
            None
        }
    })
}

fn write_response(stream: &mut TcpStream, status_code: u16, body: String) -> std::io::Result<()> {
    let status_text = match status_code {
        200 => "OK",
        400 => "Bad Request",
        401 => "Unauthorized",
        403 => "Forbidden",
        404 => "Not Found",
        500 => "Internal Server Error",
        _ => "OK",
    };
    let response = format!(
        "HTTP/1.1 {status_code} {status_text}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        body.len(),
        body
    );
    stream.write_all(response.as_bytes())?;
    stream.flush()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};

    #[test]
    fn parses_content_length_case_insensitively() {
        let head = "POST / HTTP/1.1\r\nHost: localhost\r\nContent-Length: 42\r\n\r\n";
        assert_eq!(header_content_length(head), Some(42));

        let lower = "POST / HTTP/1.1\r\ncontent-length: 7\r\n\r\n";
        assert_eq!(header_content_length(lower), Some(7));
    }

    #[test]
    fn parses_http_request_parts() {
        let request = "POST /api/mobile-sync/v1/queue-item HTTP/1.1\r\nX-HiddenShield-Pairing-Code: 123\r\n\r\n{}";
        let (method, path, headers, body) = parse_http_request(request).unwrap();
        assert_eq!(method, "POST");
        assert_eq!(path, "/api/mobile-sync/v1/queue-item");
        assert_eq!(
            headers
                .get("x-hiddenshield-pairing-code")
                .map(String::as_str),
            Some("123")
        );
        assert_eq!(body, "{}");
    }

    #[test]
    fn sync_server_accepts_queue_item_and_persists_event_and_vault_record() {
        let temp_dir = tempfile::tempdir().unwrap();
        let db_path = temp_dir.path().join("vault.db");
        super::super::storage::save_pairing_code(temp_dir.path(), "test-code").unwrap();
        let (addr, stop, handle) =
            start_test_sync_server(db_path.clone(), temp_dir.path().to_path_buf()).unwrap();

        let body = serde_json::json!({
            "queueId": "queue-e2e",
            "recordId": "record-e2e",
            "operation": "upsertVaultRecord",
            "payloadType": "vault_record",
            "payload": {
                "id": "record-e2e",
                "kind": "image",
                "title": "e2e.png",
                "watermark_uid": "uid-e2e",
                "sha256": "hash-e2e",
                "created_at": "2026-06-16T12:00:00.000Z"
            }
        })
        .to_string();
        let request = format!(
            "POST /api/mobile-sync/v1/queue-item HTTP/1.1\r\nHost: {addr}\r\nContent-Type: application/json\r\nX-HiddenShield-Pairing-Code: test-code\r\nContent-Length: {}\r\n\r\n{}",
            body.len(),
            body
        );

        let mut stream = TcpStream::connect(addr).unwrap();
        stream.write_all(request.as_bytes()).unwrap();
        let mut response = String::new();
        stream.read_to_string(&mut response).unwrap();
        assert!(response.contains("200 OK"));
        assert!(response.contains("queue-e2e"));

        stop.store(true, Ordering::SeqCst);
        let _ = TcpStream::connect(addr);
        handle.join().unwrap();

        let conn = Connection::open(db_path).unwrap();
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sync_events WHERE queue_id = 'queue-e2e'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 1);
        let vault_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM vault_records WHERE watermark_uid = 'uid-e2e'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(vault_count, 1);
    }

    #[test]
    fn sync_server_accepts_queue_batch_and_persists_all_items() {
        let temp_dir = tempfile::tempdir().unwrap();
        let db_path = temp_dir.path().join("vault.db");
        super::super::storage::save_pairing_code(temp_dir.path(), "test-code").unwrap();
        let (addr, stop, handle) =
            start_test_sync_server(db_path.clone(), temp_dir.path().to_path_buf()).unwrap();

        let body = serde_json::json!({
            "items": [
                {
                    "queueId": "queue-batch-vault",
                    "recordId": "record-batch-vault",
                    "operation": "upsertVaultRecord",
                    "payloadType": "vault_record",
                    "payload": {
                        "id": "record-batch-vault",
                        "kind": "image",
                        "title": "batch.png",
                        "watermark_uid": "uid-batch-vault",
                        "sha256": "hash-batch",
                        "revision": 1,
                        "created_at": "2026-06-16T12:00:00.000Z"
                    }
                },
                {
                    "queueId": "queue-batch-evidence",
                    "recordId": "record-batch-evidence",
                    "operation": "upsertEvidenceRecord",
                    "payloadType": "evidence_record",
                    "payload": {
                        "id": "record-batch-evidence",
                        "kind": "image",
                        "title": "suspect.png",
                        "watermark_uid": "uid-batch-vault",
                        "revision": 1,
                        "extracted_timestamp": 123
                    }
                }
            ]
        })
        .to_string();
        let request = format!(
            "POST /api/mobile-sync/v1/queue-batch HTTP/1.1\r\nHost: {addr}\r\nContent-Type: application/json\r\nX-HiddenShield-Pairing-Code: test-code\r\nContent-Length: {}\r\n\r\n{}",
            body.len(),
            body
        );

        let mut stream = TcpStream::connect(addr).unwrap();
        stream.write_all(request.as_bytes()).unwrap();
        let mut response = String::new();
        stream.read_to_string(&mut response).unwrap();
        assert!(response.contains("200 OK"));
        assert!(response.contains("\"accepted\":2"));
        assert!(response.contains("queue-batch-vault"));
        assert!(response.contains("queue-batch-evidence"));

        stop.store(true, Ordering::SeqCst);
        let _ = TcpStream::connect(addr);
        handle.join().unwrap();

        let conn = Connection::open(db_path).unwrap();
        let event_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM sync_events", [], |row| row.get(0))
            .unwrap();
        let vault_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM vault_records WHERE watermark_uid = 'uid-batch-vault'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        let evidence_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM sync_evidence_records", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(event_count, 2);
        assert_eq!(vault_count, 1);
        assert_eq!(evidence_count, 1);
    }

    #[test]
    fn sync_server_returns_desktop_changes() {
        let temp_dir = tempfile::tempdir().unwrap();
        let db_path = temp_dir.path().join("vault.db");
        super::super::storage::save_pairing_code(temp_dir.path(), "test-code").unwrap();
        let (addr, stop, handle) =
            start_test_sync_server(db_path.clone(), temp_dir.path().to_path_buf()).unwrap();

        let body = serde_json::json!({
            "queueId": "queue-change",
            "recordId": "record-change",
            "operation": "upsertVaultRecord",
            "payloadType": "vault_record",
            "payload": {
                "id": "record-change",
                "kind": "image",
                "title": "change.png",
                "watermark_uid": "uid-change",
                "sha256": "hash-change",
                "revision": 1,
                "created_at": "2026-06-16T12:00:00.000Z"
            }
        })
        .to_string();
        let post_request = format!(
            "POST /api/mobile-sync/v1/queue-item HTTP/1.1\r\nHost: {addr}\r\nContent-Type: application/json\r\nX-HiddenShield-Pairing-Code: test-code\r\nContent-Length: {}\r\n\r\n{}",
            body.len(),
            body
        );
        let mut post_stream = TcpStream::connect(addr).unwrap();
        post_stream.write_all(post_request.as_bytes()).unwrap();
        let mut post_response = String::new();
        post_stream.read_to_string(&mut post_response).unwrap();
        assert!(post_response.contains("200 OK"));

        let get_request = format!(
            "GET /api/mobile-sync/v1/changes?since=2026-06-16T11:00:00.000Z HTTP/1.1\r\nHost: {addr}\r\nX-HiddenShield-Pairing-Code: test-code\r\n\r\n"
        );
        let mut get_stream = TcpStream::connect(addr).unwrap();
        get_stream.write_all(get_request.as_bytes()).unwrap();
        let mut get_response = String::new();
        get_stream.read_to_string(&mut get_response).unwrap();
        assert!(get_response.contains("200 OK"));
        assert!(get_response.contains("uid-change"));
        assert!(get_response.contains("\"changes\""));

        stop.store(true, Ordering::SeqCst);
        let _ = TcpStream::connect(addr);
        handle.join().unwrap();
    }

    #[test]
    fn sync_server_rejects_invalid_pairing_code() {
        let temp_dir = tempfile::tempdir().unwrap();
        let db_path = temp_dir.path().join("vault.db");
        super::super::storage::save_pairing_code(temp_dir.path(), "expected-code").unwrap();
        let (addr, stop, handle) =
            start_test_sync_server(db_path.clone(), temp_dir.path().to_path_buf()).unwrap();

        let body = serde_json::json!({
            "queueId": "queue-rejected",
            "recordId": "record-rejected",
            "operation": "upsertVaultRecord",
            "payloadType": "vault_record",
            "payload": {}
        })
        .to_string();
        let request = format!(
            "POST /api/mobile-sync/v1/queue-item HTTP/1.1\r\nHost: {addr}\r\nContent-Type: application/json\r\nX-HiddenShield-Pairing-Code: wrong-code\r\nContent-Length: {}\r\n\r\n{}",
            body.len(),
            body
        );

        let mut stream = TcpStream::connect(addr).unwrap();
        stream.write_all(request.as_bytes()).unwrap();
        let mut response = String::new();
        stream.read_to_string(&mut response).unwrap();
        assert!(response.contains("403 Forbidden"));
        assert!(response.contains("pairing_code_invalid"));

        stop.store(true, Ordering::SeqCst);
        let _ = TcpStream::connect(addr);
        handle.join().unwrap();

        let conn = Connection::open(db_path).unwrap();
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM sync_events", [], |row| row.get(0))
            .unwrap();
        assert_eq!(count, 0);
    }
}
