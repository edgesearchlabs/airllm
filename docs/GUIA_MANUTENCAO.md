# Guia de Manutenção

## Escopo

Este guia explica como manter o workspace Rust do AirLLM v3.0 e quando mexer em cada área do repositório.

## Responsabilidade por área

- `crates/airllm-ollama`: transporte HTTP do Ollama, descoberta de modelos e heurísticas de roteamento
- `crates/airllm-orchestrator`: orquestração, prompts, agentes, decomposição e consolidação
- `crates/airllm-cli`: superfície de comandos e TUI
- `crates/airllm-mcp`: servidor MCP em stdio
- `crates/airllm-python`: camada PyO3
- `python/airllm`: superfície de import em Python
- `air_llm/`: base legada v2, preservada separadamente

## Fluxo de mudança

### Quando alterar o roteamento de modelos

Atualize:

- `crates/airllm-ollama/src/router.rs`
- `crates/airllm-orchestrator/agents/*.toml` se os defaults dos agentes mudarem
- a documentação de benchmark se a expectativa de latência ou qualidade mudar

### Quando alterar agentes ou prompts

Atualize:

- `crates/airllm-orchestrator/prompts/*.md`
- `crates/airllm-orchestrator/agents/*.toml`
- os testes do orchestrator se o comportamento ou a seleção de modelo mudarem

### Quando alterar a CLI

Atualize:

- `crates/airllm-cli/src/main.rs`
- `crates/airllm-cli/src/commands/*`
- `crates/airllm-cli/src/tui/*` quando a UI de streaming mudar
- README e guias de execução se a linha de comando mudar

### Quando alterar o MCP

Atualize:

- `crates/airllm-mcp/src/server.rs`
- `crates/airllm-mcp/src/tools.rs`
- os exemplos de validação MCP na documentação

### Quando alterar os bindings Python

Atualize:

- `crates/airllm-python/src/lib.rs`
- `python/airllm/__init__.py`
- `python/airllm/__init__.pyi`

## Checklist de validação

Rode primeiro os checks mais estreitos e depois o workspace completo quando a mudança atravessar fronteiras entre crates.

### Validação principal

```bash
cargo build --workspace
cargo test --workspace
```

### Lint

```bash
cargo clippy --workspace --all-targets -- -D warnings
```

### Validação de runtime

```bash
cargo run -p airllm-cli -- models
cargo run -p airllm-cli -- chat --prompt "Responda exatamente OK" --model qwen3.5:4b
cargo run -p airllm-cli -- code "Escreva uma função Rust add" --language rust --output src/lib.rs --model qwen3.5:4b
printf '{"tool":"list_models","args":{}}\n' | cargo run -q -p airllm-mcp
PYTHONPATH=python python3 - <<'PY'
from airllm import Orchestrator
print(Orchestrator("http://localhost:11434").list_models())
PY
```

## Política de documentação

- Mantenha a visão geral em inglês em `README.md`
- Mantenha a visão geral em português em `docs/README_pt-br.md`
- Mantenha instruções de execução em `docs/RUN_GUIDE.md` e `docs/GUIA_EXECUCAO.md`
- Mantenha instruções de manutenção em `docs/MAINTENANCE_GUIDE.md` e `docs/GUIA_MANUTENCAO.md`
- Atualize o benchmark sempre que houver nova alegação de desempenho

## Política de limpeza

- Remova somente arquivos claramente sem referência ou obsoletos para a superfície principal atual do produto
- Preserve assets legados que ainda sejam referenciados por documentação legada preservada
- Refaça a busca por referências antes de apagar imagens ou material de documentação

## Preparação para release

Antes de fechar um milestone ou release:

1. Rode build, testes e clippy do workspace
2. Refaça um smoke test local com o modelo padrão
3. Verifique que o payload inicial do MCP continua JSON válido
4. Verifique que `from airllm import Orchestrator` continua funcionando
5. Atualize roadmap, benchmark e context bank se o comportamento mudar