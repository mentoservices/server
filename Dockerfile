# --------------------
# 1️⃣ Build stage
# --------------------
FROM rust:latest as builder

WORKDIR /app

# Cache dependencies
COPY Cargo.toml Cargo.lock ./
RUN mkdir src && echo "fn main() {}" > src/main.rs
RUN cargo build --release
RUN rm -rf src

# Build actual app
COPY . .
RUN cargo build --release

# --------------------
# 2️⃣ Runtime stage
# --------------------
FROM debian:bookworm-slim

# Install required runtime libs
RUN apt-get update && apt-get install -y \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY --from=builder /app/target/release/your_app_name /app/app

# Rocket production vars
ENV ROCKET_PROFILE=release
ENV ROCKET_ADDRESS=0.0.0.0
ENV ROCKET_PORT=5050

EXPOSE 5050

CMD ["./app"]