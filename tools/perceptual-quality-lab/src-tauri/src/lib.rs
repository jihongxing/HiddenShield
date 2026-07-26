mod media;

use media::{
    analyze_audio_pair, analyze_image_pair, clear_lab_session, inspect_media_pair,
    prepare_abx_assets, LabState,
};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(LabState::default())
        .invoke_handler(tauri::generate_handler![
            inspect_media_pair,
            analyze_image_pair,
            analyze_audio_pair,
            prepare_abx_assets,
            clear_lab_session
        ])
        .run(tauri::generate_context!())
        .expect("failed to run HiddenShield perceptual quality laboratory");
}
