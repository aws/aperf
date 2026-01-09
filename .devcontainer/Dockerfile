# syntax=docker/dockerfile:1
FROM amazonlinux:2023

# Set architecture variables
WORKDIR /root/
ARG ARCH
RUN if [ -z "$ARCH" ]; then ARCH=$(uname -m); fi && \
    echo "export ARCH=$ARCH" >> /root/.bashrc && \
    if [ "$ARCH" = "aarch64" ]; then echo "export ASYNC_ARCH='arm64'" >> /root/.bashrc; \
    elif [ "$ARCH" = "x86_64" ]; then echo "export ASYNC_ARCH='x64'" >> /root/.bashrc; \
    else echo "Unsupported architecture: $ARCH"; exit 1; \
    fi && \
    source /root/.bashrc && \
    echo "Detected architecture: $ARCH" && \
    echo "Using async-profiler for $ASYNC_ARCH"

# Requirements
RUN dnf install -y jq tar perf tar gzip sudo procps java-21-amazon-corretto-devel gcc nodejs npm git && \
    dnf clean all

# Async profiler installation
RUN source /root/.bashrc && \
    export ASPROF_URL="$(curl -s https://api.github.com/repos/async-profiler/async-profiler/releases/latest | \
        jq -r ".assets[] | select(.name | contains(\"linux-${ASYNC_ARCH}\") and (contains(\"debug\") | not)) | .browser_download_url")" && \
    echo $ASPROF_URL && \
    curl -s -L -o /tmp/async.tar.gz $ASPROF_URL && \
    mkdir -p /opt/async-profiler && \
    ls -alh /tmp/async.tar.gz && \
    tar -xzf /tmp/async.tar.gz -C /opt/async-profiler --strip-components=1 && \
    rm /tmp/async.tar.gz && \
    chmod -R a+x /opt/async-profiler/bin/* /opt/async-profiler/lib/* 

# Install Rust for building aperf from source
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y && \
    source ~/.cargo/env && \
    echo 'source ~/.cargo/env' >> /root/.bashrc

# Build and install APerf 
COPY . /opt/aperf-src/
RUN source ~/.cargo/env && \
    cd /opt/aperf-src && \
    cargo build --release && \
    cp target/release/aperf /usr/bin/ && \
    chmod a+x /usr/bin/aperf && \
    cd /root && \
    rm -rf /opt/aperf-src

CMD ["/usr/bin/aperf", "--help"]
