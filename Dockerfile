FROM ubuntu:24.04

ARG DEBIAN_FRONTEND=noninteractive

ENV RUSTUP_HOME=/usr/local/rustup \
    CARGO_HOME=/usr/local/cargo \
    PATH=/usr/local/cargo/bin:/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin \
    RUST_VERSION=nightly-2024-05-02

# Match the GitHub grading environment, including the QEMU 8.2 security updates.
RUN apt-get update && apt-get install -y \
    ca-certificates curl git build-essential pkg-config python3 wget xz-utils \
    'qemu-system-misc=1:8.2.2+ds-*' 'qemu-user=1:8.2.2+ds-*' && \
    apt-get clean

RUN python3 -c 'import subprocess; versions = [subprocess.check_output([command, "--version"], text=True) for command in ("qemu-system-riscv64", "qemu-riscv64")]; print("\n".join(versions)); assert all(version.split("version ", 1)[1].split()[0] == "8.2.2" for version in versions), "This course requires QEMU 8.2.2."'

WORKDIR /root
RUN set -eux; \
    curl --proto '=https' --tlsv1.2 -fsSL --retry 3 https://sh.rustup.rs -o /root/rustup-init; \
    chmod +x /root/rustup-init; \
    /root/rustup-init -y --no-modify-path --profile minimal \
        --default-toolchain "$RUST_VERSION" \
        --component rust-src,llvm-tools-preview,rustfmt,clippy \
        --target riscv64gc-unknown-none-elf; \
    rm /root/rustup-init; \
    cargo install cargo-binutils --version 0.4.0 --locked; \
    rustc --version; \
    cargo --version; \
    rust-objcopy --version

WORKDIR /mnt
CMD ["/bin/bash"]
