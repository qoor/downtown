# Generic Rust builder for multiple architectures.
#
# This provides a build environment for building Rust codes.
# All flows in this image should be generic.
# You cannot define architecture dependent behaviors.
FROM --platform=${BUILDPLATFORM} rust:slim-bullseye as builder

ENV DEBIAN_FRONTEND=noninteractive \
    LANG=C.UTF-8 \
    TERM=xterm-256color \
    CARGO_HOME="/root/.cargo" \
    USER="root"

# Create CARGO_HOME folder and don't download rust docs
RUN mkdir -pv "${CARGO_HOME}" \
    && rustup set profile minimal

# Install required dependencies
RUN apt-get update \
    && apt-get install -y --no-install-recommends \
    # These should always on top.
    # After adding the amd64 architecture, the arm64 version of `apt` gives
    # package dependency errors when installing packages for cross compilation.
    gcc-x86-64-linux-gnu libc6-dev-amd64-cross \
    gcc-aarch64-linux-gnu libc6-dev-arm64-cross \
    # Add the architectures we are targeting
    && dpkg --add-architecture amd64 \
    && dpkg --add-architecture arm64 \
    # Install the openssl library with headers
    && apt-get update && apt-get install -y \
    libssl-dev:amd64 libssl-dev:arm64 \
    && apt-get clean \
    && rm -rf /var/lib/apt/lists/* \
    #
    # Make sure cargo has the right target config
    && echo '[target.x86_64-unknown-linux-gnu]' >> "${CARGO_HOME}/config" \
    && echo 'linker = "x86_64-linux-gnu-gcc"' >> "${CARGO_HOME}/config" \
    && echo 'rustflags = ["-L/usr/lib/x86_64-linux-gnu"]' >> "${CARGO_HOME}/config" \
    #
    && echo '[target.aarch64-unknown-linux-gnu]' >> "${CARGO_HOME}/config" \
    && echo 'linker = "aarch64-linux-gnu-gcc"' >> "${CARGO_HOME}/config" \
    && echo 'rustflags = ["-L/usr/lib/aarch64-linux-gnu"]' >> "${CARGO_HOME}/config"

RUN ls /usr/lib/x86_64-linux-gnu/
RUN ls /usr/lib/aarch64-linux-gnu/

RUN rustup target add \
    x86_64-unknown-linux-gnu \
    aarch64-unknown-linux-gnu

# Set platform specific environment values
ENV CC_x86_64_unknown_linux_gnu="/usr/bin/x86_64-linux-gnu-gcc" \
    CC_aarch64_unknown_linux_gnu="/usr/bin/aarch64-linux-gnu-gcc" \
    X86_64_UNKNOWN_LINUX_GNU_OPENSSL_INCLUDE_DIR="/usr/include/x86_64-linux-gnu" \
    X86_64_UNKNOWN_LINUX_GNU_OPENSSL_LIB_DIR="/usr/lib/x86_64-linux-gnu" \
    AARCH64_UNKNOWN_LINUX_GNU_OPENSSL_INCLUDE_DIR="/usr/include/aarch64-linux-gnu" \
    AARCH64_UNKNOWN_LINUX_GNU_OPENSSL_LIB_DIR="/usr/lib/aarch64-linux-gnu" \
    SQLX_OFFLINE=true

WORKDIR /usr/src/app

# Creates a dummy project used to grab dependencies
RUN USER=root cargo init --bin

# Copies over *only* your manifests and build files
COPY ./Cargo.* ./

RUN cargo build --locked --release --target=x86_64-unknown-linux-gnu
RUN cargo build --locked --release --target=aarch64-unknown-linux-gnu

COPY ./ ./
RUN touch src/main.rs

RUN cargo build --locked --release --target=x86_64-unknown-linux-gnu
RUN cargo build --locked --release --target=aarch64-unknown-linux-gnu


#
# Runtime for amd64
#
FROM --platform=linux/amd64 debian:bullseye-slim as runtime-amd64
WORKDIR /app/

# Copy files needed for runtime
COPY --from=builder /usr/src/app/target/x86_64-unknown-linux-gnu/release/downtown ./
COPY --from=builder /usr/src/app/.env.prod ./.env
COPY --from=builder /usr/src/app/private_key.pem ./private_key.pem
COPY --from=builder /usr/src/app/public_key.pem ./public_key.pem


#
# Runtime for arm64
#
FROM --platform=linux/arm64 debian:bullseye-slim as runtime-arm64
WORKDIR /app/

# Copy files needed for runtime
COPY --from=builder /usr/src/app/target/aarch64-unknown-linux-gnu/release/downtown ./
COPY --from=builder /usr/src/app/.env.prod ./.env
COPY --from=builder /usr/src/app/private_key.pem ./private_key.pem
COPY --from=builder /usr/src/app/public_key.pem ./public_key.pem


#
# Runtime for a target architecture
#
# Must be built via docker buildx.
#
FROM runtime-${TARGETARCH}

# Install required dependencies
RUN apt-get update \
    && apt-get install -y --no-install-recommends \
    ca-certificates \
    openssl \
    curl \
    && apt-get clean \
    && rm -rf /var/lib/apt/lists/*

ENV PORT=3000 \
    MYSQL_HOST=localhost \
    MYSQL_PORT=3306 \
    MYSQL_USERNAME=downtown \
    MYSQL_PASSWORD= \
    MYSQL_DATABASE=downtown \
    RSA_PRIVATE_PEM_FILE_PATH=private_key.pem \
    RSA_PUBLIC_PEM_FILE_PATH=public_key.pem \
    ACCESS_TOKEN_MAX_AGE=1800 \
    REFRESH_TOKEN_MAX_AGE=31536000

CMD [ "./downtown" ]
