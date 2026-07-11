# =============================================================================
# Builder Stage: Compile the Rust application and WASM assets
# =============================================================================
FROM rust:1.96-bookworm AS builder

# Install wasm32 target for WASM compilation
RUN rustup target add wasm32-unknown-unknown

# Install Dioxus CLI matching the project dependency (0.7.9)
RUN cargo install dioxus-cli --version 0.7.9 --locked

# Set working directory
WORKDIR /app

# Copy Cargo files to cache dependencies
COPY Cargo.toml Cargo.lock ./

# Create a dummy main.rs to cache server dependencies (speeds up subsequent builds)
RUN mkdir src && \
    echo "fn main() {}" > src/main.rs && \
    cargo build --release --features server && \
    rm -rf src

# Copy the actual source code
COPY src ./src

# Build the frontend WASM and static assets
RUN dx build --release --platform web

# Build the backend server binary
RUN cargo build --release --features server

# =============================================================================
# Runtime Stage: Minimal image for production
# =============================================================================
FROM debian:bookworm-slim

# Install ca-certificates (crucial for outbound HTTPS Google OAuth calls)
RUN apt-get update && \
    apt-get install -y ca-certificates && \
    rm -rf /var/lib/apt/lists/*

# Set working directory
WORKDIR /app

# Copy the compiled server binary from builder stage
COPY --from=builder /app/target/release/taffeite /app/taffeite

# Copy the generated web assets from the builder stage
COPY --from=builder /app/target/dx/taffeite/release/web/public /app/public

# Expose port 443
EXPOSE 443

# Set environment variable for public directory so the server can serve assets
ENV PUBLIC_DIR=/app/public

# Run the server binary
CMD ["/app/taffeite"]
