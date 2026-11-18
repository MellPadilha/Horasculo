FROM rust:1.90-bookworm@sha256:3914072ca0c3b8aad871db9169a651ccfce30cf58303e5d6f2db16d1d8a7e58f AS build
WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY apps/api ./apps/api
RUN cargo build --locked --release -p horasculo-api

FROM debian:bookworm-slim@sha256:734a72b3d6142a71993b4bc580ca01d8ca687f77a2c422d96a961d3376b179bc
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=build /app/target/release/horasculo-api /usr/local/bin/horasculo-api
USER 65532:65532
ENTRYPOINT ["horasculo-api"]
