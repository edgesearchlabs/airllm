FROM node:22-slim AS frontend-builder
RUN npm install -g bun
WORKDIR /app/frontend
COPY frontend/ .
RUN bun install && bun run build

FROM rust:1.85-slim AS rust-builder
WORKDIR /app
COPY . .
RUN cargo build --workspace --release --bin bridge --bin airllm

FROM node:22-slim AS runtime
RUN apt-get update && apt-get install -y curl && rm -rf /var/lib/apt/lists/*
WORKDIR /app

# Copy Rust binaries
COPY --from=rust-builder /app/target/release/bridge /app/bridge
COPY --from=rust-builder /app/target/release/airllm /app/airllm

# Copy frontend
COPY --from=frontend-builder /app/frontend /app/frontend

# Copy launcher
COPY frontend/bin/openairllm-launch /app/openairllm-launch
RUN chmod +x /app/openairllm-launch

ENV OLLAMA_BASE_URL=http://localhost:11434
ENV AIRLLM_BRIDGE_ADDR=0.0.0.0:18080
ENV OPENAI_MODEL=qwen2.5-coder:14b

EXPOSE 18080

HEALTHCHECK --interval=30s --timeout=5s --retries=3 \
  CMD curl -s http://localhost:18080/health || exit 1

CMD ["/app/openairllm-launch"]