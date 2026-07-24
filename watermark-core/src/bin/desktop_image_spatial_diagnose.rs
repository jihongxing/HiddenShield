use std::{env, fs};

use watermark_core::{
    image_spatial_recovery_v1::diagnose_spatial_recovery_v1_exact,
    PayloadV3MinimalAnchorBuildInput, WatermarkPayloadV3MinimalAnchor,
};

fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let mut args = env::args().skip(1);
    let input_path = args.next().ok_or_else(|| {
        "usage: desktop_image_spatial_diagnose <input> <expected-watermark-uid>".to_string()
    })?;
    let expected_uid = args.next().ok_or_else(|| {
        "usage: desktop_image_spatial_diagnose <input> <expected-watermark-uid>".to_string()
    })?;
    let bytes = fs::read(&input_path).map_err(|error| format!("read {input_path}: {error}"))?;
    let image = image::load_from_memory(&bytes)
        .map_err(|error| format!("decode {input_path}: {error}"))?
        .to_rgba8();
    let expected = WatermarkPayloadV3MinimalAnchor::new(PayloadV3MinimalAnchorBuildInput {
        watermark_id: parse_watermark_uid(&expected_uid)?,
    })
    .map_err(|error| error.to_string())?;
    let diagnostic =
        diagnose_spatial_recovery_v1_exact(&image, &expected).map_err(|error| error.to_string())?;
    println!(
        "{}",
        serde_json::to_string_pretty(&diagnostic).map_err(|error| error.to_string())?
    );
    Ok(())
}

fn parse_watermark_uid(value: &str) -> Result<[u8; 16], String> {
    let compact = value.strip_prefix("HS-").unwrap_or(value).replace('-', "");
    if compact.len() != 32 {
        return Err(format!(
            "invalid watermark UID {value}: expected 32 hexadecimal characters"
        ));
    }

    let mut decoded = [0_u8; 16];
    for (index, byte) in decoded.iter_mut().enumerate() {
        let start = index * 2;
        *byte = u8::from_str_radix(&compact[start..start + 2], 16)
            .map_err(|_| format!("invalid watermark UID {value}: expected hexadecimal digits"))?;
    }
    Ok(decoded)
}
