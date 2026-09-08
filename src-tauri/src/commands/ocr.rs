use crate::error::AppError;
use crate::ocr::{self, extract_text_from_bytes, extract_text_from_path, OcrResult};
use crate::parser::{self, ReferenceContext};

#[tauri::command]
pub fn extract_text_from_image(path: String) -> Result<OcrResult, AppError> {
    let mut ocr_res = extract_text_from_path(&path)?;
    let spatial_text = ocr::reconstruct_spatial_lines(&ocr_res.lines);
    let context = ReferenceContext::default();
    if !spatial_text.trim().is_empty()
        && parser::parse_schedule_table_events(&spatial_text, &context).len() >= 2
    {
        ocr_res.text = spatial_text;
    }
    Ok(ocr_res)
}

#[tauri::command]
pub fn extract_text_from_image_bytes(bytes: Vec<u8>) -> Result<OcrResult, AppError> {
    let mut ocr_res = extract_text_from_bytes(&bytes)?;
    let spatial_text = ocr::reconstruct_spatial_lines(&ocr_res.lines);
    let context = ReferenceContext::default();
    if !spatial_text.trim().is_empty()
        && parser::parse_schedule_table_events(&spatial_text, &context).len() >= 2
    {
        ocr_res.text = spatial_text;
    }
    Ok(ocr_res)
}
