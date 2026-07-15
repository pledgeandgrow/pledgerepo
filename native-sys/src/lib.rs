// pledge-native-sys: Rust FFI bindings to the Zig native library
// Links against libpledge_native.a (compiled from Zig source)

#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

use std::os::raw::{c_int, c_void};

// ─── Zig runtime symbol stubs ───
// These symbols are referenced by Zig's compiler-rt but not bundled
// in static libraries. MSVC's linker needs them resolved.

/// Stack canary guard value (Zig runtime expects this to exist)
#[unsafe(no_mangle)]
pub static __stack_chk_guard: u64 = 0xdeadbeef_cafebabe;

/// Called when stack canary check fails — abort the process
#[unsafe(no_mangle)]
pub extern "C" fn __stack_chk_fail() -> ! {
    std::process::abort();
}

// ___chkstk_ms: Proper stack probe implementation.
// Zig/LLVM calls this with the stack allocation size in RAX.
// It must touch each 4096-byte page from RSP down to RSP - RAX
// to ensure the OS commits the stack pages. RSP and RAX must be preserved.
#[cfg(target_arch = "x86_64")]
core::arch::global_asm!(
    ".globl ___chkstk_ms",
    "___chkstk_ms:",
    "mov r10, rax",           // r10 = remaining = size (r10 is volatile)
    "mov rcx, rsp",           // rcx = cursor = rsp (rcx is volatile)
    "cmp r10, 0x1000",        // if remaining < page_size, skip loop
    "jb 2f",
    "1:",                     // probe full pages
    "sub rcx, 0x1000",        // cursor -= 4096
    "test [rcx], al",         // touch page at cursor
    "sub r10, 0x1000",        // remaining -= 4096
    "cmp r10, 0x1000",        // more than a page left?
    "ja 1b",
    "2:",                     // probe final partial page
    "sub rcx, r10",           // cursor -= remaining
    "test [rcx], al",         // touch final page
    "ret",                    // rax preserved (never modified), rsp preserved
);

/// LdrRegisterDllNotification — Windows ntdll function for DLL load notifications
/// Not available in MSVC 2019 import libraries; provide a no-op stub
#[unsafe(no_mangle)]
pub extern "system" fn LdrRegisterDllNotification(
    _flags: u32,
    _callback: *const c_void,
    _context: *const c_void,
    _cookie: *mut c_void,
) -> i32 {
    // Return STATUS_SUCCESS (0)
    0
}

// Opaque pointer to the Zig ModuleGraph
pub type ModuleGraphHandle = *mut c_void;

unsafe extern "C" {
    // Graph operations
    pub fn pledge_graph_create() -> ModuleGraphHandle;
    pub fn pledge_graph_destroy(g: ModuleGraphHandle);
    pub fn pledge_graph_add_module(
        g: ModuleGraphHandle,
        path_ptr: *const u8,
        path_len: usize,
    ) -> u32;
    pub fn pledge_graph_add_dependency(g: ModuleGraphHandle, from: u32, to: u32);
    pub fn pledge_graph_get_dependents(
        g: ModuleGraphHandle,
        module_id: u32,
        out_ids: *mut u32,
        out_capacity: usize,
    ) -> usize;

    // I/O operations
    pub fn pledge_io_read_file(
        path_ptr: *const u8,
        path_len: usize,
        out_buf: *mut *mut u8,
        out_len: *mut usize,
    ) -> c_int;
    pub fn pledge_io_read_files_batch(
        paths_ptr: *const *const u8,
        paths_len_ptr: *const usize,
        count: usize,
        out_bufs: *mut *mut u8,
        out_lens: *mut usize,
    ) -> c_int;
    pub fn pledge_io_free(buf: *mut u8, len: usize);

    // SIMD scanning
    pub fn pledge_simd_find_imports(
        source_ptr: *const u8,
        source_len: usize,
        out_offsets: *mut usize,
        out_capacity: usize,
    ) -> usize;
}

/// RAII wrapper for ModuleGraph
pub struct Graph {
    handle: ModuleGraphHandle,
}

impl Graph {
    pub fn new() -> Self {
        unsafe {
            Self {
                handle: pledge_graph_create(),
            }
        }
    }

    pub fn add_module(&self, path: &str) -> u32 {
        unsafe {
            pledge_graph_add_module(
                self.handle,
                path.as_ptr(),
                path.len(),
            )
        }
    }

    pub fn add_dependency(&self, from: u32, to: u32) {
        unsafe { pledge_graph_add_dependency(self.handle, from, to) }
    }

    pub fn get_dependents(&self, module_id: u32, capacity: usize) -> Vec<u32> {
        let mut ids = vec![0u32; capacity];
        let count = unsafe {
            pledge_graph_get_dependents(
                self.handle,
                module_id,
                ids.as_mut_ptr(),
                capacity,
            )
        };
        ids.truncate(count);
        ids
    }
}

impl Drop for Graph {
    fn drop(&mut self) {
        unsafe { pledge_graph_destroy(self.handle) }
    }
}

// Safety: Graph is owned and not shared across threads unsafely
unsafe impl Send for Graph {}
unsafe impl Sync for Graph {}

/// Read a file using the Zig I/O layer
pub fn read_file(path: &str) -> anyhow::Result<Vec<u8>> {
    let mut buf_ptr: *mut u8 = std::ptr::null_mut();
    let mut buf_len: usize = 0;

    let result = unsafe {
        pledge_io_read_file(
            path.as_ptr(),
            path.len(),
            &mut buf_ptr as *mut *mut u8,
            &mut buf_len as *mut usize,
        )
    };

    if result != 0 {
        anyhow::bail!("Failed to read file: {}", path);
    }

    // Copy from Zig arena to Rust-owned Vec
    let data = unsafe {
        let slice = std::slice::from_raw_parts(buf_ptr, buf_len);
        slice.to_vec()
    };

    Ok(data)
}

/// Find import statements using SIMD-accelerated scanning
pub fn find_imports(source: &[u8]) -> Vec<usize> {
    let mut offsets = vec![0usize; 1024];
    let count = unsafe {
        pledge_simd_find_imports(
            source.as_ptr(),
            source.len(),
            offsets.as_mut_ptr(),
            offsets.len(),
        )
    };
    offsets.truncate(count);
    offsets
}
