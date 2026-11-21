# The image the Linux packages are built in (`cargo xtask package linux-x64`): Debian 11, whose
# glibc 2.31 is the oldest the executables then need, so they run on Debian 11, Ubuntu 20.04 and
# every later distribution. Rust comes from rustup at the workspace's `rust-version`.
FROM debian:bullseye

# Debian 11 left long-term support in August 2026: its packages are kept for good on
# archive.debian.org (deb.debian.org no longer has all of them).
RUN sed -i -e 's|deb.debian.org|archive.debian.org|g' -e '/bullseye-updates/d' /etc/apt/sources.list \
 && apt-get update \
 && apt-get install -y --no-install-recommends build-essential pkg-config libasound2-dev libudev-dev curl ca-certificates \
 && rm -rf /var/lib/apt/lists/*

ARG RUST_VERSION
ENV RUSTUP_HOME=/usr/local/rustup PATH=/usr/local/cargo/bin:$PATH
RUN curl -sSf https://sh.rustup.rs | CARGO_HOME=/usr/local/cargo sh -s -- -y --no-modify-path --profile minimal --default-toolchain "$RUST_VERSION" \
 && chmod -R a+rX /usr/local/rustup /usr/local/cargo

# The crates' cache, a volume shared by every build; open to the developer's user id, which the
# builds run as.
RUN mkdir /cargo && chmod 1777 /cargo
ENV CARGO_HOME=/cargo HOME=/tmp
WORKDIR /work
