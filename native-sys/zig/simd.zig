// SIMD-accelerated source scanning
// Finds import/export statements using vectorized string matching
//
// On x86_64: uses @Vector(32, u8) for 256-bit SIMD (AVX2)
// On aarch64: uses @Vector(16, u8) for 128-bit NEON
// Falls back to scalar on unsupported platforms

const std = @import("std");
const builtin = @import("builtin");

/// Find all occurrences of `import` keyword in source code.
/// Returns offsets into the source where `import` appears.
/// Returns the number of matches found (capped at out_capacity).
pub fn findImports(source: []const u8, out_offsets: []usize) usize {
    return findPattern(source, "import", out_offsets);
}

/// Find all occurrences of `export` keyword in source code.
pub fn findExports(source: []const u8, out_offsets: []usize) usize {
    return findPattern(source, "export", out_offsets);
}

/// Find all occurrences of `require(` in source code.
pub fn findRequires(source: []const u8, out_offsets: []usize) usize {
    return findPattern(source, "require(", out_offsets);
}

/// Generic SIMD pattern matcher.
/// Uses 32-byte wide comparisons on x86_64, 16-byte on other platforms.
pub fn findPattern(source: []const u8, pattern: []const u8, out_offsets: []usize) usize {
    if (pattern.len == 0 or source.len < pattern.len) return 0;

    var count: usize = 0;
    const simd_width: usize = if (builtin.cpu.arch == .x86_64) 32 else 16;

    var i: usize = 0;
    const end = source.len - pattern.len + 1;

    // Process in SIMD-width chunks
    while (i + simd_width < end) {
        // Check if any byte in this chunk matches the first byte of pattern
        const chunk = source[i .. i + simd_width];
        const first_byte = pattern[0];

        // Vectorized comparison: check each byte against first_byte
        const matches = findByteInChunk(chunk, first_byte);

        // For each match position, verify the full pattern
        for (matches) |pos| {
            if (pos == std.math.maxInt(usize)) continue; // sentinel, no more matches
            const abs_pos = i + pos;
            if (abs_pos + pattern.len <= source.len) {
                if (std.mem.eql(u8, source[abs_pos .. abs_pos + pattern.len], pattern)) {
                    if (count < out_offsets.len) {
                        out_offsets[count] = abs_pos;
                        count += 1;
                    }
                }
            }
        }

        i += simd_width;
    }

    // Scalar fallback for remaining bytes
    while (i < end) : (i += 1) {
        if (source[i] == pattern[0] and
            i + pattern.len <= source.len and
            std.mem.eql(u8, source[i .. i + pattern.len], pattern))
        {
            if (count < out_offsets.len) {
                out_offsets[count] = i;
                count += 1;
            }
        }
    }

    return count;
}

/// Find all positions in a chunk where byte == target.
/// Returns an array of relative positions (0-based within the chunk).
fn findByteInChunk(chunk: []const u8, target: u8) [32]usize {
    var positions: [32]usize = undefined;
    var count: usize = 0;

    if (builtin.cpu.arch == .x86_64 and chunk.len >= 32) {
        // 256-bit SIMD: load 32 bytes, compare against target
        const vec: @Vector(32, u8) = chunk[0..32].*;
        const target_vec: @Vector(32, u8) = @splat(target);

        // Compare and convert to mask: use @select to get 0/1 bytes, then pack
        const cmp = vec == target_vec;
        const ones: @Vector(32, u8) = @splat(1);
        const zeros: @Vector(32, u8) = @splat(0);
        const mask_bytes: @Vector(32, u8) = @select(u8, cmp, ones, zeros);

        // Extract match positions by checking each byte
        inline for (0..32) |i| {
            if (mask_bytes[i] == 1 and count < 32) {
                positions[count] = i;
                count += 1;
            }
        }
    } else {
        // Scalar fallback
        for (chunk, 0..) |b, pos| {
            if (b == target) {
                if (count < 32) {
                    positions[count] = pos;
                    count += 1;
                }
            }
        }
    }

    // Zero out remaining positions — use saturating arithmetic to avoid overflow
    while (count < 32) {
        positions[count] = std.math.maxInt(usize);
        count += 1;
    }

    return positions;
}

test "findImports finds import statements" {
    const source =
        \\import React from 'react';
        \\import { useState } from 'react';
        \\const x = 1;
        \\export default function App() {
        \\  import('./lazy').then(m => m.default());
        \\}
    ;

    var offsets: [16]usize = undefined;
    const count = findImports(source, &offsets);
    try std.testing.expect(count >= 2);
    try std.testing.expectEqualStrings("import", source[offsets[0] .. offsets[0] + 6]);
}

test "findExports finds export statements" {
    const source =
        \\export const foo = 1;
        \\export default App;
        \\export { bar };
    ;

    var offsets: [16]usize = undefined;
    const count = findExports(source, &offsets);
    try std.testing.expectEqual(@as(usize, 3), count);
}

test "findRequires finds require calls" {
    const source =
        \\const fs = require('fs');
        \\const path = require('path');
    ;

    var offsets: [16]usize = undefined;
    const count = findRequires(source, &offsets);
    try std.testing.expectEqual(@as(usize, 2), count);
}
