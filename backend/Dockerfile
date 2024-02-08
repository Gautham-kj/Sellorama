FROM rust:1-slim

WORKDIR /app

COPY ./scripts/entrypoint.sh ./scripts/entrypoint.sh

RUN cargo init --lib --vcs none
COPY Cargo.toml Cargo.lock ./
# RUN export http_proxy="http://edcguest:edcguest@172.31.100.30:3128/"
# RUN export https_proxy="http://edcguest:edcguest@172.31.100.30:3128/"
# RUN export HTTP_PROXY="http://edcguest:edcguest@172.31.100.30:3128/"
# RUN export HTTPS_PROXY="http://edcguest:edcguest@172.31.100.30:3128/"
# CMD ["/bin/curl" ,"ifconfig.me"]
RUN cargo build --release --lib 

COPY . .
RUN touch src/lib.rs && cargo build  --release

CMD ["./scripts/entrypoint.sh"]
