# Roadmap Paralelo — AirLLM v3.0 em 3 Frentes

> **Data**: 2026-06-27
> **Autor**: EdgeSearch Orquestrador
> **Revisor**: Erik Tonon
> **Status**: Em integração final — Tracks 1, 2 e 3 validados no workspace local
> **Plano base**: `docs/PLANO_REVISADO_V3.md`

---

## Estratégia de Paralelização

Três IAs trabalham simultaneamente em 3 frentes independentes. Cada frente produz um crate Rust isolado com interfaces bem definidas. A integração acontece ao final.

```
                    ┌─────────────────────────┐
                    │    WORKSPACE RAIZ        │
                    │    Cargo.toml (workspace) │
                    └────────┬────────────────┘
                             │
           ┌─────────────────┼─────────────────┐
           │                 │                 │
           ▼                 ▼                 ▼
    ┌──────────────┐ ┌──────────────┐ ┌──────────────┐
    │  TRACK 1     │ │  TRACK 2     │ │  TRACK 3     │
    │  GLM-5.2     │ │  GPT-5.4-pro │ │  5.1-codex   │
    │              │ │              │ │  -max        │
    │  airllm-     │ │  airllm-     │ │  airllm-     │
    │  ollama      │ │  orchestrator│ │  cli         │
    │              │ │              │ │              │
    │  Cliente     │ │  Orquestra-  │ │  CLI + TUI   │
    │  Ollama +    │ │  dor + Agentes│ │  + MCP      │
    │  Router      │ │  + Prompts   │ │  Server      │
    └──────┬───────┘ └──────┬───────┘ └──────┬───────┘
           │                │                 │
           │   Interfaces    │                 │
           └────────────────┼─────────────────┘
                            │
                    ┌───────▼────────┐
                    │  INTEGRAÇÃO     │
                    │  (Track 4)      │
                    │  Eu faço no     │
                    │  final          │
                    └────────────────┘
```

### Atribuição de IAs

| Track | IA | Crate | Por quê |
|---|---|---|---|
| **Track 1** | GLM-5.2 (limite de tokens) | `airllm-ollama` | Tarefa bem delimitada: cliente HTTP + tipos + router. Menos código, menos tokens. |
| **Track 2** | GPT-5.4-pro (sem limite) | `airllm-orchestrator` | Tarefa mais complexa: orquestrador + 7 agentes + prompts. Precisa de muito contexto. |
| **Track 3** | 5.1-codex-max (sem limite) | `airllm-cli` + `airllm-mcp` | Tarefa de produto: CLI, TUI, MCP server. Foco em UX e integração. |

### Status de Execução em 2026-06-27

- `airllm-ollama` validado com `cargo test`, `cargo clippy` e `cargo doc`
- `airllm-orchestrator` implementado sem stub, modularizado e validado com testes próprios
- Workspace completo validado com `cargo build --workspace` e `cargo test --workspace`
- Ollama local validado via CLI real com `qwen3.5:4b` em `models`, `chat` e `code`
- Benchmark atualizado documentado em `docs/BENCHMARK_OLLAMA_LOCAL_MODELS_2026-06-27.md`
- MCP stdio validado com chamada real `list_models`
- Python bindings validados com import direto do `.so` gerado em `target/debug`

---

## Interfaces Compartilhadas (Contratos)

Cada track deve respeitar estas interfaces. Todos os crates estão no mesmo workspace Cargo.

### Interface 1: `airllm-ollama` (produzido pelo Track 1)

```rust
// crates/airllm-ollama/src/lib.rs — API pública

/// Cliente Ollama assíncrono
pub struct OllamaClient { ... }

impl OllamaClient {
    pub fn new(base_url: &str) -> Self;
    pub async fn chat(&self, model: &str, messages: Vec<Message>, options: ChatOptions) 
        -> Result<String>;
    pub async fn chat_stream(&self, model: &str, messages: Vec<Message>) 
        -> Result<impl Stream<Item = Result<String>>>;
    pub async fn list_models(&self) -> Result<Vec<ModelInfo>>;
    pub async fn model_available(&self, model: &str) -> Result<bool>;
}

/// Mensagem de chat
#[derive(Serialize, Deserialize, Clone)]
pub struct Message {
    pub role: MessageRole,    // System | User | Assistant
    pub content: String,
}

/// Opções de geração
#[derive(Serialize, Deserialize, Default)]
pub struct ChatOptions {
    pub temperature: f32,
    pub top_p: f32,
    pub top_k: u32,
    pub num_ctx: u32,
}

/// Info de modelo
#[derive(Deserialize)]
pub struct ModelInfo {
    pub name: String,
    pub size: String,
    pub quantization: String,
}

/// Router de modelos
pub struct ModelRouter { ... }

impl ModelRouter {
    pub fn new() -> Self;
    pub fn classify(&self, request: &str) -> Complexity;
    pub fn select_model(&self, complexity: &Complexity) -> &str;
}

#[derive(Clone, Copy)]
pub enum Complexity { Low, Medium, High, Cloud }
```

### Interface 2: `airllm-orchestrator` (produzido pelo Track 2)

```rust
// crates/airllm-orchestrator/src/lib.rs — API pública

use airllm_ollama::{OllamaClient, ModelRouter, Message, ChatOptions};

/// Orquestrador principal
pub struct Orchestrator {
    ollama: OllamaClient,
    router: ModelRouter,
    agents: AgentRegistry,
}

impl Orchestrator {
    pub fn new(ollama: OllamaClient) -> Self;
    pub async fn code(&self, request: CodeRequest) -> Result<CodeResponse>;
    pub async fn review(&self, files: Vec<String>) -> Result<ReviewResponse>;
    pub async fn test(&self, files: Vec<String>) -> Result<TestResponse>;
    pub async fn refactor(&self, files: Vec<String>, goal: &str) -> Result<RefactorResponse>;
    pub async fn chat(&self, prompt: &str, model: Option<&str>) -> Result<String>;
}

/// Request de código
pub struct CodeRequest {
    pub task: String,
    pub language: Option<String>,
    pub files: Vec<String>,
    pub model_override: Option<String>,
}

/// Response de código
pub struct CodeResponse {
    pub output: String,
    pub files_written: Vec<String>,
    pub agent_used: String,
    pub model_used: String,
}

/// Registro de agentes
pub struct AgentRegistry { ... }

impl AgentRegistry {
    pub fn new() -> Self;
    pub fn register(&mut self, agent: Agent);
    pub fn get(&self, name: &str) -> Option<&Agent>;
}

/// Agente individual
pub struct Agent {
    pub name: String,
    pub model: String,
    pub system_prompt: String,
    pub parallelizable: bool,
}

impl Agent {
    pub async fn execute(&self, task: &SubTask, ollama: &OllamaClient) -> Result<AgentResult>;
}

/// Sub-tarefa decomposta
pub struct SubTask {
    pub id: String,
    pub description: String,
    pub agent_name: String,
    pub input_files: Vec<String>,
}

/// Resultado de um agente
pub struct AgentResult {
    pub task_id: String,
    pub output: String,
    pub files: Vec<String>,
    pub success: bool,
}
```

### Interface 3: `airllm-cli` (produzido pelo Track 3)

```rust
// crates/airllm-cli/src/main.rs — depende de airllm-orchestrator

use airllm_orchestrator::{Orchestrator, CodeRequest};

// Comandos CLI via clap:
//   airllm code "task" --language rust --output ./src/
//   airllm review src/*.rs
//   airllm test src/ --framework pytest
//   airllm refactor src/ --goal "modernize to async"
//   airllm chat --model qwen3.6:27b
//   airllm models
//   airllm routes

// crates/airllm-mcp/src/lib.rs — MCP server
//   Tools: code, review, test, list_models
//   Depende de airllm-orchestrator
```

---

## TRACK 1 — GLM-5.2 (limite de tokens)

### Crate: `airllm-ollama`

**Responsabilidade**: Cliente Ollama HTTP async + tipos compartilhados + Model Router

**Por que GLM-5.2**: É a tarefa mais autocontida. Menor volume de código. Tipos e cliente HTTP são diretos. O limite de tokens do GLM-5.2 não é problema porque cada arquivo é pequeno.

### Estrutura de arquivos

```
crates/airllm-ollama/
├── Cargo.toml
├── src/
│   ├── lib.rs              # exports públicos
│   ├── client.rs           # OllamaClient (reqwest async)
│   ├── types.rs            # Message, ChatOptions, ModelInfo, Complexity
│   ├── router.rs           # ModelRouter com heurísticas
│   ├── stream.rs           # chat_stream (SSE parsing)
│   └── error.rs            # OllamaError (thiserror)
└── tests/
    ├── test_client.rs      # testes unitários do cliente
    ├── test_router.rs      # testes do router
    └── test_stream.rs      # testes de streaming
```

### Tarefas (checklist)

#### T1.1 — Setup do crate
- [x] Criar `crates/airllm-ollama/Cargo.toml`
- [x] Dependências: `reqwest` (json, stream), `serde` (derive), `serde_json`, `tokio`, `tokio-stream`, `thiserror`, `regex`
- [x] Criar `src/lib.rs` com exports

#### T1.2 — Tipos compartilhados (`types.rs`)
- [x] `enum MessageRole { System, User, Assistant }`
- [x] `struct Message { role, content }`
- [x] `struct ChatOptions { temperature, top_p, top_k, num_ctx }` com `Default`
- [x] `struct ModelInfo { name, size, quantization }`
- [x] `enum Complexity { Low, Medium, High, Cloud }`
- [x] Todos com `Serialize`/`Deserialize`

#### T1.3 — Error handling (`error.rs`)
- [x] `enum OllamaError` com variantes: `Connection`, `Http`, `Json`, `ModelNotFound`, `StreamParse`
- [x] Implementar `thiserror::Error`
- [x] Implementar `From<reqwest::Error>` e `From<serde_json::Error>`

#### T1.4 — Cliente Ollama (`client.rs`)
- [x] `struct OllamaClient { base_url: String, http: reqwest::Client }`
- [x] `new(base_url: &str) -> Self` com timeout de 300s
- [x] `async fn chat(model, messages, options) -> Result<String>`
- [x] `async fn list_models() -> Result<Vec<ModelInfo>>`
- [x] `async fn model_available(model) -> Result<bool>`
- [x] Endpoint: `POST /api/chat` (non-streaming)
- [x] Endpoint: `GET /api/tags` (list models)

#### T1.5 — Streaming (`stream.rs`)
- [x] `async fn chat_stream(model, messages) -> Result<impl Stream<Item = Result<String>>>`
- [x] Parse de SSE chunks (cada linha é JSON com `message.content`)
- [x] Usar `bytes_stream()` do reqwest + `tokio_stream::StreamMap`

#### T1.6 — Model Router (`router.rs`)
- [x] `struct ModelRouter { rules: Vec<RoutingRule> }`
- [x] `struct RoutingRule { pattern: Regex, complexity: Complexity, model: String }`
- [x] `fn new() -> Self` com regras padrão:
  - `rename|format|complete|lint` → Low → `qwen3.5:4b`
  - `implement|create|fix|test|review` → Medium → `qwen3.6:27b`
  - `architect|refactor|design|debug complex` → High → `qwen3-coder-next:q8_0`
  - `orchestrat|plan|strategy` → Cloud → `qwen3.5:397b-cloud`
- [x] `fn classify(request: &str) -> Complexity`
- [x] `fn select_model(complexity: &Complexity) -> &str`

#### T1.7 — Testes
- [x] Testar `chat()` com mock (usar `mockito` crate)
- [x] Testar `list_models()` com mock
- [x] Testar `classify()` com vários inputs
- [x] Testar `select_model()` para cada Complexidade
- [x] Testar parsing de stream

### Entregáveis finais
- [x] Crate `airllm-ollama` compilando sem erros
- [x] `cargo test` passando
- [x] `cargo clippy` sem warnings
- [x] Documentação (`cargo doc`)
- [x] **API pública estável** — Track 2 e 3 dependem dela

### Prompt para colar na IA Track 1 (GLM-5.2)

```
Você vai implementar um crate Rust chamado `airllm-ollama` que é um cliente assíncrono para a API do Ollama. 

Crie o workspace Cargo na raiz do projeto /home/eriktonon/airllm/ com:
- Cargo.toml (workspace)
- crates/airllm-ollama/Cargo.toml
- crates/airllm-ollama/src/lib.rs, client.rs, types.rs, router.rs, stream.rs, error.rs

O cliente deve usar reqwest (async, com features json e stream), serde, tokio, thiserror e regex.

API pública necessária (NÃO mudar os nomes — outros crates dependem disso):

1. OllamaClient::new(base_url) -> Self
2. OllamaClient::chat(model, messages, options) -> Result<String>  (POST /api/chat, stream=false)
3. OllamaClient::chat_stream(model, messages) -> Result<impl Stream>  (POST /api/chat, stream=true, SSE)
4. OllamaClient::list_models() -> Result<Vec<ModelInfo>>  (GET /api/tags)
5. OllamaClient::model_available(model) -> Result<bool>
6. ModelRouter::new() -> Self  (com regras padrão)
7. ModelRouter::classify(request: &str) -> Complexity
8. ModelRouter::select_model(complexity) -> &str

Tipos: Message { role: MessageRole, content: String }, MessageRole { System, User, Assistant }, ChatOptions { temperature, top_p, top_k, num_ctx }, ModelInfo { name, size, quantization }, Complexity { Low, Medium, High, Cloud }.

Regras do router:
- "rename|format|complete|lint" → Low → "qwen3.5:4b"
- "implement|create|fix|test|review" → Medium → "qwen3.6:27b"  
- "architect|refactor|design|debug" → High → "qwen3-coder-next:q8_0"
- "orchestrat|plan|strategy" → Cloud → "qwen3.5:397b-cloud"

Ollama API: base_url padrão = "http://localhost:11434". Endpoint de chat: POST /api/chat com JSON { model, messages: [{role, content}], stream, options }. Resposta non-stream: { message: { role, content }, done: true }. Resposta stream: múltiplas linhas JSON, cada uma com { message: { content: "token" }, done: false } e a última com { done: true }.

Endpoint de models: GET /api/tags retorna { models: [{ name, size, details: { quantization_level } }] }.

Use timeout de 300s no reqwest. Implemente thiserror para erros. Escreva testes unitários com mockito.
```

---

## TRACK 2 — GPT-5.4-pro (sem limite de tokens)

### Crate: `airllm-orchestrator`

**Responsabilidade**: Orquestrador principal + 7 agentes de codificação + decomposição de tarefas + consolidação de resultados + system prompts

**Por que GPT-5.4-pro**: É a tarefa mais complexa. Requer muito contexto (prompts longos, lógica de decomposição, padrões de consolidação). Precisa de raciocínio profundo para design do orquestrador.

**Status atual**: Implementado em módulos reais, sem stub, com fallback automático para modelos locais quando um modelo preferencial não estiver disponível.

### Estrutura de arquivos

```
crates/airllm-orchestrator/
├── Cargo.toml
├── src/
│   ├── lib.rs               # exports públicos
│   ├── orchestrator.rs      # Orchestrator struct + code/review/test/refactor/chat
│   ├── agent.rs             # Agent struct + execute()
│   ├── registry.rs          # AgentRegistry
│   ├── decompose.rs         # Decomposição de tarefas em sub-tarefas
│   ├── consolidate.rs       # Consolidação de resultados paralelos
│   ├── types.rs             # CodeRequest, CodeResponse, SubTask, AgentResult, etc.
│   └── error.rs             # OrchestratorError
├── prompts/
│   ├── coder.md             # System prompt do agente Coder
│   ├── reviewer.md          # System prompt do Reviewer
│   ├── tester.md            # System prompt do Tester
│   ├── architect.md         # System prompt do Architect
│   ├── debugger.md          # System prompt do Debugger
│   ├── refactorer.md        # System prompt do Refactorer
│   └── documenter.md        # System prompt do Documenter
├── agents/
│   ├── coder.toml           # Config do agente Coder
│   ├── reviewer.toml
│   ├── tester.toml
│   ├── architect.toml
│   ├── debugger.toml
│   ├── refactorer.toml
│   └── documenter.toml
└── tests/
    ├── test_orchestrator.rs
    ├── test_decompose.rs
    ├── test_consolidate.rs
    └── test_agents.rs
```

### Tarefas (checklist)

#### T2.1 — Setup do crate
- [x] Criar `crates/airllm-orchestrator/Cargo.toml`
- [x] Dependências: `airllm-ollama` (path), `tokio` (full), `serde` (derive), `serde_json`, `toml`, `futures`, `async-trait`, `thiserror`, `tracing`, `parking_lot`
- [x] Criar `src/lib.rs` com exports

#### T2.2 — Tipos (`types.rs`)
- [x] `struct CodeRequest { task, language, files, model_override }`
- [x] `struct CodeResponse { output, files_written, agent_used, model_used }`
- [x] `struct ReviewRequest { files }` / `struct ReviewResponse { output }`
- [x] `struct TestRequest { files, framework }` / `struct TestResponse { output }`
- [x] `struct RefactorRequest { files, goal }` / `struct RefactorResponse { output }`
- [x] `struct SubTask { id, description, agent_name, input_files }`
- [x] `struct AgentResult { task_id, output, files, success }`
- [x] `struct AgentConfig { name, model, system_prompt, parallelizable, max_concurrent, temperature, top_p }` (desserializado de TOML)

#### T2.3 — Agent Registry (`registry.rs`)
- [x] `struct AgentRegistry { agents: HashMap<String, Agent> }`
- [x] `fn new() -> Self`
- [x] `fn register(&mut self, agent: Agent)`
- [x] `fn get(&self, name: &str) -> Option<&Agent>`
- [x] `fn load_from_dir(dir: &Path) -> Result<Self>` — carrega `.toml` files
- [x] `fn list(&self) -> Vec<&Agent>`

#### T2.4 — Agent (`agent.rs`)
- [x] `struct Agent { name, model, system_prompt, parallelizable, max_concurrent, config }`
- [x] `async fn execute(&self, task: &SubTask, ollama: &OllamaClient) -> Result<AgentResult>`
- [x] Constrói mensagens: [System(system_prompt), User(task.description + file contents)]
- [x] Chama `ollama.chat(model, messages, options)`
- [x] Parse output: extrai code blocks, identifica arquivos criados
- [x] `fn load_prompt(path: &Path) -> Result<String>` — carrega `.md` file

#### T2.5 — Decomposição (`decompose.rs`)
- [x] `async fn decompose(request: &CodeRequest, ollama: &OllamaClient) -> Result<Vec<SubTask>>`
- [x] Usa o modelo Architect (qwen3-coder-next ou cloud) para dividir a tarefa
- [x] Prompt: "Dada a tarefa X, divida em sub-tarefas independentes. Retorne JSON: [{id, description, agent_name, input_files}]"
- [x] Parse JSON response → `Vec<SubTask>`
- [x] Fallback: se decomposição falhar, cria uma única sub-tarefa com a tarefa original

#### T2.6 — Consolidação (`consolidate.rs`)
- [x] `async fn consolidate(results: Vec<AgentResult>, ollama: &OllamaClient) -> Result<CodeResponse>`
- [x] Se 1 resultado: retorna direto
- [x] Se múltiplos: usa Reviewer agent para consolidar
- [x] Prompt: "Consolide os seguintes resultados de código em uma resposta coerente"
- [x] Extrai arquivos escritos de todos os resultados
- [x] Identifica conflitos (mesmo arquivo modificado por múltiplos agentes)

#### T2.7 — Orchestrator (`orchestrator.rs`)
- [x] `struct Orchestrator { ollama, router, agents }`
- [x] `fn new(ollama: OllamaClient) -> Self` — carrega agentes de `agents/`
- [x] `async fn code(&self, request: CodeRequest) -> Result<CodeResponse>`
  1. `router.classify(&request.task)` → complexity
  2. `decompose(&request, &self.ollama)` → subtasks
  3. `execute_parallel(subtasks)` → results
  4. `consolidate(results, &self.ollama)` → response
- [x] `async fn execute_parallel(subtasks: Vec<SubTask>) -> Result<Vec<AgentResult>>`
  - `futures::future::join_all` sobre `tokio::spawn` para cada sub-tarefa
  - Respeita `max_concurrent` por agente (semáforo)
- [x] `async fn review(files: Vec<String>) -> Result<ReviewResponse>`
- [x] `async fn test(files: Vec<String>, framework: Option<String>) -> Result<TestResponse>`
- [x] `async fn refactor(files: Vec<String>, goal: String) -> Result<RefactorResponse>`
- [x] `async fn chat(prompt: &str, model: Option<&str>) -> Result<String>`

#### T2.8 — System Prompts (`prompts/*.md`)
- [x] `coder.md` — "You are an expert code generation agent..."
- [x] `reviewer.md` — "You are a code review agent. Analyze code for bugs, security, performance..."
- [x] `tester.md` — "You are a test generation agent. Write comprehensive tests..."
- [x] `architect.md` — "You are a software architecture agent. Design module structure..."
- [x] `debugger.md` — "You are a debugging agent. Analyze errors and propose fixes..."
- [x] `refactorer.md` — "You are a refactoring agent. Improve code quality..."
- [x] `documenter.md` — "You are a documentation agent. Generate clear docs..."

#### T2.9 — Configs TOML (`agents/*.toml`)
- [x] Cada arquivo define: name, default_model, fallback_model, parallelizable, max_concurrent, temperature, top_p, system_prompt path, routing patterns

#### T2.10 — Testes
- [x] Testar decomposição com mock
- [x] Testar execução paralela (3 sub-tarefas)
- [x] Testar consolidação
- [x] Testar registry carregando TOML
- [x] Testar agent.execute() com mock

### Entregáveis finais
- [x] Crate `airllm-orchestrator` compilando
- [x] `cargo test` passando
- [x] `cargo clippy` sem warnings
- [x] 7 system prompts escritos
- [x] 7 configs TOML
- [x] **API pública estável** — Track 3 depende dela

### Prompt para colar na IA Track 2 (GPT-5.4-pro)

```
Você vai implementar um crate Rust chamado `airllm-orchestrator` que é o orquestrador de multi-agentes de codificação do AirLLM v3.0.

O workspace Cargo já existe em /home/eriktonon/airllm/ com o crate `airllm-ollama` (cliente Ollama). Você deve criar:

crates/airllm-orchestrator/ com:
- Cargo.toml (depende de airllm-ollama via path = "../airllm-ollama")
- src/lib.rs, orchestrator.rs, agent.rs, registry.rs, decompose.rs, consolidate.rs, types.rs, error.rs
- prompts/coder.md, reviewer.md, tester.md, architect.md, debugger.md, refactorer.md, documenter.md
- agents/coder.toml, reviewer.toml, tester.toml, architect.toml, debugger.toml, refactorer.toml, documenter.toml
- tests/test_orchestrator.rs, test_decompose.rs, test_consolidate.rs

O crate airllm-ollama fornece: OllamaClient (chat, chat_stream, list_models, model_available), ModelRouter (classify, select_model), Message, ChatOptions, Complexity.

Você deve implementar:

1. Orchestrator: recebe CodeRequest, classifica complexidade via ModelRouter, decomposta em sub-tarefas usando o agente Architect, executa agentes em paralelo com tokio::spawn + futures::join_all, consolida resultados.

2. AgentRegistry: carrega agentes de arquivos TOML em agents/. Cada TOML tem: name, default_model, fallback_model, parallelizable, max_concurrent, temperature, top_p, system_prompt (path para .md).

3. Agent: executa uma sub-tarefa chamando ollama.chat() com system_prompt + task. Faz parse do output para extrair code blocks e arquivos.

4. Decompose: usa o agente Architect para dividir uma tarefa complexa em sub-tarefas. Retorna Vec<SubTask>. Fallback: tarefa única se decomposição falhar.

5. Consolidate: junta resultados de múltiplos agentes. Se houver conflitos (mesmo arquivo), usa Reviewer para resolver.

7 agentes: Coder (qwen3.6:27b), Reviewer (qwen3.6:27b), Tester (qwen3.5:4b), Architect (qwen3-coder-next:q8_0), Debugger (qwen3-coder-next:q8_0), Refactorer (qwen3.6:27b), Documenter (qwen3.5:4b).

Use tokio (full), futures, async-trait, serde, toml, thiserror, tracing, parking_lot. Implemente testes com mockito para o OllamaClient.
```

---

## TRACK 3 — 5.1-codex-max (sem limite de tokens)

### Crates: `airllm-cli` + `airllm-mcp`

**Responsabilidade**: CLI tool (clap + ratatui) + MCP Server (rmcp) + Python bindings (PyO3)

**Por que 5.1-codex-max**: Foco em produto e UX. CLI com TUI rica, MCP server, e bindings Python. Codex é otimizado para código e integração.

### Estrutura de arquivos

```
crates/airllm-cli/
├── Cargo.toml
├── src/
│   ├── main.rs             # Entry point + clap
│   ├── commands/
│   │   ├── mod.rs
│   │   ├── code.rs         # airllm code "task" --lang rust
│   │   ├── review.rs       # airllm review src/*.rs
│   │   ├── test.rs          # airllm test src/ --framework pytest
│   │   ├── refactor.rs     # airllm refactor src/ --goal "..."
│   │   ├── chat.rs          # airllm chat --model qwen3.6:27b
│   │   ├── models.rs       # airllm models
│   │   └── routes.rs       # airllm routes
│   ├── tui/
│   │   ├── mod.rs
│   │   ├── app.rs           # Estado da TUI
│   │   ├── ui.rs            # Layout ratatui
│   │   └── stream.rs        # Render streaming em tempo real
│   └── config.rs            # Config file (TOML) + env vars

crates/airllm-mcp/
├── Cargo.toml
├── src/
│   ├── lib.rs              # MCP server setup
│   ├── server.rs           # ServerHandler impl
│   ├── tools.rs            # Tool definitions (code, review, test, list_models)
│   └── error.rs

crates/airllm-python/
├── Cargo.toml
├── src/
│   └── lib.rs              # PyO3 bindings

python/
└── airllm/
    ├── __init__.py          # from .airllm import Orchestrator
    └── __init__.pyi         # Type stubs
```

### Tarefas (checklist)

#### T3.1 — Setup dos crates
- [ ] Criar `crates/airllm-cli/Cargo.toml` (depende de `airllm-orchestrator`)
- [ ] Criar `crates/airllm-mcp/Cargo.toml` (depende de `airllm-orchestrator`)
- [ ] Criar `crates/airllm-python/Cargo.toml` (depende de `airllm-orchestrator`, `pyo3`)
- [ ] Dependências CLI: `clap` (derive), `ratatui`, `crossterm`, `tokio`, `airllm-orchestrator`, `airllm-ollama`
- [ ] Dependências MCP: `rmcp`, `serde_json`, `airllm-orchestrator`, `tokio`
- [ ] Dependências Python: `pyo3` (extension-module), `airllm-orchestrator`, `tokio`

#### T3.2 — CLI: estrutura base (`main.rs`)
- [ ] `#[derive(Parser)] struct Cli { command: Commands, ollama_url }`
- [ ] `enum Commands { Code, Review, Test, Refactor, Chat, Models, Routes }`
- [ ] `#[tokio::main] async fn main()` — inicializa Orchestrator e despacha comando
- [ ] Flag `--ollama-url` (env: `OLLAMA_URL`, default: `http://localhost:11434`)
- [ ] Flag `--config` (path para config.toml opcional)

#### T3.3 — CLI: comando `code` (`commands/code.rs`)
- [ ] Args: `task: String`, `--language: Option<String>`, `--output: String` (default "."), `--model: Option<String>`, `--stream: bool`
- [ ] Constrói `CodeRequest` e chama `orchestrator.code(request).await`
- [ ] Se `--stream`: renderiza tokens em tempo real via TUI
- [ ] Se não: imprime resultado final

#### T3.4 — CLI: comando `review` (`commands/review.rs`)
- [ ] Args: `files: Vec<String>`, `--model: Option<String>`
- [ ] Chama `orchestrator.review(files).await`

#### T3.5 — CLI: comando `test` (`commands/test.rs`)
- [ ] Args: `files: Vec<String>`, `--framework: Option<String>`
- [ ] Chama `orchestrator.test(files, framework).await`

#### T3.6 — CLI: comando `refactor` (`commands/refactor.rs`)
- [ ] Args: `files: Vec<String>`, `--goal: String`
- [ ] Chama `orchestrator.refactor(files, goal).await`

#### T3.7 — CLI: comando `chat` (`commands/chat.rs`)
- [ ] Args: `--prompt: Option<String>`, `--model: Option<String>`
- [ ] Modo interativo: lê input do usuário, envia, mostra resposta com streaming
- [ ] Loop até usuário digitar `/exit`

#### T3.8 — CLI: comando `models` e `routes` (`commands/models.rs`, `commands/routes.rs`)
- [ ] `models`: lista modelos Ollama disponíveis
- [ ] `routes`: mostra regras de roteamento do ModelRouter

#### T3.9 — TUI (`tui/`)
- [ ] `app.rs`: estado da TUI (input, output, status, progress)
- [ ] `ui.rs`: layout com ratatui — painel de input, painel de output, status bar
- [ ] `stream.rs`: renderiza tokens streaming em tempo real
- [ ] Usa `crossterm` como backend
- [ ] Suporte a cores e syntax highlighting básico

#### T3.10 — Config (`config.rs`)
- [ ] `struct Config { ollama_url, default_model, agents_dir, prompts_dir }`
- [ ] Carrega de `~/.airllm/config.toml` ou env vars
- [ ] Defaults: ollama_url = `http://localhost:11434`

#### T3.11 — MCP Server (`airllm-mcp/server.rs`)
- [ ] `struct AirLLMMcpServer { orchestrator: Arc<Orchestrator> }`
- [ ] `impl ServerHandler for AirLLMMcpServer`
- [ ] `list_tools()` → retorna tools: code, review, test, list_models
- [ ] `call_tool(name, arguments)` → despacha para orchestrator
- [ ] Cada tool tem `input_schema` em JSON Schema
- [ ] `fn main()` — inicia server (stdio transport)

#### T3.12 — Python bindings (`airllm-python/lib.rs`)
- [ ] `#[pyclass] struct PyOrchestrator`
- [ ] `#[new] fn new() -> PyResult<Self>`
- [ ] `fn code(&self, task: &str, language: Option<&str>) -> PyResult<String>`
- [ ] `fn list_models(&self) -> PyResult<Vec<String>>`
- [ ] `#[pymodule] fn airllm(py, m)`
- [ ] Type stubs em `python/airllm/__init__.pyi`

#### T3.13 — Testes
- [ ] Testar CLI com args (clap)
- [ ] Testar MCP server tools
- [ ] Testar Python bindings (pytest)

### Entregáveis finais
- [ ] Crate `airllm-cli` compilando — `cargo run -- code "test"`
- [ ] Crate `airllm-mcp` compilando — MCP server funcional
- [ ] Crate `airllm-python` compilando — `python -c "import airllm"`
- [ ] `cargo clippy` sem warnings em todos os crates
- [ ] TUI funcional com streaming

### Prompt para colar na IA Track 3 (5.1-codex-max)

```
Você vai implementar três crates Rust para o AirLLM v3.0: `airllm-cli` (CLI tool), `airllm-mcp` (MCP server), e `airllm-python` (Python bindings via PyO3).

O workspace Cargo já existe em /home/eriktonon/airllm/ com os crates `airllm-ollama` e `airllm-orchestrator`. Você deve criar:

1. crates/airllm-cli/ — CLI com clap + ratatui (TUI com streaming):
   - Comandos: code "task" --lang rust, review files, test files --framework, refactor files --goal, chat --model, models, routes
   - TUI com ratatui: painel de input, painel de output com streaming, status bar
   - Config via ~/.airllm/config.toml ou env vars (OLLAMA_URL)
   - Depende de airllm-orchestrator

2. crates/airllm-mcp/ — MCP server com rmcp crate:
   - Tools: code (gera código), review (revisa), test (gera testes), list_models (lista modelos)
   - Cada tool tem input_schema em JSON Schema
   - Transport: stdio
   - Depende de airllm-orchestrator

3. crates/airllm-python/ — Python bindings com PyO3:
   - #[pyclass] PyOrchestrator com métodos code() e list_models()
   - #[pymodule] airllm
   - Type stubs em python/airllm/__init__.pyi
   - Depende de airllm-orchestrator

O crate airllm-orchestrator fornece: Orchestrator (code, review, test, refactor, chat), CodeRequest, CodeResponse, ReviewRequest, etc.

O crate airllm-ollama fornece: OllamaClient, ModelRouter, ModelInfo.

Use clap (derive), ratatui, crossterm, tokio, rmcp, pyo3 (extension-module). Implemente testes.
```

---

## Dependências entre Tracks

```
Track 1 (airllm-ollama)     ← NÃO depende de ninguém
    ↑
Track 2 (airllm-orchestrator) ← depende de Track 1
    ↑
Track 3 (airllm-cli/mcp/python) ← depende de Track 2
```

### Como rodar em paralelo sem bloquear

| Track | Pode começar imediatamente? | O que fazer enquanto Track 1 não terminou |
|---|---|---|
| **Track 1** | ✅ Sim | — |
| **Track 2** | ✅ Sim | Criar estrutura, tipos, prompts, configs TOML. Usar stubs/mocks de OllamaClient. |
| **Track 3** | ✅ Sim | Criar estrutura CLI, clap args, TUI layout, MCP tool definitions. Usar stubs de Orchestrator. |

**Cada track cria seus próprios mocks/stubs** para as dependências que ainda não existem. Quando todos terminam, eu faço a integração removendo os stubs e conectando os crates reais.

---

## Integração Final (Track 4 — eu faço)

Após as 3 IAs terminarem:

1. **Remover stubs/mocks** de cada crate
2. **Conectar dependências** via `path = "../airllm-xxx"` no Cargo.toml
3. **Resolver conflitos de tipos** entre crates
4. **Testar build completo** do workspace: `cargo build --workspace`
5. **Rodar todos os testes**: `cargo test --workspace`
6. **Testar E2E**: `cargo run -- code "implement hello world" --lang rust`
7. **Testar MCP**: conectar VS Code ao MCP server
8. **Testar Python**: `python -c "from airllm import Orchestrator; o = Orchestrator(); print(o.list_models())"`
9. **Atualizar context bank** com resultado da integração
10. **Git workflow**: branch, commit, merge

---

## Checklist de Coordenação

### Antes de iniciar as 3 IAs

- [x] Plano revisado aprovado (`docs/PLANO_REVISADO_V3.md`)
- [x] Interfaces definidas (este documento)
- [x] Modelos Qwen catalogados no Ollama
- [x] Context bank atualizado (`.edgesearch/context.md`)
- [x] Workspace Cargo inicializado (eu crio antes de iniciar as IAs)
- [x] Prompts preparados para cada IA (já neste documento)

### Após as 3 IAs terminarem

- [x] Track 1: `airllm-ollama` compila e testa OK
- [x] Track 2: `airllm-orchestrator` compila e testa OK (implementação real, sem stub)
- [x] Track 3: `airllm-cli` + `airllm-mcp` + `airllm-python` compilam e testam OK no workspace
- [x] Integração: workspace completo compila
- [x] E2E: CLI funcional
- [x] E2E: MCP server funcional
- [x] E2E: Python bindings funcionais
- [x] Context bank atualizado
- [ ] Git: branch + commit + merge

---

## Resumo Visual

```
┌─────────────────────────────────────────────────────────────┐
│                    DIA 0 (hoje)                              │
│  ✓ Plano aprovado                                            │
│  ✓ Interfaces definidas                                      │
│  ✓ Prompts preparados                                        │
│  → Eu crio workspace Cargo raiz                              │
│  → Você inicia as 3 IAs com os prompts                       │
└─────────────────────────────────────────────────────────────┘
         │              │              │
         ▼              ▼              ▼
   ┌──────────┐  ┌──────────┐  ┌──────────┐
   │ TRACK 1  │  │ TRACK 2  │  │ TRACK 3  │
   │ GLM-5.2  │  │ GPT-5.4 │  │ CODEX    │
   │          │  │  -pro   │  │ -max     │
   │ ollama   │  │ orchestr│  │ cli+mcp  │
   │ client   │  │ +agents │  │ +python  │
   │ +router  │  │ +prompts│  │ +tui     │
   └────┬─────┘  └────┬─────┘  └────┬─────┘
        │             │             │
        └─────────────┼─────────────┘
                      ▼
   ┌──────────────────────────────────┐
   │         INTEGRAÇÃO (eu)          │
   │  • Remover stubs                 │
   │  • Conectar crates                │
   │  • cargo build --workspace       │
   │  • cargo test --workspace        │
   │  • E2E tests                     │
   │  • Git workflow                  │
   └──────────────────────────────────┘
```