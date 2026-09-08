use crate::error::AppError;
use crate::share::{self, SharedImagePayload};

#[tauri::command]
pub fn get_pending_shared_image(
    include_bytes: Option<bool>,
) -> Result<Option<SharedImagePayload>, AppError> {
    share::get_pending_shared_image(include_bytes.unwrap_or(true))
}

#[tauri::command]
pub fn clear_pending_shared_image() -> Result<(), AppError> {
    share::clear_pending_shared_image()
}

#[tauri::command]
pub fn stage_shared_image(
    bytes: Vec<u8>,
    file_name: String,
    mime_type: Option<String>,
) -> Result<SharedImagePayload, AppError> {
    share::stage_shared_image(&bytes, &file_name, mime_type.as_deref())
}

#[tauri::command]
pub fn load_image_from_path(path: String) -> Result<SharedImagePayload, AppError> {
    share::load_image_from_path(&path)
}
