FROM rust:latest as builder
RUN apt-get update && apt-get install -y sqlite3 && rm -rf /var/lib/apt/lists/*
RUN cargo install sqlx-cli
WORKDIR /usr/src/ics-proxy
COPY . .
RUN sqlx database create
RUN sqlx migrate run
RUN cargo install --path .

FROM debian:stable-slim
RUN apt-get update && apt-get install -y sqlite3 openssl ca-certificates && rm -rf /var/lib/apt/lists/*
WORKDIR app
COPY --from=builder /usr/local/cargo/bin/ics-proxy ./ics-proxy
COPY --from=builder /usr/src/ics-proxy/db db
COPY templates ./templates
CMD ["./ics-proxy"]
