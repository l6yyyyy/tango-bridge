FROM rust:1.80-bookworm as builder

WORKDIR /app

# 复制必要文件
COPY Cargo.toml ./
COPY src ./src
COPY adb ./adb

# 生成新的 Cargo.lock 文件
RUN cargo fetch

# 构建
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
