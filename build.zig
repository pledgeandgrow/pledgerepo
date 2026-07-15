// Zig build script for the native I/O and graph layers
// Compiled as a static library that Rust links against via FFI

const std = @import("std");

pub fn build(b: *std.Build) void {
    const target = b.standardTargetOptions(.{});
    const optimize = b.standardOptimizeOption(.{});

    // ─── Static library: pledge_native ───
    const lib_mod = b.createModule(.{
        .root_source_file = b.path("native-sys/zig/lib.zig"),
        .target = target,
        .optimize = optimize,
        .link_libc = true,
    });

    const lib = b.addLibrary(.{
        .name = "pledge_native",
        .root_module = lib_mod,
        .linkage = .static,
    });

    // Export C ABI symbols
    lib.linker_allow_shlib_undefined = true;

    // Platform-specific linking
    // On Linux, io_uring would link additional libraries here

    b.installArtifact(lib);

    // ─── Tests ───
    const tests = b.addTest(.{
        .root_module = lib_mod,
    });

    const run_tests = b.addRunArtifact(tests);
    const test_step = b.step("test", "Run unit tests");
    test_step.dependOn(&run_tests.step);

    // ─── Benchmarks ───
    const bench_mod = b.createModule(.{
        .root_source_file = b.path("native-sys/zig/bench.zig"),
        .target = target,
        .optimize = .ReleaseFast,
    });

    const bench = b.addExecutable(.{
        .name = "pledge_bench",
        .root_module = bench_mod,
    });

    const run_bench = b.addRunArtifact(bench);
    const bench_step = b.step("bench", "Run benchmarks");
    bench_step.dependOn(&run_bench.step);
}
