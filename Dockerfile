# Docker image for Pledgepack development
# Usage: docker run -it -v $(pwd):/app -p 3000:3000 pledgepack/dev

FROM rust:1.85-slim AS builder

# Install Zig for native-sys compilation
RUN apt-get update && apt-get install -y --no-install-recommends \
    curl \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Install Zig
RUN curl -L https://ziglang.org/download/0.13.0/zig-linux-x86_64-0.13.0.tar.xz | tar -xJ -C /usr/local && \
    ln -s /usr/local/zig-linux-x86_64-0.13.0/zig /usr/local/bin/zig

WORKDIR /build

# Copy source
COPY . .

# Build the CLI binary
RUN cargo build --release --bin pledge

# Runtime image
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

# Copy the binary
COPY --from=builder /build/target/release/pledge /usr/local/bin/pledge

WORKDIR /app

EXPOSE 3000 4000

ENTRYPOINT ["pledge"]
CMD ["--help"]
