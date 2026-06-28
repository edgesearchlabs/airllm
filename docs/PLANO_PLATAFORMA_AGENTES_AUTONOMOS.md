# Plano — Plataforma de Agentes Autônomos AirLLM v4.0

> **Data**: 2026-06-28
> **Autor**: EdgeSearch Orquestrador
> **Revisor**: Erik Tonon
> **Status**: Rascunho para aprovação
> **Base**: AirLLM v3.0 (workspace Rust com `airllm-ollama`, `airllm-orchestrator`, `airllm-cli`, `airllm-mcp`, `airllm-python`)
> **Convenção**: `.github/copilot-instructions.md` (agentes `.agent.md`, skills `SKILL.md`, hooks em `.github/hooks/`, workflows em `.github/workflows/`)

---

## 1. Visão Geral

Transformar o AirLLM v3.0 (multi-agentes de codificação) em uma **plataforma de agentes autônomos** capaz de:

1. **Executar longos trabalhos sem interrupção** — agentes rodam em loop autônomo, avaliam resultados, decidem próximos passos
2. **Permissões granulares** — RBAC por agente, aprovação humana opcional para tools sensíveis
3. **Treinar e refinar modelos** — pipeline de fine-tuning LoRA/DPO por especialidade
4. **Especialistas por tema** — agentes carregam modelo fine-tuned + system prompt + tools permitidas
5. **Tools além código** — redes sociais, mensagens, email, webhooks, automações
6. **Automação disparada por eventos** — scheduler com cron + triggers externos

### Princípios

- **Rust puro** — sem runtime Node/Bun. Mantém o princípio zero-cost do AirLLM
- **Convenção EdgeSearch** — agentes em `.agent.md`, skills em `SKILL.md`, hooks em `.github/hooks/`
- **Incremental** — cada fase constrói sobre o workspace Rust existente que já compila
- **MCP-first** — todas as tools expostas via MCP para interoperabilidade com qualquer client
- **Segurança por design** — permissões obrigatórias antes de execução autônoma

---

## 2. Arquitetura

```
┌──────────────────────────────────────────────────────────────────┐
│                  PLATAFORMA DE AGENTES AUTÔNOMOS                  │
│                                                                   │
│  ┌─────────────┐  ┌──────────────┐  ┌──────────────────────────┐  │
│  │  Scheduler  │  │ Permission  │  │    Model Training         │  │
│  │  (cron +    │  │ Engine RBAC │  │    Pipeline (LoRA/DPO)    │  │
│  │   triggers) │  │             │  │                           │  │
│  └──────┬──────┘  └──────┬──────┘  └───────────┬──────────────┘  │
│         │                │                      │                 │
│  ┌──────▼────────────────▼──────────────────────▼──────────────┐  │
│  │              AGENT RUNTIME (Rust + tokio)                    │  │
│  │                                                              │  │
│  │  ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌──────────────────┐ │  │
│  │  │  Coder   │ │  Social  │ │ Message  │ │  Research       │ │  │
│  │  │  Agent   │ │  Media   │ │  Agent   │ │  Agent          │ │  │
│  │  │          │ │  Agent   │ │          │ │                  │ │  │
│  │  └──────────┘ └──────────┘ └──────────┘ └──────────────────┘ │  │
│  │                                                              │  │
│  │  Loop autônomo: executar → avaliar → decidir → repetir       │  │
│  └────────────────────────┬─────────────────────────────────────┘  │
│                           │                                       │
│  ┌────────────────────────▼─────────────────────────────────────┐  │
│  │                   TOOL LAYER (MCP)                           │  │
│  │  ┌────────┐ ┌────────┐ ┌─────────┐ ┌──────┐ ┌──────────────┐ │  │
│  │  │ GitHub │ │Twitter│ │WhatsApp│ │Email │ │ Webhook HTTP │ │  │
│  │  │ Code   │ │LinkedIn│ │Telegram│ │ SMTP │ │ Slack        │ │  │
│  │  └────────┘ └────────┘ └─────────┘ └──────┘ └──────────────┘ │  │
│  └────────────────────────┬─────────────────────────────────────┘  │
│                           │                                       │
│  ┌────────────────────────▼─────────────────────────────────────┐  │
│  │              MODEL LAYER (Ollama)                            │  │
│  │  qwen3.5:4b (draft)    │ qwen3.6:27b (coder)                 │  │
│  │  qwen3-coder-next:q8_0 │ modelos fine-tuned por especialidade │  │
│  └──────────────────────────────────────────────────────────────┘  │
│                                                                   │
│  ┌──────────────────────────────────────────────────────────────┐  │
│  │              DAEMON (systemd service)                        │  │
│  │  Processo contínuo: carrega agentes, escuta scheduler,        │  │
│  │  executa loop autônomo, persiste estado em SQLite             │  │
│  └──────────────────────────────────────────────────────────────┘  │
└──────────────────────────────────────────────────────────────────┘
```

---

## 3. Estrutura de Crates (workspace Cargo)

```
crates/
├── airllm-ollama/          # ✅ existe — cliente Ollama + router
├── airllm-orchestrator/     # ✅ existe — adicionar loop autônomo + especialistas
├── airllm-cli/             # ✅ existe — adicionar `airllm daemon`, `airllm agent run`
├── airllm-mcp/             # ✅ existe — adicionar tools de automação
├── airllm-python/          # ✅ existe
├── airllm-scheduler/       # 🆕 cron + triggers + eventos
├── airllm-permissions/     # 🆕 RBAC + approval queue + audit log
├── airllm-tools/           # 🆕 tool registry + tools de automação
│   └── src/tools/
│       ├── code.rs         # já existe no MCP
│       ├── social_media.rs # Twitter, LinkedIn, Instagram
│       ├── messaging.rs    # WhatsApp, Telegram, Slack, Discord
│       ├── email.rs        # SMTP via lettre
│       ├── webhook.rs      # HTTP genérico
│       └── file_ops.rs     # operações de arquivo seguras
├── airllm-training/        # 🆕 fine-tune pipeline (LoRA/DPO via Ollama)
├── airllm-daemon/          # 🆕 processo contínuo (systemd)
└── airllm-state/           # 🆕 persistência SQLite (estado de agentes, histórico, audit)
```

---

## 4. Agentes EdgeSearch (`.github/agents/*.agent.md`)

### 4.1 Agentes existentes (manter)

| Agente | Arquivo | Função na plataforma |
|---|---|---|
| `edgesearch-orquestrador` | `edgesearch-orquestrador.agent.md` | Coordena a plataforma — delega para especialistas |
| `edgesearch-cerebro` | `edgesearch-cerebro.agent.md` | Estado central — agora inclui estado de agentes autônomos |
| `edgesearch-tester` | `edgesearch-tester.agent.md` | Valida tools e agentes antes de deploy |
| `edgesearch-rust-backend` | `edgesearch-rust-backend.agent.md` | Implementa crates Rust da plataforma |
| `edgesearch-sre` | `edgesearch-sre.agent.md` | Dockerfile + systemd service para daemon |
| `edgesearch-documentacao` | `edgesearch-documentacao.agent.md` | Documenta plataforma |
| `edgesearch-recuperador` | `edgesearch-recuperador.agent.md` | Recupera outputs malformados |

### 4.2 Novos agentes (criar)

#### `edgesearch-agent-trainer`

```yaml
---
name: "EdgeSearch Agent Trainer"
description: "Especialista em treinar e refinar modelos LLM para uso especifico. Implementa pipelines de fine-tuning LoRA e DPO via Ollama. Ative quando: treinar modelo para especialidade, refinar comportamento de agente, criar modelo especialista em tema, avaliar qualidade de modelo fine-tuned."
tools: [read, search, edit, execute, todo]
user-invocable: true
---
```

**Responsabilidades**:
- Criar datasets de treino a partir de exemplos do usuário
- Orquestrar fine-tuning LoRA via Ollama (`ollama create` + Modelfile + adapter)
- Avaliar modelo fine-tuned vs base (benchmark comparativo)
- Versionar modelos fine-tuned (`qwen3.5:4b-social-v1`, `qwen3.5:4b-code-reviewer-v2`)
- Registrar decisões de treino no context bank

#### `edgesearch-tool-builder`

```yaml
---
name: "EdgeSearch Tool Builder"
description: "Especialista em criar tools de automacao para agentes. Implementa integracoes com redes sociais, mensageria, email, webhooks e APIs externas como tools MCP. Ative quando: criar nova tool de automacao, integrar servico externo como ferramenta de agente, expor API como tool MCP."
tools: [read, search, edit, execute, todo]
user-invocable: true
---
```

**Responsabilidades**:
- Implementar tools Rust seguindo trait `Tool` comum
- Integrar APIs externas (Twitter API, WhatsApp Business, Telegram Bot, Slack Webhook)
- Expor tools via MCP server (`airllm-mcp`)
- Documentar schema de input/output de cada tool
- Testar tools com mocks antes de expor

#### `edgesearch-permission-architect`

```yaml
---
name: "EdgeSearch Permission Architect"
description: "Especialista em modelar permissoes e aprovacoes para agentes autonomos. Define RBAC, politicas de aprovacao humana, audit trail e isolamento de agentes. Ative quando: modelar permissoes de agentes, configurar aprovacao humana para tools sensiveis, auditar acoes de agentes autonomos."
tools: [read, search, edit, execute, todo]
user-invocable: true
---
```

**Responsabilidades**:
- Modelar RBAC (roles, permissions, agents)
- Definir políticas de aprovação (auto, require-human, deny)
- Implementar audit trail (SQLite — quem, quando, o quê, resultado)
- Configurar isolamento de agentes (sandbox de filesystem, network policy)
- Documentar matriz de permissões

#### `edgesearch-scheduler-architect`

```yaml
---
name: "EdgeSearch Scheduler Architect"
description: "Especialista em agendamento e triggers para agentes autonomos. Implementa cron jobs, webhooks triggers, eventos de sistema e filas de execucao. Ative quando: agendar execucao recorrente de agente, configurar trigger por webhook, criar fila de tarefas para agentes."
tools: [read, search, edit, execute, todo]
user-invocable: true
---
```

**Responsabilidades**:
- Implementar scheduler cron (cron expression + timezone)
- Configurar triggers externos (webhook HTTP, GitHub webhook, calendário)
- Criar fila de execução (prioridade, retry, backoff)
- Integrar com `airllm-daemon` para execução contínua
- Monitorar jobs em execução (status, duração, resultado)

#### `edgesearch-autonomous-runner`

```yaml
---
name: "EdgeSearch Autonomous Runner"
description: "Especialista em loops de execucao autonoma de agentes. Implementa o ciclo executar-avaliar-decidir-repetir com checkpoints, recuperacao de falhas e persistencia de estado. Ative quando: configurar agente para rodar autonomamente, implementar loop de execucao continua, adicionar recuperacao de falhas em agente."
tools: [read, search, edit, execute, todo]
user-invocable: true
---
```

**Responsabilidades**:
- Implementar loop autônomo (execute → evaluate → decide → repeat)
- Adicionar checkpoints (salvar estado a cada N passos)
- Recuperação de falhas (resume do último checkpoint)
- Timeout e cancelamento graceful
- Logging de decisões do agente (por que escolheu X sobre Y)

---

## 5. Skills EdgeSearch (`.github/skills/*/SKILL.md`)

### 5.1 Skills existentes (manter)

- `lifecycle-hooks` — protocolo de hooks (adaptar para incluir hooks de autonomia)
- `git-workflow` — padrão de commits
- `project-context-bank` — context bank
- `decisao-linguagem` — seleção de linguagem
- `scaffold-rust-api` — scaffold Rust
- `dockerfile-review` — revisão de Dockerfile
- `revisao-owasp` — segurança
- `security-lib-scan` — scan de libs

### 5.2 Novas skills (criar)

#### `scaffold-autonomous-agent`

```yaml
---
name: scaffold-autonomous-agent
description: "Gera a estrutura completa de um agente autonomo: crate Rust com loop de execucao, config TOML, system prompt, tools permitidas, modelo associado e permissoes. Use quando: criar novo agente especialista, configurar agente para rodar autonomamente."
user-invocable: true
---
```

**Output**: crate Rust + config TOML + `.agent.md` + system prompt + testes

#### `scaffold-mcp-tool`

```yaml
---
name: scaffold-mcp-tool
description: "Gera a estrutura de uma nova tool MCP em Rust: trait Tool, implementacao, schema JSON, testes com mock e registro no MCP server. Use quando: criar nova tool de automacao, expor API externa como ferramenta de agente."
user-invocable: true
---
```

**Output**: arquivo Rust da tool + schema + testes + registro em `airllm-mcp`

#### `fine-tune-ollama-model`

```yaml
---
name: fine-tune-ollama-model
description: "Executa fine-tuning de um modelo Ollama via LoRA ou DPO. Cria dataset, Modelfile, treina adapter, avalia e versiona. Use quando: treinar modelo especialista, refinar comportamento de agente, criar modelo para tema especifico."
user-invocable: true
---
```

**Output**: modelo fine-tuned versionado + relatório de avaliação comparativa

#### `configure-permissions`

```yaml
---
name: configure-permissions
description: "Configura permissoes RBAC para agentes autonomos. Define roles, tools permitidas por agente, politicas de aprovacao humana e audit trail. Use quando: modelar permissoes de agentes, configurar aprovacao para tools sensiveis."
user-invocable: true
---
```

**Output**: arquivo `permissions.toml` + matriz de permissões + audit config

#### `configure-scheduler`

```yaml
---
name: configure-scheduler
description: "Configura agendamento cron, triggers por webhook e filas de execucao para agentes autonomos. Use quando: agendar execucao recorrente, configurar trigger externo, criar fila de tarefas."
user-invocable: true
---
```

**Output**: arquivo `schedule.toml` + webhook endpoints + fila configurada

#### `deploy-autonomous-daemon`

```yaml
---
name: deploy-autonomous-daemon
description: "Gera Dockerfile, systemd service e manifests para deploy do daemon de agentes autonomos. Inclui healthcheck, restart policy, volume de estado e configuracao de segredos. Use quando: deployar plataforma de agentes em producao."
user-invocable: true
---
```

**Output**: Dockerfile + `.service` file + docker-compose + docs de deploy

---

## 6. Hooks (`.github/hooks/`)

### 6.1 Hooks existentes (adaptar)

| Hook | Mudança |
|---|---|
| `PRE_TASK.md` | Adicionar verificação: agente-alvo tem permissões configuradas? |
| `POST_IMPLEMENT.md` | Adicionar: se tool de automação foi criada, validar permissões |
| `POST_VALIDATE.md` | Adicionar: registrar execução de agente autônomo no audit trail |
| `POST_SESSION.md` | Adicionar: se daemon está rodando, registrar estado dos agentes |

### 6.2 Novos hooks (criar)

#### `PRE_AUTONOMOUS_RUN.md`

**Quando**: antes de um agente autônomo iniciar um loop de execução.

**Executor**: `edgesearch-cerebro` em modo `PRE_AUTONOMOUS`

**Verifica**:
1. Agente tem permissões configuradas em `permissions.toml`?
2. Tools que o agente pode usar estão registradas no MCP?
3. Modelo associado está disponível no Ollama?
4. Estado anterior do agente existe (resume vs fresh start)?
5. Scheduler está ativo?

**Output**: `AutonomousContextBrief` com estado do agente, permissões, tools, modelo, último checkpoint.

#### `POST_AUTONOMOUS_CYCLE.md`

**Quando**: após cada ciclo do loop autônomo (não ao final, mas a cada iteração).

**Executor**: `edgesearch-cerebro` em modo `POST_CYCLE`

**Registra**:
1. O que o agente executou neste ciclo
2. Resultado de cada tool chamada
3. Decisão tomada (próximo passo)
4. Tokens consumidos
5. Tempo de execução
6. Erros encontrados

**Persiste em**: `airllm-state` (SQLite) — tabela `agent_cycles`

#### `POST_TRAINING.md`

**Quando**: após fine-tuning de modelo.

**Executor**: `edgesearch-tester`

**Valida**:
1. Modelo fine-tuned responde coerentemente?
2. Benchmark comparativo: fine-tuned vs base (métricas)
3. Modelo salvo e versionado no Ollama?
4. Config do agente atualizada para usar novo modelo?

---

## 7. Workflows (`.github/workflows/`)

### 7.1 Workflow existente (manter)

- `validate-agents.yml` — valida estrutura de agentes

### 7.2 Novos workflows (criar)

#### `validate-platform.yml`

```yaml
name: Validate Platform
on: [push, pull_request]
jobs:
  workspace:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - name: Build workspace
        run: cargo build --workspace
      - name: Test workspace
        run: cargo test --workspace
      - name: Clippy
        run: cargo clippy --workspace --all-targets -- -D warnings
      - name: Check MCP tools
        run: cargo run -p airllm-mcp -- --list-tools
      - name: Validate permissions.toml
        run: cargo run -p airllm-permissions -- validate config/permissions.toml
      - name: Validate schedule.toml
        run: cargo run -p airllm-scheduler -- validate config/schedule.toml
```

#### `test-autonomous-loop.yml`

```yaml
name: Test Autonomous Loop
on: [pull_request]
jobs:
  autonomous:
    runs-on: ubuntu-latest
    services:
      ollama:
        image: ollama/ollama:latest
        ports:
          - 11434:11434
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - name: Pull test model
        run: ollama pull qwen3.5:4b
      - name: Run autonomous loop test
        run: cargo test -p airllm-daemon --test autonomous_loop -- --ignored
      - name: Run permission test
        run: cargo test -p airllm-permissions --test rbac
      - name: Run scheduler test
        run: cargo test -p airllm-scheduler --test cron
```

#### `benchmark-models.yml`

```yaml
name: Benchmark Models
on:
  schedule:
    - cron: "0 2 * * 0"  # todo domingo 2h
  workflow_dispatch:
jobs:
  benchmark:
    runs-on: ubuntu-latest
    services:
      ollama:
        image: ollama/ollama:latest
        ports:
          - 11434:11434
    steps:
      - uses: actions/checkout@v4
      - name: Pull models
        run: |
          ollama pull qwen3.5:4b
          ollama pull qwen3.6:27b
      - name: Run benchmarks
        run: cargo bench -p airllm-ollama
      - name: Generate report
        run: cargo run -p airllm-cli -- benchmark --output docs/
      - name: Commit report
        run: |
          git add docs/BENCHMARK_*.md
          git commit -m "docs: atualizar benchmark semanal"
          git push
```

---

## 8. Fases de Implementação

### Fase 1 — Fundação de Autonomia (2 semanas)

**Objetivo**: loop autônomo + estado persistente + permissões básicas

| # | Tarefa | Crate | Agente |
|---|---|---|---|
| 1.1 | Criar `airllm-state` (SQLite: agentes, ciclos, audit) | `airllm-state` | `edgesearch-rust-backend` |
| 1.2 | Criar `airllm-permissions` (RBAC + approval queue) | `airllm-permissions` | `edgesearch-permission-architect` |
| 1.3 | Implementar loop autônomo em `airllm-orchestrator` | `airllm-orchestrator` | `edgesearch-autonomous-runner` |
| 1.4 | Adicionar `airllm daemon` no CLI | `airllm-cli` | `edgesearch-rust-backend` |
| 1.5 | Criar hook `PRE_AUTONOMOUS_RUN.md` | `.github/hooks/` | orquestrador |
| 1.6 | Criar hook `POST_AUTONOMOUS_CYCLE.md` | `.github/hooks/` | orquestrador |
| 1.7 | Criar skill `scaffold-autonomous-agent` | `.github/skills/` | orquestrador |
| 1.8 | Criar skill `configure-permissions` | `.github/skills/` | orquestrador |
| 1.9 | Testes: loop autônomo com mock, RBAC, checkpoint/resume | todos | `edgesearch-tester` |

**Entregáveis**:
- `cargo run -p airllm-cli -- daemon` inicia processo contínuo
- Agente executa loop: code → evaluate → decide → repeat
- Estado persistido em SQLite (resume após restart)
- Permissões validadas antes de cada tool call

### Fase 2 — Tool Ecosystem (2 semanas)

**Objetivo**: tools de automação além código + MCP expandido

| # | Tarefa | Crate | Agente |
|---|---|---|---|
| 2.1 | Criar `airllm-tools` (trait `Tool` comum + registry) | `airllm-tools` | `edgesearch-tool-builder` |
| 2.2 | Implementar tool `social_media` (Twitter API v2, LinkedIn) | `airllm-tools` | `edgesearch-tool-builder` |
| 2.3 | Implementar tool `messaging` (Telegram Bot API, Slack webhook) | `airllm-tools` | `edgesearch-tool-builder` |
| 2.4 | Implementar tool `email` (SMTP via `lettre` crate) | `airllm-tools` | `edgesearch-tool-builder` |
| 2.5 | Implementar tool `webhook` (HTTP genérico assíncrono) | `airllm-tools` | `edgesearch-tool-builder` |
| 2.6 | Expandir `airllm-mcp` para expor novas tools | `airllm-mcp` | `edgesearch-tool-builder` |
| 2.7 | Criar skill `scaffold-mcp-tool` | `.github/skills/` | orquestrador |
| 2.8 | Testes: cada tool com mock + teste de permissão | todos | `edgesearch-tester` |

**Entregáveis**:
- `airllm-mcp` expõe: code, review, test, list_models, post_social, send_message, send_email, webhook_call
- Cada tool valida permissões antes de executar
- Tools testadas com mocks (sem chamadas reais em CI)

### Fase 3 — Scheduler e Triggers (2 semanas)

**Objetivo**: agendamento cron + triggers externos + fila de execução

| # | Tarefa | Crate | Agente |
|---|---|---|---|
| 3.1 | Criar `airllm-scheduler` (cron parser + timezone) | `airllm-scheduler` | `edgesearch-scheduler-architect` |
| 3.2 | Implementar triggers webhook (HTTP endpoint) | `airllm-scheduler` | `edgesearch-scheduler-architect` |
| 3.3 | Implementar fila de execução (prioridade + retry + backoff) | `airllm-scheduler` | `edgesearch-scheduler-architect` |
| 3.4 | Integrar scheduler com daemon | `airllm-daemon` | `edgesearch-autonomous-runner` |
| 3.5 | Adicionar `airllm schedule` no CLI | `airllm-cli` | `edgesearch-rust-backend` |
| 3.6 | Criar skill `configure-scheduler` | `.github/skills/` | orquestrador |
| 3.7 | Testes: cron parsing, webhook trigger, fila com prioridade | todos | `edgesearch-tester` |

**Entregáveis**:
- `airllm schedule add --cron "0 9 * * *" --agent social-media --task "post daily summary"`
- Webhook trigger: `POST /trigger/{agent}/{task}` dispara execução
- Fila respeita prioridade e retry com backoff exponencial

### Fase 4 — Model Training Pipeline (3 semanas)

**Objetivo**: fine-tuning LoRA/DPO via Ollama + versionamento

| # | Tarefa | Crate | Agente |
|---|---|---|---|
| 4.1 | Criar `airllm-training` (dataset prep + Modelfile + adapter) | `airllm-training` | `edgesearch-agent-trainer` |
| 4.2 | Implementar pipeline LoRA (via `ollama create` + adapter) | `airllm-training` | `edgesearch-agent-trainer` |
| 4.3 | Implementar pipeline DPO (preference dataset + treino) | `airllm-training` | `edgesearch-agent-trainer` |
| 4.4 | Implementar benchmark comparativo (base vs fine-tuned) | `airllm-training` | `edgesearch-agent-trainer` |
| 4.5 | Versionar modelos (`qwen3.5:4b-{specialty}-v{N}`) | `airllm-training` | `edgesearch-agent-trainer` |
| 4.6 | Adicionar `airllm train` no CLI | `airllm-cli` | `edgesearch-rust-backend` |
| 4.7 | Criar skill `fine-tune-ollama-model` | `.github/skills/` | orquestrador |
| 4.8 | Criar hook `POST_TRAINING.md` | `.github/hooks/` | orquestrador |
| 4.9 | Testes: pipeline completo com modelo pequeno | todos | `edgesearch-tester` |

**Entregáveis**:
- `airllm train --base qwen3.5:4b --dataset social_media_examples.jsonl --name social-v1`
- Modelo fine-tuned disponível no Ollama como `qwen3.5:4b-social-v1`
- Benchmark: fine-tuned vs base em métricas de qualidade
- Agentes podem referenciar modelo fine-tuned no TOML

### Fase 5 — Especialistas e Daemon em Produção (2 semanas)

**Objetivo**: agentes especialistas configurados + daemon deployável

| # | Tarefa | Crate | Agente |
|---|---|---|---|
| 5.1 | Criar agentes especialistas (TOML + system prompt + modelo) | `airllm-orchestrator/agents/` | `edgesearch-autonomous-runner` |
| 5.2 | Configurar permissões por especialista | `config/permissions.toml` | `edgesearch-permission-architect` |
| 5.3 | Configurar schedules por especialista | `config/schedule.toml` | `edgesearch-scheduler-architect` |
| 5.4 | Criar `airllm-daemon` (processo contínuo + healthcheck) | `airllm-daemon` | `edgesearch-autonomous-runner` |
| 5.5 | Dockerfile + systemd service | `deploy/` | `edgesearch-sre` |
| 5.6 | Criar skill `deploy-autonomous-daemon` | `.github/skills/` | orquestrador |
| 5.7 | Workflow `validate-platform.yml` | `.github/workflows/` | orquestrador |
| 5.8 | Workflow `test-autonomous-loop.yml` | `.github/workflows/` | orquestrador |
| 5.9 | E2E: daemon roda 24h com 3 especialistas + scheduler | todos | `edgesearch-tester` |

**Entregáveis**:
- `systemctl start airllm-daemon` → daemon carrega agentes, scheduler, permissões
- 3 especialistas: `social-media`, `messaging`, `research`
- Cada especialista roda em loop autônomo com tools permitidas
- Healthcheck: `GET /health` retorna estado de todos os agentes
- Deploy via Dockerfile + docker-compose

---

## 9. Configuração de Especialistas (exemplo)

### `config/agents/social-media.toml`

```toml
name = "social-media"
model = "qwen3.5:4b-social-v1"
fallback_model = "qwen3.5:4b"
system_prompt = "prompts/social-media.md"
parallelizable = false
max_concurrent = 1
temperature = 0.7
top_p = 0.9

[permissions]
allowed_tools = ["post_social", "webhook_call", "list_models"]
require_approval = false
max_actions_per_hour = 10

[schedule]
cron = "0 9,12,18 * * *"
timezone = "America/Sao_Paulo"

[tools.post_social]
platforms = ["twitter", "linkedin"]
```

### `config/agents/messaging.toml`

```toml
name = "messaging"
model = "qwen3.5:4b"
system_prompt = "prompts/messaging.md"
parallelizable = true
max_concurrent = 3
temperature = 0.5

[permissions]
allowed_tools = ["send_message", "list_models"]
require_approval = true
max_actions_per_hour = 20

[schedule]
trigger = "webhook"
endpoint = "/trigger/messaging"
```

### `config/agents/research.toml`

```toml
name = "research"
model = "qwen3.6:27b"
system_prompt = "prompts/research.md"
parallelizable = false
max_concurrent = 1
temperature = 0.3

[permissions]
allowed_tools = ["web_search", "web_fetch", "code", "list_models"]
require_approval = false
max_actions_per_hour = 50

[schedule]
cron = "0 6 * * 1"  # toda segunda 6h
```

---

## 10. Matriz de Permissões (exemplo)

### `config/permissions.toml`

```toml
[roles]
[roles.admin]
tools = ["*"]
approval_required = false

[roles.operator]
tools = ["code", "review", "test", "list_models", "web_search", "web_fetch"]
approval_required = false

[roles.automated]
tools = ["post_social", "send_message", "send_email", "webhook_call"]
approval_required = true

[roles.researcher]
tools = ["web_search", "web_fetch", "code", "list_models"]
approval_required = false

[agents.social-media]
role = "automated"
overrides = { tools = ["post_social", "webhook_call"], approval_required = false }

[agents.messaging]
role = "automated"
overrides = { approval_required = true }

[agents.research]
role = "researcher"
```

---

## 11. CLI Commands (expansão do `airllm-cli`)

```bash
# Daemon
airllm daemon                          # inicia processo contínuo
airllm daemon --config config/daemon.toml
airllm daemon status                   # estado atual de todos os agentes
airllm daemon stop                     # para graceful

# Agentes
airllm agent list                      # lista agentes configurados
airllm agent run <name>                # executa agente uma vez
airllm agent run <name> --autonomous   # loop autônomo
airllm agent run <name> --checkpoint   # resume do último checkpoint

# Scheduler
airllm schedule list                   # lista jobs agendados
airllm schedule add --cron "..." --agent <name> --task "..."
airllm schedule remove <id>
airllm schedule trigger <name>         # dispara manualmente

# Training
airllm train --base qwen3.5:4b --dataset data.jsonl --name social-v1
airllm train --list                     # modelos fine-tuned disponíveis
airllm train --benchmark social-v1      # benchmark vs base

# Permissions
airllm permissions validate             # valida config
airllm permissions audit                # audit trail
airllm permissions approve <pending-id> # aprova ação pendente
```

---

## 12. Checklist de Coordenação

### Antes de iniciar

- [ ] Plano aprovado por Erik
- [ ] Context bank atualizado com decisão v4.0
- [ ] Workspace v3.0 estável (tag `v3.0.0` pushed)
- [ ] Branch `feat/airllm-v4-autonomous-platform` criada

### Durante implementação

- [ ] Fase 1: loop autônomo + estado + permissões
- [ ] Fase 2: tools de automação + MCP expandido
- [ ] Fase 3: scheduler + triggers + fila
- [ ] Fase 4: training pipeline + LoRA/DPO
- [ ] Fase 5: especialistas + daemon + deploy

### Após cada fase

- [ ] `cargo build --workspace` OK
- [ ] `cargo test --workspace` OK
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` OK
- [ ] Context bank atualizado (POST_TASK via Cérebro)
- [ ] Git workflow executado (branch + commit + merge)
- [ ] Tag semver bumped

### Ao final

- [ ] Daemon roda 24h sem crash
- [ ] 3 especialistas ativos com scheduler
- [ ] Audit trail completo em SQLite
- [ ] Permissões validadas em todas as tool calls
- [ ] Dockerfile + docker-compose funcionais
- [ ] Documentação completa (README + guias)
- [ ] Tag `v4.0.0`

---

## 13. Riscos e Mitigações

| Risco | Probabilidade | Impacto | Mitigação |
|---|---|---|---|
| Agente autônomo entra em loop infinito | Alta | Alto | Timeout por ciclo + max_actions_per_hour + checkpoint |
| Tool de automação executa ação indesejada | Média | Alto | Approval queue + audit trail + dry-run mode |
| Modelo fine-tuned degrada qualidade | Média | Médio | Benchmark comparativo obrigatório + rollback |
| Daemon crasha e perde estado | Baixa | Alto | SQLite persistente + checkpoint + systemd restart |
| Ollama sobrecarregado com múltiplos agentes | Média | Médio | max_concurrent por agente + queue + prewarm seletivo |
| Credenciais de API expostas | Baixa | Crítico | Secrets em env vars + nunca logar tokens + `.env` no `.gitignore` |

---

## 14. Próximos Passos

1. **Aprovação** — Erik revisa este plano
2. **Context bank** — registrar decisão v4.0 no `.edgesearch/context.md`
3. **Branch** — `feat/airllm-v4-autonomous-platform`
4. **Fase 1** — iniciar com `edgesearch-rust-backend` criando `airllm-state` e `airllm-permissions`
5. **Hooks** — criar `PRE_AUTONOMOUS_RUN.md` e `POST_AUTONOMOUS_CYCLE.md`
6. **Skills** — criar `scaffold-autonomous-agent` e `configure-permissions`
7. **Agentes** — criar `.agent.md` dos 5 novos agentes