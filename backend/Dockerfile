# Builder stage
FROM rust:latest AS builder

WORKDIR /app

# Install necessary build dependencies in one layer
RUN cargo init --vcs none

# Copy manifests first for better caching
COPY Cargo.toml ./

# Install sqlx-cli
RUN cargo install sqlx-cli --no-default-features --features postgres

# Now copy the real source code
COPY . .

# Build the actual application
ENV SQLX_OFFLINE=true
RUN cargo build --release

# Runtime stage
FROM debian:bookworm-slim

WORKDIR /app

# Install necessary runtime dependencies in one layer
RUN apt-get update && \
    apt-get install -y --no-install-recommends \
    libpq5 \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Create directory structure
RUN mkdir -p /app/scripts /app/migrations

# Copy only necessary files from builder
COPY --from=builder /app/target/release/backend ./
COPY --from=builder /app/scripts/start.sh ./scripts/
COPY --from=builder /app/migrations ./migrations/
COPY --from=builder /usr/local/cargo/bin/sqlx /usr/local/bin/

# Make start script executable
RUN chmod +x ./scripts/start.sh

# Verify executable permissions and file existence
RUN ls -la /app/scripts/start.sh && \
    ls -la /app/backend && \
    which sqlx

# Set environment variables
ENV PATH="/usr/local/bin:${PATH}"

ENTRYPOINT ["./scripts/start.sh"]