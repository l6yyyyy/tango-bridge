FROM rust:latest as builder

WORKDIR /app

# 只复制 Cargo.toml 和源代码，不复制 Cargo.lock
COPY Cargo.toml ./
COPY src ./src
COPY adb ./adb

# 直接构建，让 Cargo 自动生成兼容的 Cargo.lock
RUN cargo build --release

FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/tango_bridge /usr/local/bin/
COPY --from=builder /app/adb/linux/adb /usr/local/bin/adb

RUN chmod +x /usr/local/bin/adb

ENV ADB_MDNS_OPENSCREEN=1

EXPOSE 15037/tcp

CMD ["tango_bridge"]
