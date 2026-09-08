mod common;
use common::get_sample_path;
use share2cal_lib::{share, test_support::*};

#[test]
fn test_share_commands_flow() {
    let _guard = share::TEST_SHARE_MUTEX.lock().unwrap();
    let sample = get_sample_path("gilman_flyer.png");
    if sample.exists() {
        let bytes = std::fs::read(&sample).expect("Should read sample file");
        let staged = stage_shared_image(
            bytes.clone(),
            "flyer_share_test.png".to_string(),
            Some("image/png".to_string()),
        )
        .expect("Stage command should succeed");
        assert_eq!(staged.file_name, "flyer_share_test.png");

        let pending =
            get_pending_shared_image(Some(true)).expect("Get pending command should succeed");
        assert!(pending.is_some());
        let p = pending.unwrap();
        assert_eq!(p.file_name, "flyer_share_test.png");
        assert_eq!(p.bytes.unwrap(), bytes);

        clear_pending_shared_image().expect("Clear pending command should succeed");
        let empty = get_pending_shared_image(Some(false)).expect("Get pending should succeed");
        assert!(empty.is_none());
    }
}

#[test]
fn test_share_empty_bytes_rejected() {
    let _guard = share::TEST_SHARE_MUTEX.lock().unwrap();
    let result = stage_shared_image(vec![], "empty.png".to_string(), None);
    assert!(result.is_err(), "Empty bytes must return an error");
}

#[test]
fn test_share_load_image_from_path() {
    let _guard = share::TEST_SHARE_MUTEX.lock().unwrap();
    let sample = get_sample_path("gilman_flyer.png");
    if sample.exists() {
        let loaded = load_image_from_path(sample.to_str().unwrap().to_string())
            .expect("Load image from path command should succeed");
        assert_eq!(loaded.file_name, "gilman_flyer.png");
        assert!(loaded.bytes.is_some());
        assert!(!loaded.bytes.unwrap().is_empty());
    }
}
