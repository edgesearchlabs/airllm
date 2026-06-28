# Plano Revisado — AirLLM v3.0: Multi-Agentes de Codificação em Rust

> **Data**: 2026-06-27
> **Autor**: EdgeSearch Orquestrador
> **Revisor**: Erik Tonon
> **Status**: Proposta — Aguardando aprovação
> **Baseline**: AirLLM v2.11.0 (Python, Agosto 2024)

---

## 1. Visão Geral

Este plano **substitui** o plano anterior (`PLANO_MODERNIZACAO.md`). O foco mudou:

### O que mudou do plano anterior

| Aspecto | Plano Anterior | Plano Revisado (este) |
|---|---|---|
| **Objetivo** | Modernizar o AirLLM (inferência layer-wise) | Criar sistema de **multi-agentes de codificação** de alta performance |
| **Linguagem** | Python → Rust (Fase 3 opcional) | **Rust desde o início** — Python só como binding |
| **Modelos** | HuggingFace safetensors | **Ollama** com modelos Qwen locais + cloud |
| **Foco** | Inferência de LLM em low-VRAM | **Orquestração de agentes para codificação** |
| **Arquitetura** | Monolito (layer-wise engine) | **Multi-agente distribuído com pipeline paralelo** |
| **Performance** | Melhorar I/O e KV cache | **Rust + tokio + rayon + mmap + zero-copy** |

### Princípios

1. **Rust é a linguagem principal** — Python não tem performance para orquestração de múltiplos agentes em paralelo
2. **Ollama é o backend de inferência** — não reinventamos a roda; usamos modelos Qwen já disponíveis
3. **Multi-agente com pipeline paralelo** — agentes executam concorrentemente, não sequencialmente
4. **Máxima performance** — async I/O (tokio), paralelismo de dados (rayon), zero-copy (mmap), sem GC

---

## 2. Modelos Qwen Disponíveis no Ollama

Inventário dos modelos já instalados localmente:

### Modelos Locais (rodam na máquina)

| Modelo | Parâmetros | Quantização | Tamanho | Contexto | Capacidades | Caso de Uso |
|---|---|---|---|---|---|---|
| `qwen3.5:4b` | 4.7B | Q4_K_M | 3.4 GB | 262K | completion, vision, tools, thinking | **Draft model** para speculative decoding. Agente rápido para tarefas simples. |
| `qwen3.6:27b` | 27.8B | Q4_K_M | 17 GB | 262K | completion, vision, tools, thinking | **Agente principal de codificação**. Balanceamento de velocidade e qualidade. |
| `qwen3-coder-next:q8_0` | 79.7B | Q8_0 | 84 GB | 262K | completion, tools | **Agente especialista em código**. Alta precisão, tarefas complexas. |
| `granite4.1:30b` | 30B | — | 17 GB | — | — | Alternativa para raciocínio (IBM Granite) |
| `nemotron-3-nano:30b` | 30B | — | 24 GB | — | — | Alternativa NVIDIA (otimizado para raciocínio) |
| `jaahas/crow:9b` | 9B | — | 6.5 GB | — | — | Modelo leve alternativo |

### Modelos Cloud (via Ollama Cloud)

| Modelo | Parâmetros | Contexto | Capacidades | Caso de Uso |
|---|---|---|---|---|
| `qwen3.5:397b-cloud` | 397B | 262K | completion, thinking, tools, vision | **Agente orquestrador cloud** — máxima qualidade para decisões críticas |
| `kimi-k2.7-code:cloud` | 1T | 262K | vision, thinking, completion, tools | **Especialista em código cloud** — tarefas de altíssima complexidade |
| `glm-5.2:cloud` | 756B | 1M | thinking, completion, tools | **Raciocínio estendido** — contextos extremamente longos |
| `minimax-m2.7:cloud` | — | — | — | Alternativa cloud |
| `kimi-k2.6:cloud` | — | — | — | Versão anterior do Kimi |

### Estratégia de Roteamento de Modelos

```
┌─────────────────────────────────────────────────────────────┐
│                    ORQUESTRADOR (Rust)                       │
│                                                              │
│  Analisa a tarefa → classifica complexidade → roteia        │
└──────────────────────────┬──────────────────────────────────┘
                           │
           ┌───────────────┼───────────────┐
           ▼               ▼               ▼
    ┌──────────────┐ ┌──────────────┐ ┌──────────────┐
    │  BAIXA       │ │  MÉDIA       │ │  ALTA        │
    │  COMPLEXIDADE│ │  COMPLEXIDADE│ │  COMPLEXIDADE│
    │              │ │              │ │              │
    │ qwen3.5:4b   │ │ qwen3.6:27b  │ │ qwen3-coder  │
    │ (local, 4B)  │ │ (local, 27B) │ │ :q8_0 (80B)  │
    │              │ │              │ │ ou cloud     │
    │ < 100ms      │ │ ~1-3s        │ │ ~5-30s       │
    └──────────────┘ └──────────────┘ └──────────────┘
```

**Regras de roteamento**:

| Tipo de tarefa | Modelo | Por quê |
|---|---|---|
| Completar snippet, formatar, rename | `qwen3.5:4b` | Rápido, 4B é suficiente |
| Implementar função, criar teste, revisar PR | `qwen3.6:27b` | Balanceamento ideal |
| Arquitetar sistema, refatorar módulo, debug complexo | `qwen3-coder-next:q8_0` ou `kimi-k2.7-code:cloud` | Máxima qualidade |
| Decisão de arquitetura, planejamento de sprint | `qwen3.5:397b-cloud` | Raciocínio profundo |
| Contexto > 100K tokens | `glm-5.2:cloud` | Janela de 1M tokens |

---

## 3. Arquitetura do Sistema

### 3.1 Visão Geral

```
┌─────────────────────────────────────────────────────────────────────┐
│                     AirLLM v3.0 — Rust Core                         │
│                                                                      │
│  ┌─────────────────────────────────────────────────────────────┐    │
│  │              ORQUESTRADOR (Rust + tokio)                    │    │
│  │                                                              │    │
│  │  • Recebe requisição do usuário (via CLI ou MCP)            │    │
│  │  • Classifica complexidade e seleciona modelo               │    │
│  │  • Divide tarefa em sub-tarefas para agentes                │    │
│  │  • Gerencia pipeline paralelo                               │    │
│  │  • Consolida resultados                                     │    │
│  └──────────┬──────────────────┬──────────────────┬────────────┘    │
│             │                  │                  │                  │
│     ┌───────▼──────┐  ┌───────▼──────┐  ┌───────▼──────┐           │
│     │  AGENTE A     │  │  AGENTE B     │  │  AGENTE C     │           │
│     │  (tokio task) │  │  (tokio task) │  │  (tokio task) │           │
│     │               │  │               │  │               │           │
│     │  Ollama API   │  │  Ollama API   │  │  Ollama API   │           │
│     │  (HTTP async) │  │  (HTTP async) │  │  (HTTP async) │           │
│     │               │  │               │  │               │           │
│     │  qwen3.5:4b   │  │  qwen3.6:27b  │  │  kimi-k2.7    │           │
│     │  (draft)      │  │  (coder)      │  │  (cloud)      │           │
│     └───────┬───────┘  └───────┬───────┘  └───────┬───────┘           │
│             │                  │                  │                  │
│             └──────────────────┼──────────────────┘                  │
│                                ▼                                     │
│                    ┌───────────────────┐                             │
│                    │  CONSOLIDADOR     │                             │
│                    │  (merge results)  │                             │
│                    └───────────────────┘                             │
│                                                                      │
│  ┌─────────────────────────────────────────────────────────────┐    │
│  │              OLLAMA RUNTIME (já instalado)                  │    │
│  │                                                              │    │
│  │  qwen3.5:4b    qwen3.6:27b    qwen3-coder-next    cloud     │    │
│  │  (3.4GB)       (17GB)         (84GB)              models    │    │
│  └─────────────────────────────────────────────────────────────┘    │
│                                                                      │
│  ┌─────────────────────────────────────────────────────────────┐    │
│  │              PYTHON BINDINGS (PyO3) — opcional              │    │
│  │                                                              │    │
│  │  from airllm import Orchestrator                            │    │
│  │  orch = Orchestrator()                                       │    │
│  │  result = orch.code("implement user auth module")          │    │
│  └─────────────────────────────────────────────────────────────┘    │
└─────────────────────────────────────────────────────────────────────┘
```

### 3.2 Por que Rust e não Python

| Critério | Python | Rust |
|---|---|---|
| **Concorrência real** | GIL bloqueia paralelismo | `tokio` + `rayon` — paralelismo real multi-core |
| **Latência de orquestração** | ~10-50ms overhead por chamada | ~0.1-1ms overhead |
| **Memória** | GC imprevisível, pausas | Ownership determinístico, zero pausas |
| **I/O assíncrono** | asyncio single-thread | `tokio` multi-core async |
| **Tipagem** | Duck typing, erros em runtime | Type system, erros em compile-time |
| **Distribuição** | pip + virtualenv + dependências | Single binary + Python wheel opcional |
| **Integração com Ollama** | httpx (Python) | reqwest (Rust) — 5-10x mais rápido |
| **Multi-agente paralelo** | Subprocess ou asyncio (GIL) | `tokio::spawn` — verdadeiro paralelismo |

**Python não tem a melhor performance para orquestração de múltiplos agentes em paralelo.** O GIL (Global Interpreter Lock) impede que múltiplas threads executem Python bytecode simultaneamente. Para um sistema de multi-agentes onde 3-5 agentes precisam chamar a API do Ollama concorrentemente, Rust com `tokio` é a escolha correta.

### 3.3 Componentes da Arquitetura

#### 3.3.1 Orchestrator Core (`airllm-core`)

```rust
// crates/airllm-core/src/lib.rs

pub struct Orchestrator {
    ollama: OllamaClient,
    model_router: ModelRouter,
    agent_registry: AgentRegistry,
    context_store: ContextStore,
}

impl Orchestrator {
    /// Processa uma requisição de codificação do usuário
    pub async fn code(&self, request: CodeRequest) -> Result<CodeResponse> {
        // 1. Classifica complexidade
        let complexity = self.model_router.classify(&request);
        
        // 2. Divide em sub-tarefas
        let subtasks = self.decompose(&request, &complexity).await?;
        
        // 3. Executa agentes em paralelo
        let results = self.execute_parallel(subtasks).await?;
        
        // 4. Consolida resultados
        let consolidated = self.consolidate(results).await?;
        
        Ok(consolidated)
    }
    
    /// Executa múltiplos agentes concorrentemente
    async fn execute_parallel(&self, subtasks: Vec<SubTask>) -> Result<Vec<AgentResult>> {
        let futures: Vec<_> = subtasks
            .into_iter()
            .map(|task| self.spawn_agent(task))
            .collect();
        
        // tokio::join_all — verdadeiro paralelismo async
        let results = futures::future::join_all(futures).await;
        
        Ok(results)
    }
}
```

#### 3.3.2 Model Router (`airllm-core/src/router.rs`)

```rust
pub struct ModelRouter {
    rules: Vec<RoutingRule>,
}

pub enum Complexity {
    Low,    // → qwen3.5:4b (local, rápido)
    Medium, // → qwen3.6:27b (local, balanceado)
    High,   // → qwen3-coder-next:q8_0 (local, pesado)
    Cloud,  // → kimi-k2.7-code:cloud ou qwen3.5:397b-cloud
}

pub struct RoutingRule {
    pub pattern: Regex,
    pub complexity: Complexity,
    pub model: String,
}

impl ModelRouter {
    pub fn classify(&self, request: &CodeRequest) -> Complexity {
        // Heurísticas:
        // - "rename", "format", "complete" → Low
        // - "implement function", "create test", "fix bug" → Medium
        // - "architect", "refactor module", "design system" → High
        // - contexto > 100K tokens → Cloud (glm-5.2)
    }
    
    pub fn select_model(&self, complexity: &Complexity) -> &str {
        match complexity {
            Complexity::Low => "qwen3.5:4b",
            Complexity::Medium => "qwen3.6:27b",
            Complexity::High => "qwen3-coder-next:q8_0",
            Complexity::Cloud => "kimi-k2.7-code:cloud",
        }
    }
}
```

#### 3.3.3 Agent Executor (`airllm-core/src/agent.rs`)

```rust
pub struct Agent {
    id: String,
    model: String,
    ollama: OllamaClient,
    system_prompt: String,
}

impl Agent {
    pub async fn execute(&self, task: &SubTask) -> Result<AgentResult> {
        // 1. Constrói prompt com system + context + task
        let prompt = self.build_prompt(task);
        
        // 2. Chama Ollama API (HTTP async, non-blocking)
        let response = self.ollama
            .chat(&self.model, &prompt, &self.system_prompt)
            .await?;
        
        // 3. Parse resultado
        let result = self.parse_response(&response)?;
        
        Ok(result)
    }
}

pub struct OllamaClient {
    base_url: String,
    http: reqwest::Client,
}

impl OllamaClient {
    pub async fn chat(&self, model: &str, prompt: &str, system: &str) -> Result<String> {
        let payload = ChatRequest {
            model: model.to_string(),
            messages: vec![
                Message::system(system),
                Message::user(prompt),
            ],
            stream: false,
            options: ModelOptions {
                temperature: 0.7,
                top_p: 0.95,
                ..Default::default()
            },
        };
        
        let resp = self.http
            .post(&format!("{}/api/chat", self.base_url))
            .json(&payload)
            .send()
            .await?
            .json::<ChatResponse>()
            .await?;
        
        Ok(resp.message.content)
    }
    
    /// Streaming — para resposta em tempo real
    pub async fn chat_stream(&self, model: &str, prompt: &str) 
        -> Result<impl Stream<Item = Result<String>>> {
        // Usa SSE (Server-Sent Events) do Ollama
        // tokio::stream para processar chunks em tempo real
    }
}
```

#### 3.3.4 Context Store (`airllm-core/src/context.rs`)

```rust
/// Armazena contexto compartilhado entre agentes
/// Usa memory-mapped files para zero-copy
pub struct ContextStore {
    path: PathBuf,
    mmap: Mmap,
}

impl ContextStore {
    pub fn load(&self, key: &str) -> &[u8] {
        // Lê diretamente do mmap — zero copy
    }
    
    pub fn store(&mut self, key: &str, data: &[u8]) -> Result<()> {
        // Append-only log (estilo event sourcing)
    }
}
```

### 3.4 Estrutura do Projeto Rust

```
airllm/
├── Cargo.toml                    # Workspace root
├── crates/
│   ├── airllm-core/              # Core da orquestração
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── orchestrator.rs   # Orquestrador principal
│   │       ├── router.rs         # Roteamento de modelos
│   │       ├── agent.rs          # Executor de agentes
│   │       ├── ollama.rs         # Cliente Ollama (HTTP async)
│   │       ├── context.rs        # Context store (mmap)
│   │       ├── decompose.rs      # Decomposição de tarefas
│   │       ├── consolidate.rs    # Consolidação de resultados
│   │       ├── streaming.rs     # Streaming de respostas
│   │       └── profiler.rs       # Profiling e métricas
│   │
│   ├── airllm-cli/              # CLI tool
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── main.rs
│   │       ├── commands/
│   │       │   ├── code.rs       # airllm code "implement auth"
│   │       │   ├── review.rs     # airllm review src/
│   │       │   ├── test.rs       # airllm test --generate
│   │       │   ├── refactor.rs   # airllm refactor module/
│   │       │   └── chat.rs       # airllm chat (modo interativo)
│   │       └── tui.rs            # Terminal UI (ratatui)
│   │
│   ├── airllm-mcp/              # MCP Server (substitui edgesearch_mcp)
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       └── server.rs         # MCP server em Rust (rmcp crate)
│   │
│   └── airllm-python/           # Python bindings (PyO3)
│       ├── Cargo.toml
│       └── src/
│           └── lib.rs            # #[pyfunction] wrappers
│
├── python/
│   └── airllm/                   # Python package (wrapper)
│       ├── __init__.py
│       └── __init__.pyi           # Type stubs
│
├── agents/                       # Definições de agentes (TOML)
│   ├── coder.toml                # Agente de codificação
│   ├── reviewer.toml             # Agente de code review
│   ├── tester.toml               # Agente de testes
│   ├── architect.toml            # Agente de arquitetura
│   ├── debugger.toml             # Agente de debug
│   └── orchestrator.toml         # Meta-agente orquestrador
│
├── prompts/                      # System prompts por agente
│   ├── coder.md
│   ├── reviewer.md
│   ├── tester.md
│   ├── architect.md
│   └── debugger.md
│
├── benches/                      # Benchmarks (criterion)
│   ├── routing.rs
│   ├── parallel_agents.rs
│   └── ollama_latency.rs
│
├── tests/
│   ├── integration/
│   └── e2e/
│
└── docs/
    ├── ARCHITECTURE.md
    ├── MODELS.md                  # Catálogo de modelos e roteamento
    └── README_pt-br.md
```

---

## 4. Agentes de Codificação

### 4.1 Catálogo de Agentes

| Agente | Modelo Padrão | Função | Paralelizable |
|---|---|---|---|
| **Coder** | `qwen3.6:27b` | Implementa código a partir de spec | ✅ Múltiplos arquivos em paralelo |
| **Reviewer** | `qwen3.6:27b` | Revisa código, sugere melhorias | ✅ Múltiplos PRs em paralelo |
| **Tester** | `qwen3.5:4b` | Gera testes unitários | ✅ Um por arquivo |
| **Architect** | `qwen3-coder-next:q8_0` ou cloud | Define estrutura, escolhe padrões | ❌ Sequencial |
| **Debugger** | `qwen3-coder-next:q8_0` | Analisa erros, propõe correções | ✅ Múltiplos erros em paralelo |
| **Refactorer** | `qwen3.6:27b` | Refatora código existente | ✅ Múltiplos módulos em paralelo |
| **Documenter** | `qwen3.5:4b` | Gera documentação | ✅ Múltiplos arquivos em paralelo |

### 4.2 Pipeline de Codificação Típico

```
Usuário: "Implementar módulo de autenticação JWT"

┌─────────────────────────────────────────────────────────────┐
│  1. ARCHITECT (qwen3-coder-next ou cloud)                   │
│     • Define estrutura: handlers, middleware, models        │
│     • Output: plano de arquivos e interfaces                │
│     • Tempo: ~10-30s                                         │
└──────────────────────────┬──────────────────────────────────┘
                           │
                           ▼
┌─────────────────────────────────────────────────────────────┐
│  2. DECOMPOSE (orquestrador Rust)                           │
│     • Divide plano em sub-tarefas:                          │
│       - Task A: implementar JWT middleware                  │
│       - Task B: implementar login handler                   │
│       - Task C: implementar refresh token handler            │
│       - Task D: implementar user model                       │
└──────────────────────────┬──────────────────────────────────┘
                           │
           ┌───────────────┼───────────────┐
           ▼               ▼               ▼
    ┌────────────┐  ┌────────────┐  ┌────────────┐
    │  CODER A   │  │  CODER B   │  │  CODER C   │  ← PARALELO
    │ (qwen3.6   │  │ (qwen3.6   │  │ (qwen3.6   │     tokio::spawn
    │  :27b)     │  │  :27b)     │  │  :27b)     │
    │            │  │            │  │            │
    │ middleware │  │ login.rs   │  │ refresh.rs │
    │ .rs        │  │            │  │            │
    └─────┬──────┘  └─────┬──────┘  └─────┬──────┘
          │               │               │
          └───────────────┼───────────────┘
                          ▼
    ┌──────────────────────────────────────────┐
    │  3. REVIEWER (qwen3.6:27b)               │
    │     • Revisa código gerado               │
    │     • Verifica consistência entre mods   │
    │     • Sugere correções                   │
    └──────────────────┬───────────────────────┘
                       │
                       ▼
    ┌──────────────────────────────────────────┐
    │  4. TESTER (qwen3.5:4b) — PARALELO       │
    │     • Gera testes para cada arquivo      │
    │     • Um agente por arquivo (tokio)     │
    └──────────────────┬───────────────────────┘
                       │
                       ▼
    ┌──────────────────────────────────────────┐
    │  5. DOCUMENTER (qwen3.5:4b) — PARALELO   │
    │     • Gera docs para cada módulo         │
    └──────────────────────────────────────────┘
```

### 4.3 Definição de Agente (TOML)

```toml
# agents/coder.toml
[agent]
name = "coder"
description = "Implementa código a partir de especificações"
default_model = "qwen3.6:27b"
fallback_model = "qwen3.5:4b"
parallelizable = true
max_concurrent = 4

[agent.capabilities]
can_write_files = true
can_read_files = true
can_execute_commands = false
can_call_other_agents = false

[agent.prompt]
system = "prompts/coder.md"
temperature = 0.7
top_p = 0.95
max_tokens = 8192

[agent.routing]
# Tarefas que este agente pode handle
patterns = [
    "implement.*",
    "create.*function",
    "create.*handler",
    "create.*model",
    "write.*code",
]
```

### 4.4 System Prompts

```markdown
<!-- prompts/coder.md -->
You are an expert code generation agent. You receive a specification 
and produce production-ready code.

## Rules
- Write clean, idiomatic code following the project's conventions
- Include error handling
- Add inline comments for complex logic only
- Do NOT add unnecessary dependencies
- Return ONLY code, no explanations (unless asked)

## Output Format
Return code in fenced code blocks with the language specified:

```rust
// your code here
```
```

---

## 5. Integração com Ollama

### 5.1 Cliente Ollama em Rust

```rust
// crates/airllm-core/src/ollama.rs

use reqwest::Client;
use serde::{Deserialize, Serialize};
use tokio::stream::StreamExt;

pub struct OllamaClient {
    base_url: String,
    http: Client,
}

#[derive(Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<Message>,
    stream: bool,
    options: Options,
}

#[derive(Serialize)]
struct Message {
    role: String,
    content: String,
}

#[derive(Serialize, Default)]
struct Options {
    temperature: f32,
    top_p: f32,
    top_k: u32,
    num_ctx: u32,
}

impl OllamaClient {
    pub fn new(url: &str) -> Self {
        Self {
            base_url: url.to_string(),
            http: Client::builder()
                .timeout(std::time::Duration::from_secs(300))
                .build()
                .unwrap(),
        }
    }
    
    /// Chat non-streaming
    pub async fn chat(&self, model: &str, messages: Vec<Message>, options: Options) 
        -> Result<String, OllamaError> 
    {
        let req = ChatRequest {
            model: model.to_string(),
            messages,
            stream: false,
            options,
        };
        
        let resp: ChatResponse = self.http
            .post(format!("{}/api/chat", self.base_url))
            .json(&req)
            .send()
            .await?
            .json()
            .await?;
        
        Ok(resp.message.content)
    }
    
    /// Chat com streaming — resposta em tempo real
    pub async fn chat_stream(
        &self, 
        model: &str, 
        messages: Vec<Message>,
    ) -> Result<impl tokio_stream::Stream<Item = Result<String>>, OllamaError> 
    {
        let req = ChatRequest {
            model: model.to_string(),
            messages,
            stream: true,
            options: Options::default(),
        };
        
        let response = self.http
            .post(format!("{}/api/chat", self.base_url))
            .json(&req)
            .send()
            .await?;
        
        // SSE stream — cada chunk é um JSON com token parcial
        let stream = response.bytes_stream()
            .map(|chunk| {
                let chunk = chunk?;
                let line = String::from_utf8_lossy(&chunk);
                // Parse JSON line → extract token
                Ok(parse_stream_chunk(&line)?)
            });
        
        Ok(stream)
    }
    
    /// Lista modelos disponíveis
    pub async fn list_models(&self) -> Result<Vec<ModelInfo>, OllamaError> {
        let resp: ModelsResponse = self.http
            .get(format!("{}/api/tags", self.base_url))
            .send()
            .await?
            .json()
            .await?;
        
        Ok(resp.models)
    }
    
    /// Verifica se um modelo está disponível
    pub async fn model_available(&self, model: &str) -> Result<bool, OllamaError> {
        let models = self.list_models().await?;
        Ok(models.iter().any(|m| m.name == model))
    }
}
```

### 5.2 Configuração de Modelos

```toml
# config/models.toml

[models.qwen3.5_4b]
name = "qwen3.5:4b"
type = "local"
size = "4.7B"
quantization = "Q4_K_M"
vram_required = "3.4GB"
context_length = 262144
capabilities = ["completion", "vision", "tools", "thinking"]
use_for = ["draft", "simple_tasks", "testing", "documentation"]

[models.qwen3.6_27b]
name = "qwen3.6:27b"
type = "local"
size = "27.8B"
quantization = "Q4_K_M"
vram_required = "17GB"
context_length = 262144
capabilities = ["completion", "vision", "tools", "thinking"]
use_for = ["coding", "review", "refactoring"]

[models.qwen3_coder_next]
name = "qwen3-coder-next:q8_0"
type = "local"
size = "79.7B"
quantization = "Q8_0"
vram_required = "84GB"
context_length = 262144
capabilities = ["completion", "tools"]
use_for = ["architecture", "complex_coding", "debugging"]

[models.qwen3.5_397b_cloud]
name = "qwen3.5:397b-cloud"
type = "cloud"
size = "397B"
context_length = 262144
capabilities = ["completion", "thinking", "tools", "vision"]
use_for = ["orchestration", "complex_reasoning"]

[models.kimi_k2.7_code_cloud]
name = "kimi-k2.7-code:cloud"
type = "cloud"
size = "1T"
context_length = 262144
capabilities = ["vision", "thinking", "completion", "tools"]
use_for = ["complex_coding", "large_context"]

[models.glm_5.2_cloud]
name = "glm-5.2:cloud"
type = "cloud"
size = "756B"
context_length = 1000000
capabilities = ["thinking", "completion", "tools"]
use_for = ["extended_context", "long_reasoning"]
```

---

## 6. MCP Server em Rust

Substitui o `edgesearch_mcp/server.py` (Python) por uma implementação Rust nativa:

```rust
// crates/airllm-mcp/src/server.rs

use rmcp::{Server, ServerHandler};
use rmcp::model::*;

#[derive(Clone)]
pub struct AirLLMMcpServer {
    orchestrator: Arc<Orchestrator>,
}

impl ServerHandler for AirLLMMcpServer {
    /// Lista ferramentas disponíveis
    async fn list_tools(&self) -> Result<Vec<Tool>> {
        Ok(vec![
            Tool {
                name: "code".into(),
                description: "Generate code from specification".into(),
                input_schema: schema!({
                    "type": "object",
                    "properties": {
                        "task": { "type": "string" },
                        "language": { "type": "string" },
                        "files": { "type": "array", "items": { "type": "string" } }
                    },
                    "required": ["task"]
                }),
            },
            Tool {
                name: "review".into(),
                description: "Review code and suggest improvements".into(),
                input_schema: schema!({
                    "type": "object",
                    "properties": {
                        "files": { "type": "array", "items": { "type": "string" } }
                    },
                    "required": ["files"]
                }),
            },
            Tool {
                name: "test".into(),
                description: "Generate tests for code files".into(),
                input_schema: schema!({
                    "type": "object",
                    "properties": {
                        "files": { "type": "array", "items": { "type": "string" } }
                    },
                    "required": ["files"]
                }),
            },
            Tool {
                name: "list_models".into(),
                description: "List available Ollama models".into(),
                input_schema: schema!({"type": "object"}),
            },
        ])
    }
    
    /// Executa ferramenta chamada
    async fn call_tool(&self, name: &str, arguments: Value) -> Result<CallToolResult> {
        match name {
            "code" => {
                let req: CodeRequest = serde_json::from_value(arguments)?;
                let result = self.orchestrator.code(req).await?;
                Ok(CallToolResult::success(result.output))
            }
            "review" => {
                let req: ReviewRequest = serde_json::from_value(arguments)?;
                let result = self.orchestrator.review(req).await?;
                Ok(CallToolResult::success(result.output))
            }
            "test" => {
                let req: TestRequest = serde_json::from_value(arguments)?;
                let result = self.orchestrator.test(req).await?;
                Ok(CallToolResult::success(result.output))
            }
            "list_models" => {
                let models = self.orchestrator.ollama().list_models().await?;
                let output = models.iter()
                    .map(|m| format!("- {} ({}B, {})", m.name, m.size, m.quantization))
                    .collect::<Vec<_>>()
                    .join("\n");
                Ok(CallToolResult::success(output))
            }
            _ => Err(Error::method_not_found(name)),
        }
    }
}
```

---

## 7. Python Bindings (PyO3)

Para compatibilidade com o ecossistema Python existente:

```rust
// crates/airllm-python/src/lib.rs

use pyo3::prelude::*;
use pyo3::asyncio;

#[pyclass]
pub struct Orchestrator {
    inner: airllm_core::Orchestrator,
}

#[pymethods]
impl Orchestrator {
    #[new]
    fn new() -> PyResult<Self> {
        let rt = tokio::runtime::Runtime::new()
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;
        
        let inner = rt.block_on(async {
            airllm_core::Orchestrator::new().await
        }).map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;
        
        Ok(Self { inner })
    }
    
    /// Gera código a partir de especificação
    fn code(&self, task: &str, language: Option<&str>) -> PyResult<String> {
        Python::with_gil(|py| {
            pyo3::asyncio::run_until_complete(py, async {
                let req = CodeRequest {
                    task: task.to_string(),
                    language: language.map(String::from),
                };
                let result = self.inner.code(req).await
                    .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;
                Ok(result.output)
            })
        })
    }
    
    /// Lista modelos disponíveis
    fn list_models(&self) -> PyResult<Vec<String>> {
        // ...
    }
}

/// Python module
#[pymodule]
fn airllm(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_class::<Orchestrator>()?;
    Ok(())
}
```

```python
# python/airllm/__init__.py
from .airllm import Orchestrator

__all__ = ["Orchestrator"]

# Uso:
# from airllm import Orchestrator
# orch = Orchestrator()
# result = orch.code("implement JWT auth module", language="rust")
```

---

## 8. CLI Tool

```rust
// crates/airllm-cli/src/main.rs

use clap::{Parser, Subcommand};
use airllm_core::Orchestrator;

#[derive(Parser)]
#[command(name = "airllm")]
#[command(about = "Multi-agent code orchestration with Ollama")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
    
    /// Ollama URL (default: http://localhost:11434)
    #[arg(long, env = "OLLAMA_URL", default_value = "http://localhost:11434")]
    ollama_url: String,
}

#[derive(Subcommand)]
enum Commands {
    /// Generate code from specification
    Code {
        /// Task description
        task: String,
        
        /// Programming language
        #[arg(short, long)]
        language: Option<String>,
        
        /// Output directory
        #[arg(short, long, default_value = ".")]
        output: String,
        
        /// Force specific model
        #[arg(short, long)]
        model: Option<String>,
        
        /// Enable streaming output
        #[arg(long)]
        stream: bool,
    },
    
    /// Review code files
    Review {
        /// Files to review
        files: Vec<String>,
        
        /// Model to use
        #[arg(short, long)]
        model: Option<String>,
    },
    
    /// Generate tests
    Test {
        /// Files to generate tests for
        files: Vec<String>,
        
        /// Test framework
        #[arg(short, long)]
        framework: Option<String>,
    },
    
    /// Refactor code
    Refactor {
        /// Files to refactor
        files: Vec<String>,
        
        /// Refactoring goal
        #[arg(short, long)]
        goal: String,
    },
    
    /// Interactive chat mode
    Chat {
        /// Initial prompt
        #[arg(short, long)]
        prompt: Option<String>,
        
        /// Model to use
        #[arg(short, long)]
        model: Option<String>,
    },
    
    /// List available models
    Models,
    
    /// Show model routing rules
    Routes,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let orch = Orchestrator::new(&cli.ollama_url).await?;
    
    match cli.command {
        Commands::Code { task, language, output, model, stream } => {
            let result = orch.code(CodeRequest {
                task,
                language,
                output_dir: output,
                model_override: model,
                stream,
            }).await?;
            
            println!("{}", result.output);
        }
        Commands::Review { files, model } => {
            let result = orch.review(files, model).await?;
            println!("{}", result.output);
        }
        Commands::Test { files, framework } => {
            let result = orch.test(files, framework).await?;
            println!("{}", result.output);
        }
        Commands::Refactor { files, goal } => {
            let result = orch.refactor(files, goal).await?;
            println!("{}", result.output);
        }
        Commands::Chat { prompt, model } => {
            orch.chat_interactive(prompt, model).await?;
        }
        Commands::Models => {
            let models = orch.list_models().await?;
            for m in models {
                println!("  {} ({}B, {})", m.name, m.size, m.quantization);
            }
        }
        Commands::Routes => {
            orch.print_routing_rules();
        }
    }
    
    Ok(())
}
```

**Uso**:
```bash
# Gerar código
airllm code "implement JWT auth module" --language rust --output ./src/auth/

# Review código
airllm review src/auth/*.rs

# Gerar testes
airllm test src/auth/ --framework pytest

# Refatorar
airllm refactor src/legacy/ --goal "modernize to async/await"

# Modo interativo
airllm chat --model qwen3.6:27b

# Listar modelos
airllm models
```

---

## 9. Roadmap de Implementação

### Sprint 1 (Semana 1-2): Fundação Rust

- [ ] Inicializar workspace Cargo
- [ ] Implementar `OllamaClient` (chat, streaming, list_models)
- [ ] Implementar `ModelRouter` com heurísticas
- [ ] Configurar modelos em `config/models.toml`
- [ ] Testes unitários do cliente Ollama
- [ ] Benchmark de latência: Rust vs Python (httpx)

### Sprint 2 (Semana 3-4): Orquestrador e Agentes

- [ ] Implementar `Orchestrator` core
- [ ] Implementar `Agent` executor com `tokio::spawn`
- [ ] Criar definições de agentes em TOML
- [ ] Escrever system prompts
- [ ] Implementar decomposição de tarefas
- [ ] Implementar consolidação de resultados
- [ ] Testes de paralelismo (3+ agentes concorrentes)

### Sprint 3 (Semana 5-6): CLI e UX

- [ ] Implementar CLI com `clap`
- [ ] Terminal UI com `ratatui` (progress bars, streaming)
- [ ] Comandos: code, review, test, refactor, chat
- [ ] Streaming de respostas em tempo real
- [ ] Configuração via arquivo TOML + env vars
- [ ] Testes E2E da CLI

### Sprint 4 (Semana 7-8): MCP Server e Python Bindings

- [ ] Implementar MCP server com `rmcp` crate
- [ ] Expor tools: code, review, test, list_models
- [ ] Python bindings com PyO3
- [ ] Publicar wheel Python
- [ ] Testes de integração MCP com VS Code

### Sprint 5 (Semana 9-10): Otimização e Polish

- [ ] Profiling e otimização de hot paths
- [ ] Cache de prompts (LRU em memória)
- [ ] Reutilização de conexões HTTP (connection pooling)
- [ ] Retry com backoff exponencial
- [ ] Métricas e observabilidade (tracing)
- [ ] Documentação final
- [ ] Publicação no crates.io e PyPI

---

## 10. Métricas de Performance Esperadas

| Métrica | Python (atual) | Rust (esperado) | Melhoria |
|---|---|---|---|
| Overhead de orquestração | 10-50ms | 0.1-1ms | 50-500x |
| Latência de chamada Ollama | 50-100ms (httpx) | 5-15ms (reqwest) | 5-10x |
| Agentes em paralelo | 1 (GIL) ou subprocess | 4-8 (tokio::spawn) | 4-8x |
| Memória base | ~100MB (Python) | ~5-10MB (Rust binary) | 10-20x |
| Startup time | 2-5s (import) | < 50ms (binary) | 40-100x |
| Throughput (tokens/s agregado) | ~50-100 | ~500-1000 | 10x |

---

## 11. Dependências Rust

```toml
# Cargo.toml (workspace)
[workspace.dependencies]
tokio = { version = "1", features = ["full"] }
reqwest = { version = "0.12", features = ["json", "stream"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
toml = "0.8"
clap = { version = "4", features = ["derive"] }
rmcp = "0.1"                    # MCP server
pyo3 = { version = "0.22", features = ["extension-module"] }
ratatui = "0.28"                # Terminal UI
crossterm = "0.28"              # Terminal backend
tracing = "0.1"                 # Logging
tracing-subscriber = "0.3"
rayon = "1.10"                  # Data parallelism
tokio-stream = "0.1"           # Streaming
futures = "0.3"                 # Async utilities
anyhow = "1"                    # Error handling
thiserror = "1"                # Custom errors
regex = "1"                     # Pattern matching
memmap2 = "0.9"                 # Memory-mapped files
parking_lot = "0.12"            # Fast Mutex/RwLock
```

---

## 12. Comparação com o Plano Anterior

| Critério | Plano Anterior | Plano Revisado |
|---|---|---|
| **Foco** | Modernizar inferência layer-wise | Multi-agentes de codificação |
| **Backend de LLM** | HuggingFace safetensors | Ollama (já rodando localmente) |
| **Linguagem** | Python → Rust (Fase 3 opcional) | **Rust desde o início** |
| **Modelos** | Llama, QWen, Mistral (safetensors) | **Qwen 3.5/3.6, Kimi K2.7, GLM-5.2** (via Ollama) |
| **Complexidade** | Alta ( reimplementar inferência) | **Média** (usar Ollama como backend) |
| **Tempo até MVP** | 2-4 semanas (Fase 1) | **2 semanas** (Sprint 1) |
| **Performance** | Melhorar I/O e KV cache | **Rust + tokio + paralelismo real** |
| **Caso de uso** | Inferência de LLM em low-VRAM | **Orquestração de agentes para codificação** |
| **MCP** | Não mencionado | **Sim — substitui edgesearch_mcp em Rust** |
| **Python** | Mantido como principal | **Apenas bindings (PyO3) — opcional** |

---

## 13. Riscos e Mitigações

| Risco | Probabilidade | Impacto | Mitigação |
|---|---|---|---|
| `rmcp` crate imaturo para MCP server | Média | Alto | Fallback: MCP server em Python que chama o binary Rust via subprocess |
| PyO3 build complexo em Windows | Média | Médio | CI com cross-compilation; publicar wheels para Linux/Mac/Windows |
| Ollama não suporta múltiplas requisições concorrentes bem | Baixa | Alto | Testar com 4+ agentes em paralelo; usar múltiplas instâncias se necessário |
| Modelos Qwen locais sem VRAM suficiente para paralelo | Alta | Médio | Roteamento inteligente: 1 modelo pesado + múltiplos leves em paralelo |
| System prompts não produzem código de qualidade | Média | Alto | Iterar prompts com benchmarks; usar few-shot examples |
| `qwen3-coder-next:q8_0` (84GB) muito lento para uso interativo | Alta | Médio | Usar apenas para tarefas não-interativas; default para `qwen3.6:27b` |

---

## 14. Conclusão

O plano anterior focava em modernizar o motor de inferência layer-wise do AirLLM. Este plano revisado **muda o foco para o que realmente importa**: construir um sistema de **multi-agentes de codificação de alta performance** usando Rust, com modelos Qwen já disponíveis no Ollama.

**Vantagens chave**:
1. **Não reinventa a roda** — Ollama já faz inferência; focamos em orquestração
2. **Rust desde o início** — sem GIL, sem GC, paralelismo real
3. **Modelos já disponíveis** — Qwen 3.5 (4B), Qwen 3.6 (27B), Qwen3-Coder-Next (80B), mais cloud
4. **MCP nativo em Rust** — substitui o server Python por algo 10x mais eficiente
5. **Python opcional** — bindings PyO3 para quem precisa, mas Rust é cidadão de primeira classe
6. **MVP em 2 semanas** — apenas cliente Ollama + router + CLI básica