# Benchmark Local — Ollama Local Models (2026-06-27)

> Escopo: substituir o benchmark antigo focado apenas em Qwen por uma fotografia mais útil dos modelos locais não-cloud disponíveis no Ollama.
> Ambiente: `/home/eriktonon/airllm`

## Ambiente de Teste

- CPU: Intel Core i9-13900K
- Threads lógicas: 32
- RAM: 62 GiB
- GPU: NVIDIA GeForce RTX 5080 16 GB
- Ollama local: `http://localhost:11434`

## Modelos Medidos

- `qwen3.5:4b`
- `jaahas/crow:9b`
- `qwen3.6:27b`
- `granite4.1:30b`
- `nemotron-3-nano:30b`
- `qwen3-coder-next:q8_0`

## Metodologia

- Chamada direta ao endpoint `POST /api/chat`
- `stream=false`
- `temperature=0.0`
- 1 execução por modelo e por cenário
- Cenários:
  - `chat`: resposta curtíssima (`OK_SPEED`)
  - `code`: geração curta de uma função Rust `add(a, b) -> i32`

### Observação importante

O benchmark antigo baseado apenas em Qwen foi aposentado porque a superfície atual do projeto já usa e valida mais modelos locais no Ollama. O objetivo deste documento é representar melhor o estado atual do runtime local e da distribuição de agentes.

## Resultados — Chat curto

| Modelo | Wall (s) | Total (s) | Load (s) | Eval (s) | Eval count | Tok/s | Preview |
|---|---:|---:|---:|---:|---:|---:|---|
| `qwen3.5:4b` | 4.778 | 4.778 | 3.312 | 1.293 | 194 | 150.05 | `OK_SPEED` |
| `jaahas/crow:9b` | 10.773 | 10.773 | 10.244 | 0.428 | 41 | 95.73 | `OK_SPEED` |
| `qwen3.6:27b` | 46.775 | 46.775 | 7.844 | 38.093 | 213 | 5.59 | `OK_SPEED` |
| `granite4.1:30b` | 15.767 | 15.767 | 15.015 | 0.215 | 3 | 13.96 | `OK_SPEED` |
| `nemotron-3-nano:30b` | 32.761 | 32.761 | 29.362 | 2.517 | 73 | 29.00 | `OK_SPEED` |
| `qwen3-coder-next:q8_0` | 50.908 | 50.908 | 41.822 | 0.549 | 3 | 5.46 | `OK_SPEED` |

## Resultados — Código curto

Prompt usado:

```text
System: You are a concise coding assistant. Return only code.
User: Write a compact Rust function add(a: i32, b: i32) -> i32 for src/lib.rs. Return code only if possible.
```

| Modelo | Wall (s) | Total (s) | Load (s) | Eval (s) | Eval count | Tok/s | Preview |
|---|---:|---:|---:|---:|---:|---:|---|
| `qwen3.5:4b` | 7.121 | 7.107 | 6.267 | 0.727 | 109 | 149.92 | `fn add(a: i32, b: i32) -> i32 { a + b }` |
| `jaahas/crow:9b` | 4.081 | 4.067 | 2.382 | 1.553 | 149 | 95.94 | `fn add(a: i32, b: i32) -> i32 { a + b }` |
| `qwen3.6:27b` | 101.787 | 101.786 | 5.810 | 95.268 | 579 | 6.08 | `fn add(a: i32, b: i32) -> i32 { a + b }` |
| `granite4.1:30b` | 18.304 | 18.304 | 15.536 | 2.407 | 27 | 11.22 | `fn add(a: i32, b: i32) -> i32 { a + b }` |
| `nemotron-3-nano:30b` | 12.437 | 12.433 | 7.463 | 3.775 | 112 | 29.67 | `fn add(a: i32, b: i32) -> i32 { a + b }` |
| `qwen3-coder-next:q8_0` | 81.470 | 81.449 | 33.534 | 6.061 | 31 | 5.11 | `fn add(a: i32, b: i32) -> i32 { a + b }` |

## Leitura prática

- `qwen3.5:4b` continua sendo a melhor opção para ciclos rápidos locais.
- `jaahas/crow:9b` entrou como uma boa alternativa leve para documentação e tarefas intermediárias.
- `granite4.1:30b` e `nemotron-3-nano:30b` são úteis para diversificar reviewer, planner, debugger e security sem depender apenas de Qwen.
- `qwen3.6:27b` e `qwen3-coder-next:q8_0` continuam relevantes pela qualidade, mas têm custo alto de latência local.

## Novo mapa de agentes locais no Ollama

| Agente | Modelo padrão | Fallback |
|---|---|---|
| `coder` | `qwen3.6:27b` | `qwen3.5:4b` |
| `reviewer` | `granite4.1:30b` | `qwen3.6:27b` |
| `tester` | `qwen3.5:4b` | `jaahas/crow:9b` |
| `architect` | `qwen3-coder-next:q8_0` | `granite4.1:30b` |
| `debugger` | `nemotron-3-nano:30b` | `qwen3.6:27b` |
| `refactorer` | `qwen3.6:27b` | `granite4.1:30b` |
| `documenter` | `jaahas/crow:9b` | `qwen3.5:4b` |
| `planner` | `granite4.1:30b` | `qwen3.6:27b` |
| `security` | `nemotron-3-nano:30b` | `granite4.1:30b` |
| `performance` | `qwen3-coder-next:q8_0` | `granite4.1:30b` |

## Reprodutibilidade

Script de benchmark:

```bash
python3 scripts/benchmark_ollama_local_models.py
```

O script foi mantido no repositório para que os números possam ser regenerados após troca de hardware, modelos ou prompts.

## Otimizações aplicadas depois do benchmark base

Após o benchmark base, o projeto recebeu uma rodada curta de otimização no cliente Ollama e no orchestrator:

- `keep_alive="30m"` em chamadas `chat`
- cache local de `list_models()` com TTL curto no cliente
- `prewarm_model()` no cliente Ollama
- `prewarm_models()` no orchestrator e nos bindings Python
- fast-path no `code()` para requests simples com um único agente, evitando o caminho de execução paralela

### Medidas observadas após a otimização

#### 1. Repeated `list_models()` in-process

Medição via binding Python no mesmo processo:

| Chamada | Tempo (s) | Quantidade de modelos |
|---|---:|---:|
| 1 | 0.0111 | 11 |
| 2 | 0.0038 | 11 |
| 3 | 0.0032 | 11 |

Leitura prática: ganho aproximado de `3x` nas chamadas repetidas de descoberta de modelos no mesmo processo.

#### 2. `qwen3.5:4b` chat curto sem prewarm explícito

| Execução | Wall (s) | Load (s) | Eval (s) | Eval count |
|---|---:|---:|---:|---:|
| 1 | 4.778 | 3.355 | 1.265 | 187 |
| 2 | 1.509 | 0.115 | 1.264 | 187 |
| 3 | 1.443 | 0.083 | 1.250 | 187 |

#### 3. `qwen3.5:4b` chat curto com prewarm explícito

Prewarm executado via `Orchestrator.prewarm_models(['qwen3.5:4b'])`.

| Execução | Wall (s) | Load (s) | Eval (s) | Eval count |
|---|---:|---:|---:|---:|
| 1 | 1.512 | 0.095 | 1.256 | 187 |
| 2 | 1.497 | 0.102 | 1.261 | 187 |
| 3 | 1.463 | 0.088 | 1.255 | 187 |

Leitura prática: o prewarm derrubou a primeira chamada curta de aproximadamente `4.78s` para `1.51s`, uma redução da ordem de `68%` no tempo de parede inicial desse cenário.

#### 4. Fast-path no `code()` simples via CLI compilado

Comando medido:

```bash
target/debug/airllm code \
  "Write a compact Rust function add(a: i32, b: i32) -> i32 for src/lib.rs. Return code only if possible." \
  --language rust \
  --output src/lib.rs \
  --model qwen3.5:4b
```

| Execução | Wall (s) |
|---|---:|
| 1 | 1.011 |
| 2 | 0.943 |
| 3 | 0.904 |

Média observada: `0.953s`.

Leitura prática: o fast-path mantém o fluxo simples de geração de código abaixo de 1 segundo em caminho quente, com menor overhead de coordenação.