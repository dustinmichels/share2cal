pub mod ocr;

use ocr::{extract_text_from_bytes, extract_text_from_path, OcrResult};

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[tauri::command]
fn extract_text_from_image(path: String) -> Result<OcrResult, String> {
    extract_text_from_path(&path)
}

#[tauri::command]
fn extract_text_from_image_bytes(bytes: Vec<u8>) -> Result<OcrResult, String> {
    extract_text_from_bytes(&bytes)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            greet,
            extract_text_from_image,
            extract_text_from_image_bytes
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
