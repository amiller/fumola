# Self-contained image for scheduler-svc.
#
# Stage 1 builds fumola from source. Stage 2 is a small python:3.12-slim
# image with the binary baked in plus the handler / server / UI.
#
# Build:
#   docker build -t scheduler-svc:local .
# Run:
#   docker run --rm -p 8080:8080 scheduler-svc:local
#
# Deploy to a tee-daemon CVM (already supports the `dockerfile` runtime):
#   curl -X POST $CVM/_api/projects \
#     -H "Authorization: Bearer $TOKEN" \
#     -H "Content-Type: application/json" \
#     -d '{"name":"scheduler-svc","source":"https://github.com/<you>/scheduler-svc","ref":"main"}'
#
FROM rust:1.82-slim AS builder
RUN apt-get update && apt-get install -y --no-install-recommends git pkg-config && rm -rf /var/lib/apt/lists/*
RUN git clone --depth 1 https://github.com/Adapton/fumola.git /src
WORKDIR /src
RUN cargo build --release -p fumola

FROM python:3.12-slim
RUN pip install --no-cache-dir aiohttp
COPY --from=builder /src/target/release/fumola /usr/local/bin/fumola
RUN chmod +x /usr/local/bin/fumola
WORKDIR /app
COPY server.py handle.fumola index.html ./
EXPOSE 8080
ENTRYPOINT ["python", "/app/server.py"]
