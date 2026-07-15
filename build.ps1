#!/usr/bin/env pwsh
# Build script for Pledge — compiles Zig native library and Rust crates
#
# Usage:
#   .\build.ps1              Debug build
#   .\build.ps1 release      Release build
#   .\build.ps1 zig          Build only the Zig native library
#   .\build.ps1 test         Run tests

param(
    [Parameter(Position = 0)]
    [ValidateSet("debug", "release", "zig", "test", "bench", "clean")]
    [string]$Mode = "debug"
)

$ErrorActionPreference = "Stop"
$ProjectRoot = Split-Path -Parent $MyInvocation.MyCommand.Path

Write-Host ""
Write-Host "  pledge build system" -ForegroundColor Cyan
Write-Host "  -------------------" -ForegroundColor DarkGray
Write-Host ""

switch ($Mode) {
    "zig" {
        Write-Host "  Building Zig native library..." -ForegroundColor Yellow
        Push-Location $ProjectRoot
        zig build -Doptimize=ReleaseFast
        Pop-Location
        Write-Host "  Done. Library at zig-out/libpledge_native.a" -ForegroundColor Green
    }

    "release" {
        Write-Host "  Building Zig native library (release)..." -ForegroundColor Yellow
        Push-Location $ProjectRoot
        zig build -Doptimize=ReleaseFast
        Pop-Location

        Write-Host "  Building Rust crates (release)..." -ForegroundColor Yellow
        cargo build --release

        Write-Host "  Done. Binary at target/release/pledge.exe" -ForegroundColor Green
    }

    "test" {
        Write-Host "  Running Zig tests..." -ForegroundColor Yellow
        Push-Location $ProjectRoot
        zig build test
        Pop-Location

        Write-Host "  Running Rust tests..." -ForegroundColor Yellow
        cargo test --workspace

        Write-Host "  All tests passed." -ForegroundColor Green
    }

    "bench" {
        Write-Host "  Running Zig benchmarks..." -ForegroundColor Yellow
        Push-Location $ProjectRoot
        zig build bench
        Pop-Location
    }

    "clean" {
        Write-Host "  Cleaning build artifacts..." -ForegroundColor Yellow
        Push-Location $ProjectRoot
        Remove-Item -Recurse -Force target -ErrorAction SilentlyContinue
        Remove-Item -Recurse -Force zig-out -ErrorAction SilentlyContinue
        Pop-Location
        Write-Host "  Done." -ForegroundColor Green
    }

    default {
        Write-Host "  Building Zig native library (debug)..." -ForegroundColor Yellow
        Push-Location $ProjectRoot
        zig build
        Pop-Location

        Write-Host "  Building Rust crates (debug)..." -ForegroundColor Yellow
        cargo build

        Write-Host "  Done. Binary at target/debug/pledge.exe" -ForegroundColor Green
    }
}

Write-Host ""
