# =============================================================================
# Builder Stage: Compile the Rust application and WASM assets
# =============================================================================
FROM rust:bookworm AS builder

# Install wasm32 target for WASM compilation
RUN rustup target add wasm32-unknown-unknown

# Install Dioxus CLI
RUN cargo install dioxus-cli --locked

# Set working directory
WORKDIR /app

# Copy Cargo files
COPY Cargo.toml Cargo.lock ./

# Create a dummy main.rs to cache dependencies
RUN mkdir src && \
    echo "fn main() {}" > src/main.rs && \
    cargo build --release --features server && \
    rm -rf src

# Copy the actual source code
COPY src ./src

# Build the application with release optimizations
# This compiles both the server binary and WASM frontend assets
RUN dx build --release

# =============================================================================
# Runtime Stage: Minimal image for production
# =============================================================================
FROM debian:bookworm-slim

# Install ca-certificates for HTTPS requests (required for Google OAuth and MongoDB)
RUN apt-get update && \
    apt-get install -y ca-certificates && \
    rm -rf /var/lib/apt/lists/*

# Create a non-root user for security
RUN useradd -m -u 1000 appuser

# Set working directory
WORKDIR /app

# Copy the compiled server binary from builder stage
COPY --from=builder /app/target/release/taffeite /app/taffeite

# Copy the generated web assets from the Dioxus build
# dx build --release places assets in target/dx/taffeite/release/web/public
COPY --from=builder /app/target/dx/taffeite/release/web/public /app/public

# Change ownership to non-root user
RUN chown -R appuser:appuser /app

# Switch to non-root user
USER appuser

# Expose the application port
EXPOSE 8080

# Set environment variable for public directory
ENV PUBLIC_DIR=/app/public

# Run the application
CMD ["/app/taffeite"]
