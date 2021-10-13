FROM rust:1.55-alpine AS chef 
RUN apk update && apk add --no-cache musl-dev
RUN cargo install cargo-chef 
WORKDIR /app

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --target x86_64-unknown-linux-musl --release --recipe-path recipe.json

COPY . .
RUN cargo build --target x86_64-unknown-linux-musl --release

FROM alpine AS runner
RUN addgroup -S runner && adduser -S runner -G runner

COPY --from=builder /app/target/x86_64-unknown-linux-musl/release/cookie-clicker-afk /usr/local/bin/app
USER runner
CMD ["/usr/local/bin/app"]