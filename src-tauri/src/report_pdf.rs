use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::sync::mpsc;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use tauri::{AppHandle, Manager, Runtime};

use crate::utils::process::hide_window;

pub const REPORT_PDF_GENERATION_BUDGET_MS: u64 = 3_000;
const REPORT_PDF_WORKER_STARTUP_TIMEOUT: Duration = Duration::from_secs(15);
const REPORT_PDF_WORKER_RESPONSE_TIMEOUT: Duration = Duration::from_secs(5);

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReportPdfRenderResult {
    pub generation_ms: f64,
    pub page_count: usize,
    pub bytes: u64,
    pub sha256: String,
    pub page_overflow: Vec<ReportPdfPageOverflow>,
    pub font_state: ReportPdfFontState,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReportPdfPageOverflow {
    pub page: usize,
    pub client_height: u64,
    pub scroll_height: u64,
    pub overflow: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReportPdfFontState {
    pub sans_loaded: bool,
    pub serif_loaded: bool,
}

#[derive(Default)]
pub struct ReportPdfWorkerManager {
    worker: Option<ReportPdfWorker>,
}

impl ReportPdfWorkerManager {
    pub fn prewarm<R: Runtime>(&mut self, app_handle: &AppHandle<R>) -> Result<(), String> {
        self.ensure_worker(app_handle).map(|_| ())
    }

    pub fn render<R: Runtime>(
        &mut self,
        app_handle: &AppHandle<R>,
        document: &Value,
        output_path: &Path,
    ) -> Result<ReportPdfRenderResult, String> {
        let first_result = self
            .ensure_worker(app_handle)
            .and_then(|worker| worker.render(document, output_path));
        if first_result.is_ok() {
            return first_result;
        }

        self.worker = None;
        self.ensure_worker(app_handle)?
            .render(document, output_path)
            .map_err(|retry_error| {
                format!(
                    "Chromium 报告 worker 渲染失败；首次错误: {}; 重试错误: {retry_error}",
                    first_result.expect_err("first render result must be an error")
                )
            })
    }

    fn ensure_worker<R: Runtime>(
        &mut self,
        app_handle: &AppHandle<R>,
    ) -> Result<&mut ReportPdfWorker, String> {
        if self.worker.is_none() {
            self.worker = Some(ReportPdfWorker::spawn(resolve_worker_config(app_handle)?)?);
        }
        self.worker
            .as_mut()
            .ok_or_else(|| "Chromium 报告 worker 未初始化".to_string())
    }
}

struct ReportPdfWorker {
    child: Child,
    stdin: ChildStdin,
    stdout: Option<BufReader<ChildStdout>>,
    next_request_id: u64,
}

impl ReportPdfWorker {
    fn spawn(config: ReportPdfWorkerConfig) -> Result<Self, String> {
        let mut command = Command::new(&config.node_path);
        command
            .arg(&config.worker_path)
            .arg("--resourceDir")
            .arg(&config.resource_dir)
            .arg("--maxGenerationMs")
            .arg(REPORT_PDF_GENERATION_BUDGET_MS.to_string())
            .current_dir(&config.working_dir)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null());
        hide_window(&mut command);

        let mut child = command.spawn().map_err(|error| {
            format!(
                "启动 Chromium 报告 worker 失败（Node: {}）: {error}",
                config.node_path.display()
            )
        })?;
        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| "无法连接 Chromium 报告 worker stdin".to_string())?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| "无法连接 Chromium 报告 worker stdout".to_string())?;
        let mut worker = Self {
            child,
            stdin,
            stdout: Some(BufReader::new(stdout)),
            next_request_id: 1,
        };

        let ready = worker.read_message::<WorkerReadyMessage>(REPORT_PDF_WORKER_STARTUP_TIMEOUT)?;
        if ready.message_type != "ready" {
            return Err(format!(
                "Chromium 报告 worker 返回了无效启动消息: {}",
                ready.message_type
            ));
        }
        log::info!(
            "Chromium report worker ready in {:.2}ms using {}",
            ready.launch_ms,
            ready.executable_path
        );
        Ok(worker)
    }

    fn render(
        &mut self,
        document: &Value,
        output_path: &Path,
    ) -> Result<ReportPdfRenderResult, String> {
        let request_id = self.next_request_id;
        self.next_request_id += 1;
        let request = WorkerRenderRequest {
            message_type: "render",
            request_id,
            document,
            output_path: output_path.to_string_lossy(),
        };
        let serialized = serde_json::to_string(&request)
            .map_err(|error| format!("序列化 Chromium 报告请求失败: {error}"))?;
        writeln!(self.stdin, "{serialized}")
            .and_then(|_| self.stdin.flush())
            .map_err(|error| format!("发送 Chromium 报告请求失败: {error}"))?;

        let response =
            self.read_message::<WorkerRenderResponse>(REPORT_PDF_WORKER_RESPONSE_TIMEOUT)?;
        if response.request_id != Some(request_id) {
            return Err(format!(
                "Chromium 报告 worker 响应编号不匹配: expected {request_id}, got {:?}",
                response.request_id
            ));
        }
        if !response.ok {
            return Err(response
                .error
                .unwrap_or_else(|| "Chromium 报告 worker 返回未知错误".to_string()));
        }
        let result = response
            .result()
            .ok_or_else(|| "Chromium 报告 worker 缺少渲染指标".to_string())?;
        validate_render_result(&result)?;
        Ok(result)
    }

    fn read_message<T: for<'de> Deserialize<'de>>(
        &mut self,
        timeout: Duration,
    ) -> Result<T, String> {
        let mut stdout = self
            .stdout
            .take()
            .ok_or_else(|| "Chromium 报告 worker stdout 不可用".to_string())?;
        let (sender, receiver) = mpsc::sync_channel(1);
        std::thread::spawn(move || {
            let mut line = String::new();
            let result = stdout
                .read_line(&mut line)
                .map(|bytes| (bytes, line))
                .map_err(|error| format!("读取 Chromium 报告 worker 响应失败: {error}"));
            let _ = sender.send((stdout, result));
        });
        let (stdout, response) = match receiver.recv_timeout(timeout) {
            Ok(response) => response,
            Err(mpsc::RecvTimeoutError::Timeout) => {
                let _ = self.child.kill();
                let _ = self.child.wait();
                return Err(format!(
                    "REPORT_PDF_WORKER_RESPONSE_TIMEOUT: {}ms",
                    timeout.as_millis()
                ));
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                return Err("Chromium 报告 worker 响应线程异常退出".to_string());
            }
        };
        self.stdout = Some(stdout);
        let (bytes, line) = response?;
        if bytes == 0 {
            let status = self
                .child
                .try_wait()
                .map_err(|error| format!("读取 Chromium worker 状态失败: {error}"))?;
            return Err(format!("Chromium 报告 worker 已退出: {status:?}"));
        }
        serde_json::from_str(&line)
            .map_err(|error| format!("解析 Chromium 报告 worker 响应失败: {error}; line={line}"))
    }
}

impl Drop for ReportPdfWorker {
    fn drop(&mut self) {
        let _ = writeln!(self.stdin, "{{\"type\":\"shutdown\"}}");
        let _ = self.stdin.flush();
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

#[derive(Debug)]
struct ReportPdfWorkerConfig {
    node_path: PathBuf,
    worker_path: PathBuf,
    resource_dir: PathBuf,
    working_dir: PathBuf,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct WorkerRenderRequest<'a> {
    #[serde(rename = "type")]
    message_type: &'static str,
    request_id: u64,
    document: &'a Value,
    output_path: std::borrow::Cow<'a, str>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct WorkerReadyMessage {
    #[serde(rename = "type")]
    message_type: String,
    launch_ms: f64,
    executable_path: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct WorkerRenderResponse {
    request_id: Option<u64>,
    ok: bool,
    error: Option<String>,
    generation_ms: Option<f64>,
    page_count: Option<usize>,
    bytes: Option<u64>,
    sha256: Option<String>,
    page_overflow: Option<Vec<ReportPdfPageOverflow>>,
    font_state: Option<ReportPdfFontState>,
}

impl WorkerRenderResponse {
    fn result(self) -> Option<ReportPdfRenderResult> {
        Some(ReportPdfRenderResult {
            generation_ms: self.generation_ms?,
            page_count: self.page_count?,
            bytes: self.bytes?,
            sha256: self.sha256?,
            page_overflow: self.page_overflow?,
            font_state: self.font_state?,
        })
    }
}

fn resolve_worker_config<R: Runtime>(
    app_handle: &AppHandle<R>,
) -> Result<ReportPdfWorkerConfig, String> {
    let resource_dir = std::env::var_os("HIDDENSHIELD_REPORT_PDF_RESOURCE_DIR")
        .map(PathBuf::from)
        .filter(|path| path.join("chromium-worker.mjs").is_file())
        .or_else(|| {
            app_handle
                .path()
                .resource_dir()
                .ok()
                .map(|path| path.join("report-pdf"))
                .filter(|path| path.join("chromium-worker.mjs").is_file())
        })
        .or_else(|| {
            let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("resources/report-pdf");
            path.join("chromium-worker.mjs").is_file().then_some(path)
        })
        .ok_or_else(|| "未找到 Chromium 报告资源目录".to_string())?;
    let worker_path = resource_dir.join("chromium-worker.mjs");
    let node_path = std::env::var_os("HIDDENSHIELD_NODE_PATH")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            if cfg!(windows) {
                PathBuf::from("node.exe")
            } else {
                PathBuf::from("node")
            }
        });
    let working_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")));
    Ok(ReportPdfWorkerConfig {
        node_path,
        worker_path,
        resource_dir,
        working_dir,
    })
}

fn validate_render_result(result: &ReportPdfRenderResult) -> Result<(), String> {
    if result.generation_ms > REPORT_PDF_GENERATION_BUDGET_MS as f64 {
        return Err(format!(
            "REPORT_PDF_GENERATION_BUDGET_EXCEEDED: {:.2}ms > {}ms",
            result.generation_ms, REPORT_PDF_GENERATION_BUDGET_MS
        ));
    }
    if result.page_count != 4 {
        return Err(format!(
            "REPORT_PDF_PAGE_COUNT_INVALID: expected 4, got {}",
            result.page_count
        ));
    }
    if result.page_overflow.iter().any(|page| page.overflow) {
        return Err("REPORT_PDF_PAGE_OVERFLOW_DETECTED".to_string());
    }
    if !result.font_state.sans_loaded || !result.font_state.serif_loaded {
        return Err("REPORT_PDF_CONTROLLED_FONT_NOT_LOADED".to_string());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn successful_result(generation_ms: f64) -> ReportPdfRenderResult {
        ReportPdfRenderResult {
            generation_ms,
            page_count: 4,
            bytes: 100,
            sha256: "a".repeat(64),
            page_overflow: (1..=4)
                .map(|page| ReportPdfPageOverflow {
                    page,
                    client_height: 1123,
                    scroll_height: 1123,
                    overflow: false,
                })
                .collect(),
            font_state: ReportPdfFontState {
                sans_loaded: true,
                serif_loaded: true,
            },
        }
    }

    #[test]
    fn accepts_render_within_three_second_budget() {
        assert!(validate_render_result(&successful_result(2_999.99)).is_ok());
    }

    #[test]
    fn rejects_render_over_three_second_budget() {
        let error = validate_render_result(&successful_result(3_000.01))
            .expect_err("render over budget must fail");
        assert!(error.contains("REPORT_PDF_GENERATION_BUDGET_EXCEEDED"));
    }

    #[test]
    fn rejects_missing_controlled_fonts() {
        let mut result = successful_result(500.0);
        result.font_state.serif_loaded = false;
        assert_eq!(
            validate_render_result(&result).expect_err("missing font must fail"),
            "REPORT_PDF_CONTROLLED_FONT_NOT_LOADED"
        );
    }
}
