FROM alpine:latest AS builder

RUN apk add git curl pkgconfig gcc musl-dev rustup perl make
RUN rustup-init -t x86_64-unknown-linux-musl --default-toolchain nightly --profile minimal -y

COPY . /app
COPY .sqlx /app/.sqlx
WORKDIR /app

ENV SQLX_OFFLINE=true

RUN /root/.cargo/bin/cargo build -r

FROM alpine:latest

COPY --from=builder /app/target/release/evehypernet-rs /app/

WORKDIR /app/

CMD ["./evehypernet-rs"]