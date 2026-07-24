// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    if hidden_shield_lib::cloud_sync_runtime_qa::run_from_env_if_requested() {
        return;
    }
    hidden_shield_lib::run();
}
