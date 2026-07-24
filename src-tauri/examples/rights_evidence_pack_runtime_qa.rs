use tauri::ipc::{CallbackFn, InvokeBody};
use tauri::webview::InvokeRequest;

fn main() -> Result<(), String> {
    let args = std::env::args().collect::<Vec<_>>();
    let case_dir = required_arg(&args, "--case-dir")?;
    let app = hidden_shield_lib::commands::report::build_rights_evidence_pack_runtime_qa_app()?;
    let webview = tauri::WebviewWindowBuilder::new(&app, "main", Default::default())
        .build()
        .map_err(|error| format!("构建 MockRuntime Webview 失败: {error}"))?;
    let response = tauri::test::get_ipc_response(
        &webview,
        InvokeRequest {
            cmd: "verify_rights_evidence_pack".into(),
            callback: CallbackFn(0),
            error: CallbackFn(1),
            url: "http://tauri.localhost"
                .parse()
                .map_err(|error| format!("构建测试 URL 失败: {error}"))?,
            body: InvokeBody::Json(serde_json::json!({
                "input": {
                    "caseDir": case_dir
                }
            })),
            headers: Default::default(),
            invoke_key: tauri::test::INVOKE_KEY.to_string(),
        },
    )
    .map_err(|error| format!("已注册命令返回错误: {error}"))?;
    let payload = response
        .deserialize::<serde_json::Value>()
        .map_err(|error| format!("反序列化 IPC JSON 失败: {error}"))?;

    assert_json_contract(&payload)?;
    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({
            "status": "passed",
            "command": "verify_rights_evidence_pack",
            "payload": payload
        }))
        .map_err(|error| format!("序列化 QA 输出失败: {error}"))?
    );
    Ok(())
}

fn assert_json_contract(payload: &serde_json::Value) -> Result<(), String> {
    let expected_matched_fields = [
        "directoryContractStatus",
        "attachmentIntegrityStatus",
        "eventChainStatus",
        "attachmentChainStatus",
    ];
    for field in expected_matched_fields {
        if payload.get(field).and_then(serde_json::Value::as_str) != Some("matched") {
            return Err(format!("IPC JSON 字段 {field} 未返回 matched"));
        }
    }
    if payload
        .get("signatureStatus")
        .and_then(serde_json::Value::as_str)
        != Some("not_signed")
    {
        return Err("IPC JSON 未保留 not_signed 状态".to_string());
    }
    if payload
        .get("trustedTimeStatus")
        .and_then(serde_json::Value::as_str)
        != Some("not_timestamped")
    {
        return Err("IPC JSON 未保留 not_timestamped 状态".to_string());
    }
    let declared_root = payload
        .get("declaredRootDigest")
        .and_then(serde_json::Value::as_str);
    let computed_root = payload
        .get("computedRootDigest")
        .and_then(serde_json::Value::as_str);
    if declared_root.is_none() || declared_root != computed_root {
        return Err("IPC JSON 声明 root digest 与复算值不一致".to_string());
    }
    if payload
        .get("attachments")
        .and_then(serde_json::Value::as_array)
        .map(Vec::len)
        != Some(4)
    {
        return Err("IPC JSON 未返回四类附件逐项结果".to_string());
    }
    if payload.get("directory_contract_status").is_some()
        || payload.get("attachment_integrity_status").is_some()
    {
        return Err("IPC JSON 泄露 snake_case 字段".to_string());
    }
    Ok(())
}

fn required_arg(args: &[String], name: &str) -> Result<String, String> {
    args.windows(2)
        .find(|window| window[0] == name)
        .map(|window| window[1].clone())
        .ok_or_else(|| format!("缺少参数 {name}"))
}
