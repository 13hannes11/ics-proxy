FROM rust:1.56 as builder
RUN apt-get update && apt-get install -y sqlite3 && rm -rf /var/lib/apt/lists/*
WORKDIR /usr/src/ics-proxy
COPY . .
RUN cd db && ./create_db.sh
RUN cargo install --path .

FROM debian:buster-slim
RUN apt-get update && apt-get install -y sqlite3 openssl ca-certificates && rm -rf /var/lib/apt/lists/*
WORKDIR app
COPY --from=builder /usr/local/cargo/bin/ics-proxy ./ics-proxy
COPY --from=builder /usr/src/ics-proxy/db db
COPY templates ./templates
CMD ["./ics-proxy"]
