FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    curl \
    unzip \
    && rm -rf /var/lib/apt/lists/*

# 下载预编译的 ADB
RUN curl -L -o /tmp/platform-tools.zip https://dl.google.com/android/repository/platform-tools-latest-linux.zip && \
    unzip /tmp/platform-tools.zip -d /tmp && \
    mv /tmp/platform-tools/adb /usr/local/bin/ && \
    chmod +x /usr/local/bin/adb && \
    rm -rf /tmp/platform-tools /tmp/platform-tools.zip

# 复制本地的 tango-bridge 二进制文件
COPY ./tango_bridge /usr/local/bin/
RUN chmod +x /usr/local/bin/tango_bridge

ENV ADB_MDNS_OPENSCREEN=1

EXPOSE 15037/tcp

CMD ["tango_bridge"]
