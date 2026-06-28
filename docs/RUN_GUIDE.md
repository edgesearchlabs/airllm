# Run Guide

## Purpose

This guide describes how to build, test, and run the current AirLLM v3.0 workspace locally.

## Prerequisites

- Rust toolchain
- Python 3.11+
- Ollama running on `http://localhost:11434`
- Local models available, preferably:
  - `qwen3.5:4b`
  - `qwen3.6:27b`

## 1. Build the workspace

```bash
cargo build --workspace
```

## 2. Run the test suite

```bash
cargo test --workspace
```

## 3. List models through AirLLM

```bash
cargo run -p airllm-cli -- models
```

## 4. Run a chat request

```bash
cargo run -p airllm-cli -- chat --prompt "Reply exactly OK" --model qwen3.5:4b
```

## 5. Run a code request

```bash
cargo run -p airllm-cli -- code \
  "Write a compact Rust function add(a: i32, b: i32) -> i32 for src/lib.rs. Return code only if possible." \
  --language rust \
  --output src/lib.rs \
  --model qwen3.5:4b
```

## 6. Run the MCP server

```bash
cargo run -p airllm-mcp
```

Example request via stdio:

```bash
printf '{"tool":"list_models","args":{}}\n' | cargo run -q -p airllm-mcp
```

## 7. Use the Python bindings

```bash
PYTHONPATH=python python3 - <<'PY'
from airllm import Orchestrator
orch = Orchestrator("http://localhost:11434")
print(orch.list_models())
PY
```

Optional model prewarm:

```bash
PYTHONPATH=python python3 - <<'PY'
from airllm import Orchestrator
orch = Orchestrator("http://localhost:11434")
print(orch.prewarm_models(["qwen3.5:4b"]))
PY
```

## 8. Reproduce the local Qwen benchmark

See [BENCHMARK_OLLAMA_LOCAL_MODELS_2026-06-27.md](BENCHMARK_OLLAMA_LOCAL_MODELS_2026-06-27.md) for methodology and recorded results.

## Recommended validation sequence

1. `cargo build --workspace`
2. `cargo test --workspace`
3. `cargo run -p airllm-cli -- models`
4. `cargo run -p airllm-cli -- chat ...`
5. `cargo run -p airllm-cli -- code ...`
6. `cargo run -p airllm-mcp`
7. `PYTHONPATH=python python3 ...`