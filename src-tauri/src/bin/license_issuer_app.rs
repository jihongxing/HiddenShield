#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::env;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;

use chrono::{Duration, SecondsFormat, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use zeroize::Zeroizing;

const ISSUER_PASSWORD_ENV: &str = "HIDDENSHIELD_ISSUER_PASSWORD";
const SERVICE_PROVIDER_DIRECTORY: &str = ".hiddenshield-service-provider";
const PRODUCTION_SIGNING_DIRECTORY: &str = "production-signing\\20260717";
const PRODUCTION_KEY_FILE: &str = "hslic1-ed25519-production-key.json";
const PRODUCTION_PASSWORD_FILE: &str = "hslic1-password.dpapi.xml";
const DELIVERY_DIRECTORY: &str = "HiddenShield-License-Delivery";
const OPERATOR_ID: &str = "ops-jihx";

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct IssueLicenseOutput {
    license_path: String,
    audit_path: String,
    token: String,
    license_id: String,
    customer_reference: String,
    expires_at: String,
}

#[derive(Debug)]
struct ServicePaths {
    root: PathBuf,
    key_path: PathBuf,
    password_path: PathBuf,
    delivery_root: PathBuf,
    sequence_state_path: PathBuf,
}

#[derive(Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct SequenceState {
    date: String,
    last_sequence: u32,
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            inspect_activation_request,
            issue_license,
            issuer_readiness,
        ])
        .run(tauri::generate_context!())
        .expect("授权签发台启动失败");
}

#[tauri::command]
fn issuer_readiness() -> Result<(), String> {
    let paths = service_paths()?;
    let binary = issuer_binary_path()?;
    if !binary.is_file() {
        return Err(format!(
            "找不到签发引擎：{}。请先执行 npm run issuer:cli。",
            binary.display()
        ));
    }
    if !paths.key_path.is_file() || !paths.password_path.is_file() {
        return Err(
            "未找到服务方正式签发材料。请确认当前 Windows 用户拥有服务方目录访问权限。".to_string(),
        );
    }
    Ok(())
}

#[tauri::command]
fn inspect_activation_request(request_path: String) -> Result<Value, String> {
    let output = run_issuer(&["inspect-request", "--request", &request_path], None)?;
    let parsed = parse_issuer_json(&output)?;
    if parsed.get("status").and_then(Value::as_str) != Some("valid") {
        return Err("激活请求校验未通过。".to_string());
    }
    Ok(parsed)
}

#[tauri::command]
fn issue_license(request_path: String) -> Result<IssueLicenseOutput, String> {
    let paths = service_paths()?;
    let password = load_dpapi_password(&paths.password_path)?;
    let customer_reference = reserve_customer_reference(&paths)?;
    let output_directory = paths.delivery_root.join(&customer_reference);
    fs::create_dir_all(&output_directory).map_err(|_| "无法创建客户交付目录。".to_string())?;
    let license_path = output_directory.join("HiddenShield-License.hslicense");
    let audit_path = output_directory.join("issuance-audit.json");
    let expires_at = (Utc::now() + Duration::days(364)).to_rfc3339_opts(SecondsFormat::Secs, true);
    let license_path_text = license_path.to_string_lossy().to_string();
    let audit_path_text = audit_path.to_string_lossy().to_string();
    let arguments = vec![
        "issue".to_string(),
        "--key".to_string(),
        paths.key_path.to_string_lossy().to_string(),
        "--password-env".to_string(),
        ISSUER_PASSWORD_ENV.to_string(),
        "--request".to_string(),
        request_path,
        "--expires-at".to_string(),
        expires_at.clone(),
        "--operator-id".to_string(),
        OPERATOR_ID.to_string(),
        "--output".to_string(),
        license_path_text.clone(),
        "--audit-output".to_string(),
        audit_path_text.clone(),
    ];
    let argument_references = arguments.iter().map(String::as_str).collect::<Vec<_>>();
    let output = run_issuer(&argument_references, Some(password.as_str()))?;
    let issued = parse_issuer_json(&output)?;
    if issued.get("status").and_then(Value::as_str) != Some("issued") {
        return Err("签发引擎没有返回成功结果。".to_string());
    }
    let token = fs::read_to_string(&license_path)
        .map_err(|_| "签发完成，但无法读取许可证交付文件。".to_string())?
        .trim()
        .to_string();
    let license_id = issued
        .get("licenseId")
        .and_then(Value::as_str)
        .ok_or_else(|| "签发完成，但未返回许可证编号。".to_string())?
        .to_string();

    Ok(IssueLicenseOutput {
        license_path: license_path_text,
        audit_path: audit_path_text,
        token,
        license_id,
        customer_reference,
        expires_at,
    })
}

fn service_paths() -> Result<ServicePaths, String> {
    let user_profile = env::var_os("USERPROFILE")
        .map(PathBuf::from)
        .ok_or_else(|| "无法读取当前 Windows 用户目录。".to_string())?;
    let root = env::var_os("HIDDENSHIELD_ISSUER_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            user_profile
                .join(SERVICE_PROVIDER_DIRECTORY)
                .join(PRODUCTION_SIGNING_DIRECTORY)
        });
    Ok(ServicePaths {
        key_path: root.join(PRODUCTION_KEY_FILE),
        password_path: root.join(PRODUCTION_PASSWORD_FILE),
        delivery_root: user_profile.join("Documents").join(DELIVERY_DIRECTORY),
        sequence_state_path: root.join("issuer-sequence-state.json"),
        root,
    })
}

fn load_dpapi_password(password_path: &Path) -> Result<Zeroizing<String>, String> {
    if !cfg!(windows) {
        return Err("授权签发台当前只支持 Windows DPAPI 服务方材料。".to_string());
    }
    let script = "$credential = Import-Clixml -LiteralPath $env:HIDDENSHIELD_ISSUER_PASSWORD_FILE; [Console]::Out.Write($credential.GetNetworkCredential().Password)";
    let output = Command::new("powershell.exe")
        .args(["-NoProfile", "-NonInteractive", "-Command", script])
        .env("HIDDENSHIELD_ISSUER_PASSWORD_FILE", password_path)
        .output()
        .map_err(|_| "无法读取受保护的签发口令。".to_string())?;
    if !output.status.success() {
        return Err(
            "无法读取受保护的签发口令。请使用保存签发材料的 Windows 用户启动签发台。".to_string(),
        );
    }
    let password =
        String::from_utf8(output.stdout).map_err(|_| "受保护的签发口令格式无效。".to_string())?;
    if password.chars().count() < 8 {
        return Err("受保护的签发口令不符合最小长度要求。".to_string());
    }
    Ok(Zeroizing::new(password))
}

fn reserve_customer_reference(paths: &ServicePaths) -> Result<String, String> {
    fs::create_dir_all(&paths.root).map_err(|_| "无法访问服务方签发状态目录。".to_string())?;
    let date = Utc::now().format("%Y%m%d").to_string();
    let mut state = read_sequence_state(&paths.sequence_state_path)?;
    let sequence = if state.date == date {
        state
            .last_sequence
            .checked_add(1)
            .ok_or_else(|| "当天签发序号已超出范围。".to_string())?
    } else {
        1
    };
    state.date = date.clone();
    state.last_sequence = sequence;
    write_sequence_state(&paths.sequence_state_path, &state)?;
    Ok(format!("{date}-{sequence:05}"))
}

fn read_sequence_state(path: &Path) -> Result<SequenceState, String> {
    if !path.exists() {
        return Ok(SequenceState::default());
    }
    let bytes = fs::read(path).map_err(|_| "无法读取签发序号状态。".to_string())?;
    serde_json::from_slice(&bytes).map_err(|_| "签发序号状态格式无效。".to_string())
}

fn write_sequence_state(path: &Path, state: &SequenceState) -> Result<(), String> {
    let temporary_path = path.with_extension("json.pending");
    let bytes =
        serde_json::to_vec_pretty(state).map_err(|_| "无法写入签发序号状态。".to_string())?;
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temporary_path)
        .map_err(|_| "无法锁定签发序号状态。".to_string())?;
    file.write_all(&bytes)
        .and_then(|_| file.sync_all())
        .map_err(|_| "无法写入签发序号状态。".to_string())?;
    if path.exists() {
        fs::remove_file(path).map_err(|_| "无法更新签发序号状态。".to_string())?;
    }
    fs::rename(&temporary_path, path).map_err(|_| "无法更新签发序号状态。".to_string())
}

fn issuer_binary_path() -> Result<PathBuf, String> {
    if let Some(path) = env::var_os("HIDDENSHIELD_ISSUER_CLI") {
        return Ok(PathBuf::from(path));
    }
    let filename = if cfg!(windows) {
        "offline_license_issuer.exe"
    } else {
        "offline_license_issuer"
    };
    Ok(PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join(if cfg!(debug_assertions) {
            "debug"
        } else {
            "release"
        })
        .join("examples")
        .join(filename))
}

fn run_issuer(arguments: &[&str], password: Option<&str>) -> Result<String, String> {
    let binary = issuer_binary_path()?;
    if !binary.is_file() {
        return Err(format!(
            "找不到签发引擎：{}。请先执行 npm run issuer:cli。",
            binary.display()
        ));
    }
    let mut command = Command::new(binary);
    command.args(arguments);
    command.env_remove(ISSUER_PASSWORD_ENV);
    if let Some(password) = password {
        command.env(ISSUER_PASSWORD_ENV, password);
    }
    let output = command
        .output()
        .map_err(|_| "无法启动签发引擎。".to_string())?;
    if !output.status.success() {
        let error = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(if error.is_empty() {
            "签发引擎执行失败。".to_string()
        } else {
            format!("签发失败：{error}")
        });
    }
    String::from_utf8(output.stdout).map_err(|_| "签发引擎返回了无法识别的结果。".to_string())
}

fn parse_issuer_json(output: &str) -> Result<Value, String> {
    serde_json::from_str(output.trim())
        .map_err(|_| "签发引擎返回格式无效，请检查其版本。".to_string())
}
