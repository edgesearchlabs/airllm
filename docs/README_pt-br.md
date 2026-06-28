# AirLLM v3.0 — Documentação (PT-BR)

O repositório agora tem dois papéis principais:

1. **AirLLM v3.0 em Rust**, focado em orquestração de agentes de codificação com Ollama.
2. **AirLLM v2 legado em Python**, preservado para compatibilidade e referência histórica.

English README: [../README.md](../README.md)

## Estado Atual

- `airllm-ollama`: cliente async para Ollama, roteador de modelos, testes, clippy e docs validados
- `airllm-orchestrator`: orquestrador modular real com prompts e configs de agentes
- `airllm-cli`, `airllm-mcp` e `airllm-python`: integrados e validados localmente
- Ollama local testado com `qwen3.5:4b` e `qwen3.6:27b`
- Benchmark documentado em [BENCHMARK_OLLAMA_LOCAL_MODELS_2026-06-27.md](BENCHMARK_OLLAMA_LOCAL_MODELS_2026-06-27.md)
- Comparativo tecnológico v2-v3 documentado em [BENCHMARK_V2_V3_STACK_2026-06-27.md](BENCHMARK_V2_V3_STACK_2026-06-27.md)
- Relatório consolidado de validação em [TEST_REPORT_2026-06-27.md](TEST_REPORT_2026-06-27.md)

## Estrutura do Repositório

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

## Início Rápido

### Pré-requisitos

- Rust
- Python 3.11+
- Ollama rodando em `http://localhost:11434`
- Pelo menos um modelo local instalado, como `qwen3.5:4b`

### Build e testes

```bash
cargo build --workspace
cargo test --workspace
```

### Listar modelos locais pelo AirLLM

```bash
cargo run -p airllm-cli -- models
```

### Rodar um chat

```bash
cargo run -p airllm-cli -- chat --prompt "Responda exatamente OK" --model qwen3.5:4b
```

### Rodar uma geração de código

```bash
cargo run -p airllm-cli -- code \
  "Escreva uma função Rust compacta add(a: i32, b: i32) -> i32 em src/lib.rs. Retorne só código se possível." \
  --language rust \
  --output src/lib.rs \
  --model qwen3.5:4b
```

### Rodar o servidor MCP

```bash
cargo run -p airllm-mcp
```

### Usar os bindings Python

```bash
PYTHONPATH=python python3 - <<'PY'
from airllm import Orchestrator
orch = Orchestrator("http://localhost:11434")
print(orch.list_models())
PY
```

## Resumo de Benchmark

| Modelo | Chat direto | Code direto |
|---|---:|---:|
| `qwen3.5:4b` | 4.778s | 7.121s |
| `jaahas/crow:9b` | 10.773s | 4.081s |
| `qwen3.6:27b` | 46.775s | 101.787s |
| `granite4.1:30b` | 15.767s | 18.304s |
| `nemotron-3-nano:30b` | 32.761s | 12.437s |
| `qwen3-coder-next:q8_0` | 50.908s | 81.470s |

Detalhes completos: [BENCHMARK_OLLAMA_LOCAL_MODELS_2026-06-27.md](BENCHMARK_OLLAMA_LOCAL_MODELS_2026-06-27.md)

## Índice de Documentação

- Roadmap: [ROADMAP_PARALELO_3_FRENTES.md](ROADMAP_PARALELO_3_FRENTES.md)
- Plano revisado: [PLANO_REVISADO_V3.md](PLANO_REVISADO_V3.md)
- Guia de execução: [GUIA_EXECUCAO.md](GUIA_EXECUCAO.md)
- Guia de manutenção: [GUIA_MANUTENCAO.md](GUIA_MANUTENCAO.md)
- Run guide em inglês: [RUN_GUIDE.md](RUN_GUIDE.md)
- Maintenance guide em inglês: [MAINTENANCE_GUIDE.md](MAINTENANCE_GUIDE.md)
- Benchmark de modelos locais: [BENCHMARK_OLLAMA_LOCAL_MODELS_2026-06-27.md](BENCHMARK_OLLAMA_LOCAL_MODELS_2026-06-27.md)
- Benchmark comparativo v2-v3: [BENCHMARK_V2_V3_STACK_2026-06-27.md](BENCHMARK_V2_V3_STACK_2026-06-27.md)
- Relatório consolidado de testes: [TEST_REPORT_2026-06-27.md](TEST_REPORT_2026-06-27.md)

## Índice de Validação

- Testes do cliente Ollama: [../crates/airllm-ollama/tests/test_client.rs](../crates/airllm-ollama/tests/test_client.rs)
- Testes do router: [../crates/airllm-ollama/tests/test_router.rs](../crates/airllm-ollama/tests/test_router.rs)
- Testes de stream: [../crates/airllm-ollama/tests/test_stream.rs](../crates/airllm-ollama/tests/test_stream.rs)
- Testes do orchestrator: [../crates/airllm-orchestrator/tests/test_orchestrator.rs](../crates/airllm-orchestrator/tests/test_orchestrator.rs)
- Testes de agentes: [../crates/airllm-orchestrator/tests/test_agents.rs](../crates/airllm-orchestrator/tests/test_agents.rs)
- Testes de decomposição: [../crates/airllm-orchestrator/tests/test_decompose.rs](../crates/airllm-orchestrator/tests/test_decompose.rs)
- Testes de consolidação: [../crates/airllm-orchestrator/tests/test_consolidate.rs](../crates/airllm-orchestrator/tests/test_consolidate.rs)
- Testes do MCP: [../crates/airllm-mcp/src/server.rs](../crates/airllm-mcp/src/server.rs) e [../crates/airllm-mcp/src/tools.rs](../crates/airllm-mcp/src/tools.rs)
- Smoke test do binding Python: [../crates/airllm-python/src/lib.rs](../crates/airllm-python/src/lib.rs)

## Componentes Legados

As partes abaixo foram preservadas intencionalmente:

- [../air_llm/README.md](../air_llm/README.md): pacote Python legado do AirLLM v2
- [../training/README.md](../training/README.md): notas antigas de treinamento
- [../training/README_en.md](../training/README_en.md): versão em inglês das notas antigas

Nesta limpeza foram removidos apenas arquivos de topo claramente obsoletos e assets sem referência na documentação principal atual.# AirLLM — Documentação (PT-BR)

> **AirLLM** otimiza o uso de memória durante a inferência, permitindo que modelos de linguagem grandes (LLMs) de 70B parâmetros rodem em uma única GPU de 4GB **sem quantização, destilação ou pruning**. É possível rodar o **Llama3.1 405B** em apenas **8GB de VRAM**.

---

## Sumário

- [Início Rápido](#início-rápido)
- [Compressão de Modelo — 3x de Aceleração](#compressão-de-modelo--3x-de-aceleração)
- [Configurações](#configurações)
- [Execução no MacOS](#execução-no-macos)
- [Notebooks de Exemplo](#notebooks-de-exemplo)
- [Modelos Suportados](#modelos-suportados)
- [Arquitetura Interna](#arquitetura-interna)
- [FAQ](#faq)

---

## Início Rápido

### 1. Instalação

```bash
pip install airllm
```

### 2. Inferência

Inicialize o modelo passando o ID do repositório HuggingFace ou o caminho local. A inferência é feita de forma similar a um modelo transformer comum:

```python
from airllm import AutoModel

MAX_LENGTH = 128
# Pode usar o ID do repositório HuggingFace:
model = AutoModel.from_pretrained("garage-bAInd/Platypus2-70B-instruct")

# Ou usar o caminho local do modelo...
# model = AutoModel.from_pretrained("/home/ubuntu/.cache/huggingface/hub/models--garage-bAInd--Platypus2-70B-instruct/snapshots/...")

input_text = ['What is the capital of United States?']

input_tokens = model.tokenizer(input_text,
    return_tensors="pt",
    return_attention_mask=False,
    truncation=True,
    max_length=MAX_LENGTH,
    padding=False)

generation_output = model.generate(
    input_tokens['input_ids'].cuda(),
    max_new_tokens=20,
    use_cache=True,
    return_dict_in_generate=True)

output = model.tokenizer.decode(generation_output.sequences[0])
print(output)
```

> **Nota:** Durante a primeira inferência, o modelo original é decomposto e salvo camada por camada. Certifique-se de que há espaço em disco suficiente no diretório de cache do HuggingFace.

---

## Compressão de Modelo — 3x de Aceleração

A compressão é baseada em quantização em blocos (block-wise quantization), acelerando a inferência em até **3x** com perda de precisão praticamente insignificante. Mais detalhes no [paper original](https://arxiv.org/abs/2212.09720).

### Como ativar a compressão:

1. Instale o `bitsandbytes`: `pip install -U bitsandbytes`
2. Certifique-se de que o airllm está na versão 2.0.0+: `pip install -U airllm`
3. Passe o argumento `compression` ao inicializar o modelo:

```python
model = AutoModel.from_pretrained("garage-bAInd/Platypus2-70B-instruct",
                     compression='4bit'  # ou '8bit' para quantização em 8 bits
                    )
```

### Diferença entre compressão e quantização tradicional

A quantização tradicional precisa quantizar tanto pesos quanto ativações para obter velocidade real, o que dificulta a manutenção da precisão e o tratamento de outliers.

No AirLLM, o gargalo é o carregamento do disco, então **apenas os pesos são quantizados**, reduzindo o tamanho do arquivo carregado sem comprometer a precisão das ativações.

---

## Configurações

Ao inicializar o modelo, os seguintes parâmetros são suportados:

| Parâmetro | Tipo | Padrão | Descrição |
|---|---|---|---|
| `compression` | `str` | `None` | `'4bit'` ou `'8bit'` para quantização em blocos |
| `profiling_mode` | `bool` | `False` | Se `True`, exibe tempos de execução |
| `layer_shards_saving_path` | `str` | `None` | Caminho alternativo para salvar o modelo dividido em camadas |
| `hf_token` | `str` | `None` | Token da API HuggingFace para modelos gated |
| `prefetching` | `bool` | `True` | Pré-carregamento para sobrepor carregamento e computação |
| `delete_original` | `bool` | `False` | Se `True`, deleta o modelo original após divisão, economizando espaço em disco |

---

## Execução no MacOS

No MacOS, o processo é o mesmo do Linux:

- Instale o [MLX](https://github.com/ml-explore/mlx) e o `torch`
- Apenas processadores **Apple Silicon** são suportados
- Pode ser necessário instalar o Python nativo ([referência](https://stackoverflow.com/a/65432861/21230266))

Notebook de exemplo: [run_on_macos.ipynb](https://github.com/lyogavin/airllm/blob/main/air_llm/examples/run_on_macos.ipynb)

---

## Notebooks de Exemplo

- [Notebook geral — todos os tipos de modelos](https://github.com/lyogavin/airllm/blob/main/air_llm/examples/run_all_types_of_models.ipynb)
- [Llama3.1 405B](https://github.com/lyogavin/airllm/blob/main/air_llm/examples/run_llama3.1_405B.ipynb)
- [Execução no MacOS](https://github.com/lyogavin/airllm/blob/main/air_llm/examples/run_on_macos.ipynb)

### Exemplos por modelo

**ChatGLM:**
```python
from airllm import AutoModel
model = AutoModel.from_pretrained("THUDM/chatglm3-6b-base")
```

**QWen:**
```python
from airllm import AutoModel
model = AutoModel.from_pretrained("Qwen/Qwen-7B")
```

**Baichuan / InternLM / Mistral:**
```python
from airllm import AutoModel
model = AutoModel.from_pretrained("baichuan-inc/Baichuan2-7B-Base")
# model = AutoModel.from_pretrained("internlm/internlm-20b")
# model = AutoModel.from_pretrained("mistralai/Mistral-7B-Instruct-v0.1")
```

---

## Modelos Suportados

| Modelo | Classe AirLLM | Arquitetura Detectada |
|---|---|---|
| Llama2 / Llama3 | `AirLLMLlama2` | `LlamaForCausalLM` |
| QWen | `AirLLMQWen` | `QWenModel` |
| QWen2 / QWen2.5 | `AirLLMQWen2` | `Qwen2ForCausalLM` |
| Baichuan | `AirLLMBaichuan` | `BaichuanForCausalLM` |
| ChatGLM | `AirLLMChatGLM` | `ChatGLMModel` |
| InternLM | `AirLLMInternLM` | `InternLMForCausalLM` |
| Mistral | `AirLLMMistral` | `MistralForCausalLM` |
| Mixtral (MoE) | `AirLLMMixtral` | `MixtralForCausalLM` |

O `AutoModel` detecta automaticamente a arquitetura do modelo via `AutoConfig` e seleciona a classe correta.

---

## Arquitetura Interna

### Como funciona o AirLLM

O AirLLM usa uma estratégia de **inferência camada por camada (layer-wise inference)** para reduzir drasticamente o uso de memória GPU:

```
┌─────────────────────────────────────────────────────────┐
│                    DISCO (Modelo Dividido)                │
│  ┌──────┐ ┌──────┐ ┌──────┐ ┌──────┐ ┌──────┐ ┌──────┐ │
│  │embed │ │layer0│ │layer1│ │ ...  │ │ norm │ │lm_head│ │
│  │.sft  │ │.sft  │ │.sft  │ │      │ │.sft  │ │.sft   │ │
│  └──┬───┘ └──┬───┘ └──┬───┘ └──┬───┘ └──┬───┘ └──┬────┘ │
└─────┼────────┼────────┼────────┼────────┼────────┼──────┘
      │ load   │ load   │ load   │        │ load   │ load
      ▼        ▼        ▼        ▼        ▼        ▼
┌─────────────────────────────────────────────────────────┐
│                    RAM (State Dict por Camada)            │
│  Carrega uma camada por vez do disco → RAM               │
└─────────────────────────────────────────────────────────┘
      │ move to device
      ▼
┌─────────────────────────────────────────────────────────┐
│                    GPU (Apenas 1 camada por vez)          │
│  1. Carrega pesos da camada para GPU                     │
│  2. Executa forward pass                                 │
│  3. Libera memória GPU (layer.to('meta') + clean_memory) │
│  4. Próxima camada...                                    │
└─────────────────────────────────────────────────────────┘
```

### Componentes principais

#### `AirLLMBaseModel` (`airllm_base.py`)

Classe base que herda de `GenerationMixin` (HuggingFace). Responsável por:

- **Divisão do modelo em camadas**: O modelo é decomposto em shards (uma por camada) e salvo em disco no formato safetensors
- **Forward pass camada por camada**: Cada camada é carregada do disco → RAM → GPU, executada, e então liberada da GPU
- **Prefetching**: Usa `ThreadPoolExecutor` para pré-carregar a próxima camada enquanto a atual está sendo computada (sobreposição I/O ↔ compute)
- **KV Cache**: Suporta cache de chave-valor para geração autoregressiva
- **Profiling**: Modo opcional para medir tempos de carregamento e computação

#### `AutoModel` (`auto_model.py`)

Fábrica que detecta automaticamente a arquitetura do modelo via `AutoConfig` e instancia a classe correta:

```python
# Detecção por arquitetura:
"Qwen2ForCausalLM" → AirLLMQWen2
"QWenModel"         → AirLLMQWen
"BaichuanForCausalLM" → AirLLMBaichuan
"ChatGLMModel"      → AirLLMChatGLM
"InternLMForCausalLM" → AirLLMInternLM
"MistralForCausalLM" → AirLLMMistral
"MixtralForCausalLM" → AirLLMMixtral
"LlamaForCausalLM"   → AirLLMLlama2 (fallback padrão)
```

No MacOS, usa `AirLLMLlamaMlx` diretamente.

#### `utils.py` — Divisão e Compressão

- `split_and_save_layers()`: Decompõe o modelo HuggingFace em shards por camada, salvando como safetensors
- `compress_layer_state_dict()`: Aplica quantização 4bit (NF4) ou 8bit (block-wise) usando bitsandbytes
- `uncompress_layer_state_dict()`: Descomprime os pesos ao carregar para GPU
- `find_or_create_local_splitted_path()`: Localiza ou cria o diretório de shards
- `check_space()`: Verifica espaço em disco antes de dividir o modelo

#### Sistema de Persistência (`persist/`)

Padrão Strategy para carregar/salvar shards:

| Persister | Plataforma | Formato |
|---|---|---|
| `SafetensorModelPersister` | Linux/GPU | `.safetensors` |
| `MlxModelPersister` | MacOS (Apple Silicon) | `.mlx.npz` |

O `ModelPersister.get_model_persister()` é uma factory singleton que seleciona o persister correto baseado na plataforma.

#### `LayeredProfiler` (`profiler.py`)

Coleta e exibe métricas de tempo por etapa:
- `load_safe_tensor`: tempo de carregamento do disco
- `compression_time`: tempo de descompressão
- `create_layer_from_state_dict`: tempo de movimentação para GPU
- Tempo total de inferência (process e wall time)

### Fluxo de inferência detalhado

1. **Inicialização**: O modelo é carregado com `init_empty_weights()` (meta tensors, sem uso de memória). Tenta-se usar `BetterTransformer` ou `attn_implementation='sdpa'` para flash attention.

2. **Primeira execução**: O modelo original é dividido em camadas e salvo em disco (uma vez apenas). Nas execuções subsequentes, os shards são reutilizados.

3. **Forward pass**:
   - Para cada camada (embed → layers[0..N] → norm → lm_head):
     - **Prefetch**: A próxima camada é submetida ao `ThreadPoolExecutor` para carregamento assíncrono
     - **Load**: A camada atual é carregada do disco para RAM (CPU)
     - **Move**: Os pesos são movidos da RAM para a GPU
     - **Compute**: A camada executa o forward pass
     - **Cleanup**: A camada é movida para `meta` (libera GPU) e `clean_memory()` é chamado
   - O resultado final (logits) é concatenado e retornado

4. **Geração autoregressiva**: O `generate()` do HuggingFace `GenerationMixin` chama `forward()` repetidamente, passando o KV cache acumulado.

### Por que funciona sem muita memória GPU

- **Apenas uma camada na GPU por vez**: Em vez de carregar o modelo inteiro (70B = ~140GB em fp16), apenas uma camada (~2GB) está na GPU em qualquer momento
- **RAM como buffer**: As ativações intermediárias são mantidas em RAM, não em VRAM
- **Disco como armazenamento**: Os pesos residem em disco e são carregados sob demanda
- **Trade-off**: A velocidade é significativamente menor que a inferência normal (limitada por I/O de disco), mas permite rodar modelos enormes em hardware modesto

---

## FAQ

### 1. MetadataIncompleteBuffer

```
safetensors_rust.SafetensorError: Error while deserializing header: MetadataIncompleteBuffer
```

**Causa provável:** Espaço em disco insuficiente. O processo de divisão do modelo consome muito disco. Limpe o cache do HuggingFace e tente novamente.

### 2. ValueError: max() arg is an empty sequence

Provavelmente você está carregando um modelo QWen ou ChatGLM com a classe `AirLLMLlama2`. Use `AutoModel`:

```python
from airllm import AutoModel  # em vez de AirLLMLlama2
model = AutoModel.from_pretrained(...)
```

### 3. 401 Client Error — Repo model is gated

Modelos gated exigem token do HuggingFace:

```python
model = AutoModel.from_pretrained("meta-llama/Llama-2-7b-hf", hf_token='SEU_TOKEN_HF')
```

### 4. ValueError: Asking to pad but the tokenizer does not have a padding token

Desative o padding:

```python
input_tokens = model.tokenizer(input_text,
    return_tensors="pt",
    return_attention_mask=False,
    truncation=True,
    max_length=MAX_LENGTH,
    padding=False  # desativar padding
)
```

---

## Citação

```bibtex
@software{airllm2023,
  author = {Gavin Li},
  title = {AirLLM: scaling large language models on low-end commodity computers},
  url = {https://github.com/lyogavin/airllm/},
  version = {0.0},
  year = {2023},
}
```

## Licença

Apache 2.0