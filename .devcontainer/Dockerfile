FROM ubuntu:22.04

# Install basic tools
RUN apt-get update && apt-get install -y \
    build-essential \
    git \
    curl \
    vim \
    sudo \
    gcc \
    clang \
    && rm -rf /var/lib/apt/lists/*

# Install Rust (latest stable) and Cargo
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \
    | sh -s -- -y --default-toolchain stable

# Make Rust binaries available in PATH
ENV PATH="/root/.cargo/bin:${PATH}"

# Set default shell
SHELL ["/bin/bash", "-c"]