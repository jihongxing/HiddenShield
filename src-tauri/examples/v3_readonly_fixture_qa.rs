use std::fs;
use std::path::PathBuf;

use hidden_shield_lib::commands::v3_readonly_fixture::{
    build_v3_readonly_fixture_bytes, build_v3_readonly_fixture_media_bytes,
    decode_v3_readonly_fixture_for_desktop, decode_v3_readonly_media_fixture_for_desktop,
};

fn main() -> Result<(), String> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let out_dir = PathBuf::from(required_arg(&args, "--out-dir")?);
    fs::create_dir_all(&out_dir).map_err(|error| format!("create output dir: {error}"))?;

    let image_bytes = build_v3_readonly_fixture_bytes("image")?;
    let audio_bytes = build_v3_readonly_fixture_bytes("audio")?;
    let image_media_bytes = build_v3_readonly_fixture_media_bytes("image")?;
    let audio_media_bytes = build_v3_readonly_fixture_media_bytes("audio")?;
    let image_path = out_dir.join("v3-readonly-image-anchor.bin");
    let audio_path = out_dir.join("v3-readonly-audio-anchor.bin");
    let image_media_path = out_dir.join("v3-readonly-image-container.png");
    let audio_media_path = out_dir.join("v3-readonly-audio-container.wav");
    fs::write(&image_path, &image_bytes)
        .map_err(|error| format!("write image fixture: {error}"))?;
    fs::write(&audio_path, &audio_bytes)
        .map_err(|error| format!("write audio fixture: {error}"))?;
    fs::write(&image_media_path, &image_media_bytes)
        .map_err(|error| format!("write image media fixture: {error}"))?;
    fs::write(&audio_media_path, &audio_media_bytes)
        .map_err(|error| format!("write audio media fixture: {error}"))?;

    let image = decode_v3_readonly_fixture_for_desktop(
        "v3_image_desktop_write_mobile_read",
        "image",
        &image_bytes,
    )?;
    let audio = decode_v3_readonly_fixture_for_desktop(
        "v3_audio_desktop_write_mobile_read",
        "audio",
        &audio_bytes,
    )?;
    let image_media = decode_v3_readonly_media_fixture_for_desktop(
        "v3_image_desktop_write_mobile_read",
        "image",
        &image_media_bytes,
    )?;
    let audio_media = decode_v3_readonly_media_fixture_for_desktop(
        "v3_audio_desktop_write_mobile_read",
        "audio",
        &audio_media_bytes,
    )?;
    let result = serde_json::json!({
        "desktop": {
            "image": image,
            "audio": audio,
            "imageMedia": image_media,
            "audioMedia": audio_media,
        },
        "fixtures": {
            "imageBytesPath": image_path,
            "audioBytesPath": audio_path,
            "imageMediaPath": image_media_path,
            "audioMediaPath": audio_media_path,
        },
        "defaultV3WriteEnabled": true,
    });
    let json = serde_json::to_string_pretty(&result)
        .map_err(|error| format!("serialize result: {error}"))?;
    let json_path = out_dir.join("desktop-v3-readonly-fixtures.json");
    fs::write(&json_path, format!("{json}\n")).map_err(|error| format!("write json: {error}"))?;
    println!("{json}");
    Ok(())
}

fn required_arg<'a>(args: &'a [String], name: &str) -> Result<&'a str, String> {
    args.windows(2)
        .find(|pair| pair[0] == name)
        .map(|pair| pair[1].as_str())
        .ok_or_else(|| format!("missing required argument {name}"))
}
