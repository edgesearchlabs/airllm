# Maintenance Guide

## Scope

This guide explains how to maintain the AirLLM v3.0 Rust workspace and when to touch each area of the repository.

## Repository ownership

- `crates/airllm-ollama`: Ollama transport, model discovery, routing heuristics
- `crates/airllm-orchestrator`: orchestration, prompts, agent configs, decomposition, consolidation
- `crates/airllm-cli`: command surface and TUI
- `crates/airllm-mcp`: MCP stdio server
- `crates/airllm-python`: PyO3 layer
- `python/airllm`: Python package import surface
- `air_llm/`: legacy v2 codebase, preserved separately

## Change workflow

### When changing model routing

Update:

- `crates/airllm-ollama/src/router.rs`
- `crates/airllm-orchestrator/agents/*.toml` if agent defaults also change
- benchmark documentation if latency or quality assumptions change

### When changing agents or prompts

Update:

- `crates/airllm-orchestrator/prompts/*.md`
- `crates/airllm-orchestrator/agents/*.toml`
- orchestrator tests if behavior or model selection rules change

### When changing CLI behavior

Update:

- `crates/airllm-cli/src/main.rs`
- `crates/airllm-cli/src/commands/*`
- `crates/airllm-cli/src/tui/*` when streaming UI changes
- README and run guides if command usage changes

### When changing MCP behavior

Update:

- `crates/airllm-mcp/src/server.rs`
- `crates/airllm-mcp/src/tools.rs`
- MCP validation examples in documentation

### When changing Python bindings

Update:

- `crates/airllm-python/src/lib.rs`
- `python/airllm/__init__.py`
- `python/airllm/__init__.pyi`

## Validation checklist

Run the narrowest relevant checks first, then the full workspace when the change crosses crate boundaries.

### Core validation

```bash
cargo build --workspace
cargo test --workspace
```

### Linting

```bash
cargo clippy --workspace --all-targets -- -D warnings
```

### Runtime validation

```bash
cargo run -p airllm-cli -- models
cargo run -p airllm-cli -- chat --prompt "Reply exactly OK" --model qwen3.5:4b
cargo run -p airllm-cli -- code "Write a Rust add function" --language rust --output src/lib.rs --model qwen3.5:4b
printf '{"tool":"list_models","args":{}}\n' | cargo run -q -p airllm-mcp
PYTHONPATH=python python3 - <<'PY'
from airllm import Orchestrator
print(Orchestrator("http://localhost:11434").list_models())
PY
```

## Documentation policy

- Keep the top-level English overview in `README.md`
- Keep the Portuguese overview in `docs/README_pt-br.md`
- Keep runnable instructions in both `docs/RUN_GUIDE.md` and `docs/GUIA_EXECUCAO.md`
- Keep maintenance instructions in both `docs/MAINTENANCE_GUIDE.md` and `docs/GUIA_MANUTENCAO.md`
- Update benchmark docs whenever timing claims are added or changed

## Cleanup policy

- Remove only files that are clearly unreferenced or obsolete for the current top-level product surface
- Preserve legacy assets still referenced by preserved legacy docs
- Re-run repository-wide reference search before deleting images or documentation collateral

## Release preparation

Before cutting a release or milestone:

1. Run workspace build, tests, and clippy
2. Re-run a local Ollama smoke test with the default local model
3. Verify MCP ready payload is valid JSON
4. Verify `from airllm import Orchestrator` still works
5. Update roadmap, benchmark, and context bank if behavior changed