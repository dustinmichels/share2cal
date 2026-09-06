fn main() {
    let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    if target_os == "macos" || target_os == "ios" {
        cc::Build::new()
            .file("src/ocr_apple.m")
            .flag("-fobjc-arc")
            .compile("ocr_apple");

        println!("cargo:rustc-link-lib=framework=Vision");
        println!("cargo:rustc-link-lib=framework=CoreGraphics");
        println!("cargo:rustc-link-lib=framework=ImageIO");
        println!("cargo:rustc-link-lib=framework=Foundation");
    }

    tauri_build::build()
}
