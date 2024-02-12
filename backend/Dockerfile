FROM rust:1-slim

WORKDIR /appv

RUN cargo init --lib --vcs none
COPY Cargo.toml Cargo.lock ./
RUN cargo install sqlx-cli --no-default-features --features postgres
RUN cargo build --release --lib 

COPY . .
ENTRYPOINT [ "./scripts/start.sh" ]

