fn main() {
    // CARGO_MANIFEST_DIR = <root>/native-sys
    // zig-out is at <root>/zig-out
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let root = std::path::Path::new(&manifest_dir)
        .parent()
        .expect("failed to get parent of manifest dir");
    let lib_dir = root.join("zig-out").join("lib");

    // On macOS, Zig's archive format may not be 8-byte aligned, which Apple's ld rejects.
    // Re-create the archive with proper alignment using ar.
    if cfg!(target_os = "macos") {
        let lib_path = lib_dir.join("libpledge_native.a");
        if lib_path.exists() {
            let extract_dir = lib_dir.join("_pledge_extracted");
            let _ = std::fs::remove_dir_all(&extract_dir);
            let _ = std::fs::create_dir_all(&extract_dir);

            let extract_ok = std::process::Command::new("ar")
                .arg("x")
                .arg(&lib_path)
                .current_dir(&extract_dir)
                .status()
                .map_or(false, |s| s.success());

            if extract_ok {
                let _ = std::fs::remove_file(&lib_path);
                let mut ar = std::process::Command::new("ar");
                ar.arg("rcs").arg(&lib_path);
                if let Ok(entries) = std::fs::read_dir(&extract_dir) {
                    let mut o_files: Vec<_> = entries
                        .flatten()
                        .map(|e| e.path())
                        .filter(|p| p.extension().map_or(false, |e| e == "o"))
                        .collect();
                    o_files.sort();
                    for f in &o_files {
                        ar.arg(f);
                    }
                }
                let _ = ar.status();
                let _ = std::fs::remove_dir_all(&extract_dir);
            }
        }
    }

    // Tell cargo to look for the static library
    println!("cargo:rustc-link-search=native={}", lib_dir.display());
    println!("cargo:rustc-link-lib=static=pledge_native");

    // Link Windows libraries needed by Zig runtime
    if cfg!(target_os = "windows") {
        println!("cargo:rustc-link-lib=dylib=ntdll");
    }

    // Tell cargo to rerun if Zig source changes
    let zig_src = root.join("native-sys").join("zig");
    println!(
        "cargo:rerun-if-changed={}",
        zig_src.join("lib.zig").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        zig_src.join("io.zig").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        zig_src.join("graph.zig").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        zig_src.join("simd.zig").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        zig_src.join("bench.zig").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        root.join("build.zig").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        root.join("build.zig.zon").display()
    );
}
