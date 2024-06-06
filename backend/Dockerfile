FROM rust:latest

WORKDIR /app

RUN cargo init --vcs none
COPY Cargo.toml ./
RUN cargo install sqlx-cli --no-default-features --features postgres
COPY . .
ENV SQLX_OFFLINE=true
RUN cargo build --release 

ENTRYPOINT [ "./scripts/start.sh" ]

