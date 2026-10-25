fn main() {
    // ggml's Metal sources use `@available`, which references
    // ___isPlatformVersionAtLeast. Rust links the final binary with
    // -nodefaultlibs, so the clang runtime that provides this symbol is not
    // pulled in automatically. Add it explicitly on macOS.
    #[cfg(target_os = "macos")]
    {
        if let Ok(output) = std::process::Command::new("cc")
            .arg("--print-runtime-dir")
            .output()
        {
            if output.status.success() {
                let dir = String::from_utf8_lossy(&output.stdout).trim().to_string();
                if !dir.is_empty() {
                    println!("cargo:rustc-link-search=native={dir}");
                    println!("cargo:rustc-link-lib=static=clang_rt.osx");
                }
            }
        }
    }
    tauri_build::build()
}
