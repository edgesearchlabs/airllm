# Test Report — 2026-06-27

This report consolidates the validation work executed for AirLLM v3.0 during the current modernization cycle.

## Workspace Validation

- `cargo test --workspace` → PASS
- `cargo clippy --workspace --all-targets -- -D warnings` → PASS
- `cargo check -p airllm-orchestrator -p airllm-cli -p airllm-mcp -p airllm-python` → PASS

## Crate-Level Coverage

### `airllm-ollama`

- [crates/airllm-ollama/tests/test_client.rs](crates/airllm-ollama/tests/test_client.rs)
- [crates/airllm-ollama/tests/test_router.rs](crates/airllm-ollama/tests/test_router.rs)
- [crates/airllm-ollama/tests/test_stream.rs](crates/airllm-ollama/tests/test_stream.rs)

### `airllm-orchestrator`

- [crates/airllm-orchestrator/tests/test_agents.rs](crates/airllm-orchestrator/tests/test_agents.rs)
- [crates/airllm-orchestrator/tests/test_consolidate.rs](crates/airllm-orchestrator/tests/test_consolidate.rs)
- [crates/airllm-orchestrator/tests/test_decompose.rs](crates/airllm-orchestrator/tests/test_decompose.rs)
- [crates/airllm-orchestrator/tests/test_orchestrator.rs](crates/airllm-orchestrator/tests/test_orchestrator.rs)

### `airllm-cli`

- smoke parsing in [crates/airllm-cli/src/main.rs](crates/airllm-cli/src/main.rs)

### `airllm-mcp`

- tool presence and dispatch tests in [crates/airllm-mcp/src/server.rs](crates/airllm-mcp/src/server.rs) and [crates/airllm-mcp/src/tools.rs](crates/airllm-mcp/src/tools.rs)

### `airllm-python`

- import smoke test in [crates/airllm-python/src/lib.rs](crates/airllm-python/src/lib.rs)

## Runtime Validation

### Local Ollama

- model listing through CLI → PASS
- short chat with `qwen3.5:4b` → PASS
- short code generation with `qwen3.5:4b` → PASS
- MCP stdio `list_models` → PASS
- Python import surface → PASS

## Performance Validation

- local multi-model benchmark → [docs/BENCHMARK_OLLAMA_LOCAL_MODELS_2026-06-27.md](docs/BENCHMARK_OLLAMA_LOCAL_MODELS_2026-06-27.md)
- v2 Python-style vs v3 Rust stack comparison → [docs/BENCHMARK_V2_V3_STACK_2026-06-27.md](docs/BENCHMARK_V2_V3_STACK_2026-06-27.md)

## Optimization Validation

- `list_models()` cache in-process improvement measured
- explicit prewarm support measured on `qwen3.5:4b`
- fast-path for simple `code()` requests measured through the compiled CLI
