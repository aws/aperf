# syntax=docker/dockerfile:1
FROM amazonlinux:2023

WORKDIR /root/

# Set architecture variables
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
RUN dnf install -y jq tar perf tar gzip sudo  procps java-21-amazon-corretto-devel && \
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

# Install aperf with architecture detection
RUN source /root/.bashrc && \
    export APERF_VERSION="$(curl -s https://api.github.com/repos/aws/aperf/releases/latest | jq -r .tag_name)" && \
    echo https://github.com/aws/aperf/releases/download/$APERF_VERSION/aperf-$APERF_VERSION-$ARCH.tar.gz && \
    curl -s -L -o /opt/aperf.tar.gz https://github.com/aws/aperf/releases/download/$APERF_VERSION/aperf-$APERF_VERSION-$ARCH.tar.gz && \
    tar zxf /opt/aperf.tar.gz -C /opt/ --strip-components=1 && \
    rm /opt/aperf.tar.gz && \
    mv /opt/aperf /usr/bin/ && \
    chmod a+x /usr/bin/aperf

CMD ["/usr/bin/aperf", "--help"]
