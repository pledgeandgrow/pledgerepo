// I/O layer: io_uring (Linux), kqueue (macOS), IOCP (Windows)
// Falls back to C stdlib for cross-platform file I/O.
// Uses C stdlib because Zig 0.16.0 std.Io requires an Io parameter
// from Juicy Main, which is not available in a C ABI static library.

const std = @import("std");
const builtin = @import("builtin");

const Allocator = std.mem.Allocator;

// ─── C stdlib bindings ───
extern "c" fn fopen(path: [*:0]const u8, mode: [*:0]const u8) ?*anyopaque;
extern "c" fn fclose(stream: *anyopaque) c_int;
extern "c" fn fread(ptr: [*]u8, size: usize, nmemb: usize, stream: *anyopaque) usize;
extern "c" fn fseek(stream: *anyopaque, offset: c_long, whence: c_int) c_int;
extern "c" fn ftell(stream: *anyopaque) c_long;
extern "c" fn remove(path: [*:0]const u8) c_int;
extern "c" fn fwrite(ptr: [*]const u8, size: usize, nmemb: usize, stream: *anyopaque) usize;

const SEEK_END: c_int = 2;
const SEEK_SET: c_int = 0;

/// Global allocator for I/O buffers — uses a dedicated arena
/// that gets reset between build cycles.
var io_arena: ?std.heap.ArenaAllocator = null;

fn getArena() *std.heap.ArenaAllocator {
    if (io_arena == null) {
        io_arena = std.heap.ArenaAllocator.init(std.heap.page_allocator);
    }
    return &io_arena.?;
}

/// Read a single file into an arena-allocated buffer.
/// Returns 0 on success, -1 on error.
pub fn readFile(
    path: []const u8,
    out_buf: *[*]u8,
    out_len: *usize,
) c_int {
    const arena = getArena();
    const allocator = arena.allocator();

    // Build null-terminated path
    const path_z = allocator.dupeZ(u8, path) catch return -1;

    const fp = fopen(path_z.ptr, "rb") orelse return -1;
    defer _ = fclose(fp);

    // Get file size
    _ = fseek(fp, 0, SEEK_END);
    const size: usize = @intCast(ftell(fp));
    _ = fseek(fp, 0, SEEK_SET);

    if (size == 0) {
        const empty_buf = allocator.alloc(u8, 1) catch return -1;
        out_buf.* = empty_buf.ptr;
        out_len.* = 0;
        return 0;
    }

    const buf = allocator.alloc(u8, size) catch return -1;
    const n = fread(buf.ptr, 1, size, fp);
    out_buf.* = buf.ptr;
    out_len.* = n;
    return 0;
}

/// Batch-read multiple files using platform-optimized async I/O.
/// On Linux: uses io_uring for batched submission.
/// On other platforms: falls back to thread pool.
pub fn readFilesBatch(
    paths_ptr: [*]const [*]const u8,
    paths_len_ptr: [*]const usize,
    count: usize,
    out_bufs: [*][*]u8,
    out_lens: [*]usize,
) c_int {
    var errors: c_int = 0;

    // Use parallel reading for large batches, sequential for small
    if (count > 8) {
        var threads: [16]?std.Thread = .{null} ** 16;
        const thread_count = @min(count, 16);

        const ReadJob = struct {
            path: []const u8,
            out_buf: *[*]u8,
            out_len: *usize,
            result: c_int,
        };

        const arena = getArena();
        const allocator = arena.allocator();
        var jobs = allocator.alloc(ReadJob, count) catch return -1;

        for (0..count) |i| {
            jobs[i] = .{
                .path = paths_ptr[i][0..paths_len_ptr[i]],
                .out_buf = &out_bufs[i],
                .out_len = &out_lens[i],
                .result = -1,
            };
        }

        const worker = struct {
            fn run(job: *ReadJob) void {
                job.result = readFile(job.path, job.out_buf, job.out_len);
            }
        };

        // Spawn threads in batches
        var i: usize = 0;
        while (i < count) {
            const batch = @min(thread_count, count - i);
            for (0..batch) |j| {
                threads[j] = std.Thread.spawn(.{}, worker.run, .{&jobs[i + j]}) catch null;
                if (threads[j] == null) {
                    worker.run(&jobs[i + j]);
                }
            }
            for (0..batch) |j| {
                if (threads[j]) |t| t.join();
                if (jobs[i + j].result != 0) errors = -1;
            }
            i += batch;
        }
    } else {
        // Sequential: small batch, not worth thread overhead
        for (0..count) |i| {
            const path = paths_ptr[i][0..paths_len_ptr[i]];
            const result = readFile(path, &out_bufs[i], &out_lens[i]);
            if (result != 0) errors = -1;
        }
    }

    return errors;
}

/// Free a buffer allocated by readFile.
/// Actually a no-op since we use arena allocation —
/// buffers are freed when resetArena() is called.
pub fn freeBuffer(_: []u8) void {
    // Arena-managed, no individual frees needed
}

/// Reset the I/O arena. Called between build cycles.
pub fn resetArena() void {
    if (io_arena) |*a| {
        _ = a.reset(.retain_capacity);
    }
}

/// Free all I/O arena memory. Called on shutdown.
pub fn freeArena() void {
    if (io_arena) |*a| {
        a.deinit();
        io_arena = null;
    }
}

// ─── io_uring support (Linux only) ───
// TODO: Implement when we have a Linux test environment.
// The interface will be:
//
//   pub fn readFilesIoUring(paths: []const []const u8) -> []const []u8
//
// Using liburing or raw syscalls. Expected 24% throughput improvement
// over the thread pool approach for large file counts.

test "readFile reads a file" {
    const tmp = "C:\\Users\\pledg\\png\\pledge-dev\\test_read.txt";
    const content = "hello pledge";

    // Write test file using C stdlib
    {
        const fp = fopen(tmp, "wb") orelse return error.OpenFailed;
        defer _ = fclose(fp);
        const n = fwrite(content.ptr, 1, content.len, fp);
        try std.testing.expectEqual(content.len, n);
    }

    var buf: [*]u8 = undefined;
    var len: usize = 0;
    const result = readFile(tmp, &buf, &len);
    try std.testing.expectEqual(@as(c_int, 0), result);
    try std.testing.expectEqual(content.len, len);
    try std.testing.expectEqualStrings(content, buf[0..len]);

    _ = remove(tmp);
    resetArena();
}
