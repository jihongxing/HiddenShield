use std::path::PathBuf;

use hidden_shield_lib::commands::report::run_mobile_report_handoff_runtime_qa;

fn main() -> Result<(), String> {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    let fixture_dir = required_arg(&args, "--fixture-dir")?;
    let output_dir = required_arg(&args, "--output-dir")?;
    let fixture_dir = std::fs::canonicalize(&fixture_dir)
        .map_err(|error| format!("解析移动交接 fixture 失败: {error}"))?;
    let output_dir = PathBuf::from(output_dir);
    std::fs::create_dir_all(&output_dir)
        .map_err(|error| format!("创建运行态 QA 输出目录失败: {error}"))?;
    std::env::set_var("HIDDENSHIELD_REPORT_OUTPUT_DIR", &output_dir);

    let app = tauri::test::mock_app();
    let exported =
        run_mobile_report_handoff_runtime_qa(app.handle(), fixture_dir.to_string_lossy().as_ref())?;

    println!(
        "{}",
        serde_json::to_string(&exported)
            .map_err(|error| format!("序列化运行态 QA 结果失败: {error}"))?
    );
    Ok(())
}

fn required_arg(args: &[String], name: &str) -> Result<String, String> {
    let index = args
        .iter()
        .position(|value| value == name)
        .ok_or_else(|| format!("缺少参数 {name}"))?;
    args.get(index + 1)
        .cloned()
        .ok_or_else(|| format!("缺少参数值 {name}"))
}
