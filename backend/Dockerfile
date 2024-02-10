FROM rust:1-slim

WORKDIR /app

COPY ./scripts/entrypoint.sh ./scripts/entrypoint.sh

RUN cargo init --lib --vcs none
COPY Cargo.toml Cargo.lock ./
RUN cargo build --release --lib 

COPY . .
RUN touch src/lib.rs && cargo build  --release

CMD ["./scripts/entrypoint.sh"]
