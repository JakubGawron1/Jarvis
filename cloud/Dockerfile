# syntax=docker/dockerfile:1
FROM rust:1-bookworm AS build
WORKDIR /src
COPY jarvis /src/jarvis
COPY skills /src/skills
COPY vault /src/vault
COPY releases /src/releases
RUN cd /src/jarvis && cargo build -p jarvis-daemon --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY --from=build /src/jarvis/target/release/jarvisd /usr/local/bin/jarvisd
COPY skills /app/skills
COPY vault /app/vault
COPY releases /app/releases
ENV JARVIS_ROOT=/app
ENV JARVIS_KIND=cloud
# Do not set JARVIS_BIND: Render injects PORT (usually 10000). Local docker without PORT binds 0.0.0.0:7420.
EXPOSE 7420
CMD ["jarvisd"]
