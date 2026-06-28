# Guia de Execução

## Objetivo

Este guia mostra como compilar, testar e executar localmente o workspace atual do AirLLM v3.0.

## Pré-requisitos

- Rust
- Python 3.11+
- Ollama rodando em `http://localhost:11434`
- Modelos locais disponíveis, de preferência:
  - `qwen3.5:4b`
  - `qwen3.6:27b`

## 1. Compilar o workspace

```bash
cargo build --workspace
```

## 2. Rodar os testes

```bash
cargo test --workspace
```

## 3. Listar modelos pelo AirLLM

```bash
cargo run -p airllm-cli -- models
```

## 4. Rodar um chat

```bash
cargo run -p airllm-cli -- chat --prompt "Responda exatamente OK" --model qwen3.5:4b
```

## 5. Rodar uma geração de código

```bash
cargo run -p airllm-cli -- code \
  "Escreva uma função Rust compacta add(a: i32, b: i32) -> i32 em src/lib.rs. Retorne só código se possível." \
  --language rust \
  --output src/lib.rs \
  --model qwen3.5:4b
```

## 6. Subir o servidor MCP

```bash
cargo run -p airllm-mcp
```

Exemplo de chamada via stdio:

```bash
printf '{"tool":"list_models","args":{}}\n' | cargo run -q -p airllm-mcp
```

## 7. Usar os bindings Python

```bash
PYTHONPATH=python python3 - <<'PY'
from airllm import Orchestrator
orch = Orchestrator("http://localhost:11434")
print(orch.list_models())
PY
```

Opcional: preaquecer modelos antes de uso interativo:

```bash
PYTHONPATH=python python3 - <<'PY'
from airllm import Orchestrator
orch = Orchestrator("http://localhost:11434")
print(orch.prewarm_models(["qwen3.5:4b"]))
PY
```

## 8. Reproduzir o benchmark local de Qwen

Consulte [BENCHMARK_OLLAMA_LOCAL_MODELS_2026-06-27.md](BENCHMARK_OLLAMA_LOCAL_MODELS_2026-06-27.md) para a metodologia e os resultados registrados.

## Sequência recomendada de validação

1. `cargo build --workspace`
2. `cargo test --workspace`
3. `cargo run -p airllm-cli -- models`
4. `cargo run -p airllm-cli -- chat ...`
5. `cargo run -p airllm-cli -- code ...`
6. `cargo run -p airllm-mcp`
7. `PYTHONPATH=python python3 ...`