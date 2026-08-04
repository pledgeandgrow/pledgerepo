fn main() {
    // CARGO_MANIFEST_DIR = <root>/native-sys
    // zig-out is at <root>/zig-out
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let root = std::path::Path::new(&manifest_dir)
        .parent()
        .expect("failed to get parent of manifest dir");
    let lib_dir = root.join("zig-out").join("lib");

    // On macOS, Zig's archive format is incompatible with Apple's linker.
    // Link the object file directly instead of the static library.
    if cfg!(target_os = "macos") {
        // Zig installs object files to zig-out/ (not zig-out/lib/)
        let obj_candidates = [
            root.join("zig-out").join("pledge_native.o"),
            root.join("zig-out").join("lib").join("pledge_native.o"),
            root.join("zig-out").join("obj").join("pledge_native.o"),
        ];
        let obj_path = obj_candidates.iter().find(|p| p.exists());

        if let Some(obj) = obj_path {
            println!("cargo:rustc-link-arg={}", obj.display());
        } else {
            // Fallback to static library if object file not found
            println!("cargo:rustc-link-search=native={}", lib_dir.display());
            println!("cargo:rustc-link-lib=static=pledge_native");
        }
    } else {
        // Tell cargo to look for the static library
        println!("cargo:rustc-link-search=native={}", lib_dir.display());
        println!("cargo:rustc-link-lib=static=pledge_native");
    }

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
