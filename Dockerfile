# Top-level wrapper for the examples/web-scheduler app.
#
# Lets a tee-daemon CVM deploy this fork directly:
#   curl -X POST $CVM/_api/projects \
#     -H "Authorization: Bearer $TOKEN" \
#     -H "Content-Type: application/json" \
#     -d '{"name":"scheduler-svc",
#          "source":"https://github.com/amiller/fumola",
#          "ref":"examples/web-scheduler"}'
#
# Builds fumola from the same repo (faster than git-cloning a second
# time), then assembles the runtime image with server + handler + UI.
# The actual app source lives in examples/web-scheduler/.
#
FROM rust:1.82-slim AS builder
RUN apt-get update && apt-get install -y --no-install-recommends pkg-config && rm -rf /var/lib/apt/lists/*
COPY . /src
WORKDIR /src
RUN cargo build --release -p fumola

FROM python:3.12-slim
RUN pip install --no-cache-dir aiohttp
COPY --from=builder /src/target/release/fumola /usr/local/bin/fumola
RUN chmod +x /usr/local/bin/fumola
WORKDIR /app
COPY examples/web-scheduler/server.py /app/server.py
COPY examples/web-scheduler/handle.fumola /app/handle.fumola
COPY examples/web-scheduler/index.html /app/index.html
EXPOSE 8080
ENTRYPOINT ["python", "/app/server.py"]
