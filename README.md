# AirLLM v3.0

AirLLM is now a Rust-first multi-agent coding workspace powered by Ollama, while the legacy Python inference package remains in this repository for compatibility and historical reference.

Portuguese overview: [docs/README_pt-br.md](docs/README_pt-br.md)

## Current Status

- `airllm-ollama`: async Ollama client, model router, tests, clippy, and docs validated
- `airllm-orchestrator`: real modular orchestrator with prompts and agent configs
- `airllm-cli`, `airllm-mcp`, and `airllm-python`: integrated and validated locally
- Local Ollama tested with `qwen3.5:4b` and `qwen3.6:27b`
- Benchmark documented in [docs/BENCHMARK_OLLAMA_LOCAL_MODELS_2026-06-27.md](docs/BENCHMARK_OLLAMA_LOCAL_MODELS_2026-06-27.md)
- v2-v3 technology comparison documented in [docs/BENCHMARK_V2_V3_STACK_2026-06-27.md](docs/BENCHMARK_V2_V3_STACK_2026-06-27.md)
- consolidated validation report documented in [docs/TEST_REPORT_2026-06-27.md](docs/TEST_REPORT_2026-06-27.md)
- Runtime optimization round applied: keep-alive, model-list cache, explicit prewarm, and simple-code fast-path

## Repository Layout

```text
.
├── crates/
│   ├── airllm-ollama/
│   ├── airllm-orchestrator/
│   ├── airllm-cli/
│   ├── airllm-mcp/
│   └── airllm-python/
├── python/airllm/
├── docs/
├── air_llm/
└── agentes-development/
```

## Quick Start

### Prerequisites

- Rust toolchain
- Python 3.11+
- Ollama running on `http://localhost:11434`
- At least one local model installed, such as `qwen3.5:4b`

### Build and test

```bash
cargo build --workspace
cargo test --workspace
```

### List local models through AirLLM

```bash
cargo run -p airllm-cli -- models
```

### Run a chat request

```bash
cargo run -p airllm-cli -- chat --prompt "Reply exactly OK" --model qwen3.5:4b
```

### Run a code request

```bash
cargo run -p airllm-cli -- code \
  "Write a compact Rust function add(a: i32, b: i32) -> i32 for src/lib.rs. Return code only if possible." \
  --language rust \
  --output src/lib.rs \
  --model qwen3.5:4b
```

### Run the MCP server

```bash
cargo run -p airllm-mcp
```

### Use the Python bindings

```bash
PYTHONPATH=python python3 - <<'PY'
from airllm import Orchestrator
orch = Orchestrator("http://localhost:11434")
print(orch.list_models())
PY
```

## Benchmark Snapshot

| Model | Chat direct | Code direct |
|---|---:|---:|
| `qwen3.5:4b` | 4.778s | 7.121s |
| `jaahas/crow:9b` | 10.773s | 4.081s |
| `qwen3.6:27b` | 46.775s | 101.787s |
| `granite4.1:30b` | 15.767s | 18.304s |
| `nemotron-3-nano:30b` | 32.761s | 12.437s |
| `qwen3-coder-next:q8_0` | 50.908s | 81.470s |

Full report: [docs/BENCHMARK_OLLAMA_LOCAL_MODELS_2026-06-27.md](docs/BENCHMARK_OLLAMA_LOCAL_MODELS_2026-06-27.md)

Technology comparison vs the legacy Python-era control plane: [docs/BENCHMARK_V2_V3_STACK_2026-06-27.md](docs/BENCHMARK_V2_V3_STACK_2026-06-27.md)

## Test and Benchmark Index

- Benchmark index: [benchmark_results/README.md](benchmark_results/README.md)
- Java calculator benchmark: [benchmark_results/benchmark_report.md](benchmark_results/benchmark_report.md)
- Java battery report: [benchmark_results/java_battery_report.md](benchmark_results/java_battery_report.md)
- Java calculator raw data: [benchmark_results/benchmark_results.json](benchmark_results/benchmark_results.json)
- Java battery raw data: [benchmark_results/java_battery_results.json](benchmark_results/java_battery_results.json)
- Benchmark runner (calculator): [benchmark_results/run_benchmark.py](benchmark_results/run_benchmark.py)
- Benchmark runner (battery): [benchmark_results/run_java_battery.py](benchmark_results/run_java_battery.py)

Current Java battery highlights:

- Best `hello world` model: [qwen2.5-coder:14b result](benchmark_results/java_battery_report.md)
- Best `calculator` model: [codegemma:7b result](benchmark_results/java_battery_report.md)
- Best overall average in the current Java battery: [codegemma:7b result](benchmark_results/java_battery_report.md)

## Documentation Index

- Roadmap: [docs/ROADMAP_PARALELO_3_FRENTES.md](docs/ROADMAP_PARALELO_3_FRENTES.md)
- Revised plan: [docs/PLANO_REVISADO_V3.md](docs/PLANO_REVISADO_V3.md)
- Run guide: [docs/RUN_GUIDE.md](docs/RUN_GUIDE.md)
- Maintenance guide: [docs/MAINTENANCE_GUIDE.md](docs/MAINTENANCE_GUIDE.md)
- Local-model benchmark: [docs/BENCHMARK_OLLAMA_LOCAL_MODELS_2026-06-27.md](docs/BENCHMARK_OLLAMA_LOCAL_MODELS_2026-06-27.md)
- v2-v3 comparison benchmark: [docs/BENCHMARK_V2_V3_STACK_2026-06-27.md](docs/BENCHMARK_V2_V3_STACK_2026-06-27.md)
- Consolidated test report: [docs/TEST_REPORT_2026-06-27.md](docs/TEST_REPORT_2026-06-27.md)
- Java benchmark index: [benchmark_results/README.md](benchmark_results/README.md)
- Java battery report: [benchmark_results/java_battery_report.md](benchmark_results/java_battery_report.md)
- Portuguese run guide: [docs/GUIA_EXECUCAO.md](docs/GUIA_EXECUCAO.md)
- Portuguese maintenance guide: [docs/GUIA_MANUTENCAO.md](docs/GUIA_MANUTENCAO.md)

## Validation Index

- Ollama client tests: [crates/airllm-ollama/tests/test_client.rs](crates/airllm-ollama/tests/test_client.rs)
- Router tests: [crates/airllm-ollama/tests/test_router.rs](crates/airllm-ollama/tests/test_router.rs)
- Stream tests: [crates/airllm-ollama/tests/test_stream.rs](crates/airllm-ollama/tests/test_stream.rs)
- Orchestrator tests: [crates/airllm-orchestrator/tests/test_orchestrator.rs](crates/airllm-orchestrator/tests/test_orchestrator.rs)
- Agent tests: [crates/airllm-orchestrator/tests/test_agents.rs](crates/airllm-orchestrator/tests/test_agents.rs)
- Decomposition tests: [crates/airllm-orchestrator/tests/test_decompose.rs](crates/airllm-orchestrator/tests/test_decompose.rs)
- Consolidation tests: [crates/airllm-orchestrator/tests/test_consolidate.rs](crates/airllm-orchestrator/tests/test_consolidate.rs)
- MCP tests: [crates/airllm-mcp/src/server.rs](crates/airllm-mcp/src/server.rs) and [crates/airllm-mcp/src/tools.rs](crates/airllm-mcp/src/tools.rs)
- Python binding smoke test: [crates/airllm-python/src/lib.rs](crates/airllm-python/src/lib.rs)

## Legacy Components

The following parts are preserved intentionally:

- [air_llm/README.md](air_llm/README.md): legacy AirLLM v2 Python package
- [training/README.md](training/README.md): legacy training notes
- [training/README_en.md](training/README_en.md): legacy English training notes

This cleanup pass only removes clearly obsolete top-level collateral and unused assets that are no longer referenced by the current top-level documentation.