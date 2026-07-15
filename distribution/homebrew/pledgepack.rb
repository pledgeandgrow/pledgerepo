# Homebrew formula for Pledgepack
# Install with: brew install pledgepack
# Or: brew tap pledgepack/tap && brew install pledgepack

class Pledgepack < Formula
  desc "A Rust+Zig bundler with incremental computation, WASM plugins, and Rollup-quality output"
  homepage "https://pledgepack.dev"
  url "https://github.com/pledgepack/pledgepack/archive/refs/tags/v0.1.0.tar.gz"
  sha256 "0000000000000000000000000000000000000000000000000000000000000000"
  license "MIT"
  head "https://github.com/pledgepack/pledgepack.git", branch: "main"

  # Build dependencies — Rust toolchain
  depends_on "rust" => :build

  # Link to Zig for native-sys compilation
  depends_on "zig" => :build

  def install
    system "cargo", "build", "--release", "--bin", "pledge"
    bin.install "target/release/pledge" => "pledge"
  end

  test do
    assert_match "pledge", shell_output("#{bin}/pledge --version")
  end
end
