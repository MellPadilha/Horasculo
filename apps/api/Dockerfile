FROM rust:1.90-bookworm AS build
WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY apps/api ./apps/api
RUN cargo build --locked --release -p horasculo-api

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=build /app/target/release/horasculo-api /usr/local/bin/horasculo-api
USER 65532:65532
ENTRYPOINT ["horasculo-api"]
