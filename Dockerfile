FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    curl \
    && rm -rf /var/lib/apt/lists/*

# 下载预编译的 ADB
RUN curl -L -o /usr/local/bin/adb https://dl.google.com/android/repository/platform-tools-latest-linux.zip && \
    unzip /usr/local/bin/adb -d /tmp && \
    mv /tmp/platform-tools/adb /usr/local/bin/ && \
    chmod +x /usr/local/bin/adb && \
    rm -rf /tmp/platform-tools /usr/local/bin/adb.zip

# 下载预编译的 tango-bridge
RUN curl -L -o /usr/local/bin/tango_bridge https://github.com/tango-adb/bridge-rs/releases/latest/download/linux-x64

RUN chmod +x /usr/local/bin/tango_bridge

ENV ADB_MDNS_OPENSCREEN=1

EXPOSE 15037/tcp

CMD ["tango_bridge"]
