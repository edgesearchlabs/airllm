# Relatório de Testes por Cenário — AirLLM v4.0

> **Data**: 2026-06-28
> **Executor**: EdgeSearch Orquestrador
> **Versão**: v4.0.0
> **Ollama**: 0.30.11
> **GPU**: 16GB VRAM

---

## Cenário 1: Dev — Calculadora Rust Funcional

### Prompt
```
Create a complete Rust calculator program in a single main.rs file.
Requirements: 1) reads a line like 5 + 3 from stdin 2) supports + - * / operators
3) handles division by zero with error message 4) loops until user types quit
5) prints result. The code must compile with rustc main.rs.
```

### Resultados por modelo

| Modelo | Compila? | Funciona? | Latência | Observação |
|---|---|---|---|---|
| `qwen2.5-coder:14b` | ✅ Sim | ✅ Sim | ~9s | Código limpo, compila sem erros, todas operações corretas |
| `qwen3.5:4b` | ❌ Não | ❌ Não | ~5s | Loop infinito sem ler input, não compila |

### Testes funcionais (qwen2.5-coder:14b)

```
$ echo "10 + 5" | ./calc
Result: 15 ✅

$ echo "10 / 0" | ./calc
Error: Division by zero ✅

$ echo "7 * 3" | ./calc
Result: 21 ✅

$ echo "20 - 8" | ./calc
Result: 12 ✅
```

### Veredito Cenário Dev

**qwen2.5-coder:14b** é o melhor modelo para código: gera código que compila e funciona corretamente. `qwen3.5:4b` é rápido mas produz código incompleto para tarefas complexas.

---

## Cenário 2: Conhecimento Geral — Capital do Brasil

### Prompt
```
What is the capital of Brazil and who founded it?
```

### Resultados por modelo

| Modelo | Resposta Correta? | Latência | Qualidade | Observação |
|---|---|---|---|---|
| `qwen3.5:4b` | ✅ Sim | 17.7s | ⭐⭐⭐⭐⭐ | Resposta detalhada: JK, Lúcio Costa, Oscar Niemeyer |
| `gemma4:12b` | ✅ Sim | 16.4s | ⭐⭐⭐⭐ | Resposta correta mas menos detalhada |

### Melhor resposta (qwen3.5:4b)
> The current capital of Brazil is **Brasília**. It became the capital in October 1960...
> President Juscelino Kubitschek (JK): He initiated the relocation plan in 1956...
> Lúcio Costa: A Brazilian architect and urban planner who developed the master city plan...
> Oscar Niemeyer: An architectural visionary whose designs defined much of the city's iconic modernist structures...

### Veredito
`qwen3.5:4b` produziu resposta mais completa e bem estruturada, citando os 3 responsáveis principais.

---

## Cenário 3: Conhecimento Geral — Quantum Entanglement

### Prompt
```
Explain what is quantum entanglement in simple terms for a beginner
```

### Resultados por modelo

| Modelo | Resposta Correta? | Latência | Qualidade | Observação |
|---|---|---|---|---|
| `codegemma:7b` | ✅ Sim | 7.6s | ⭐⭐⭐⭐ | Analogia das moedas, aplicações listadas |
| `deepseek-coder-v2:16b` | ✅ Sim | 8.9s | ⭐⭐⭐⭐⭐ | Explicação mais técnica e precisa |

### Melhor resposta (deepseek-coder-v2:16b)
> Quantum entanglement is a fascinating phenomenon... if you interact with one particle, it instantly affects the other particle even if they are separated by vast distances... until you measure or observe one of the entangled particles, its state is uncertain... This concept has important implications for things like cryptography, computing, and our understanding of reality itself.

### Veredito
`deepseek-coder-v2:16b` produziu explicação mais precisa e completa, com contexto histórico e implicações práticas.

---

## Cenário 4: Conhecimento Geral — TCP vs UDP

### Prompt
```
What are 3 important differences between TCP and UDP? Be concise.
```

### Resultados por modelo

| Modelo | Resposta Correta? | Latência | Qualidade | Observação |
|---|---|---|---|---|
| `qwen2.5-coder:14b` | ✅ Sim | 8.7s | ⭐⭐⭐⭐⭐ | 3 diferenças claras: connection, reliability, use cases |
| `granite4.1:30b` | ✅ Sim | 35.8s | ⭐⭐⭐⭐⭐ | 3 diferenças com detalhes técnicos (header size, SYN/ACK) |

### Melhor resposta (granite4.1:30b)
> 1. **Reliability**: TCP is connection-oriented with error checking and retransmission. UDP is connectionless, no guarantee of delivery.
> 2. **Overhead & Speed**: TCP incurs higher overhead due to handshakes. UDP has minimal header overhead (8 bytes).
> 3. **Use Cases**: TCP for reliability (web, email). UDP for real-time, low-latency (streaming, gaming, DNS).

### Veredito
`granite4.1:30b` produziu resposta mais técnica com detalhes específicos (8 bytes header, SYN/SYN-ACK), mas levou 4x mais tempo que `qwen2.5-coder:14b`.

---

## Ranking Final por Cenário

### Cenário Dev (código que compila e funciona)

| Rank | Modelo | Latência | Compila | Funciona |
|---|---|---|---|---|
| 🥇 1 | `qwen2.5-coder:14b` | 9s | ✅ | ✅ |
| 🥈 2 | `qwen3.5:4b` | 5s | ❌ | ❌ |

### Cenário Conhecimento Geral (qualidade da resposta)

| Rank | Modelo | Latência | Qualidade | Cenário |
|---|---|---|---|---|
| 🥇 1 | `qwen3.5:4b` | 17.7s | ⭐⭐⭐⭐⭐ | Capital Brasil |
| 🥈 2 | `gemma4:12b` | 16.4s | ⭐⭐⭐⭐ | Capital Brasil |
| 🥇 1 | `deepseek-coder-v2:16b` | 8.9s | ⭐⭐⭐⭐⭐ | Quantum |
| 🥈 2 | `codegemma:7b` | 7.6s | ⭐⭐⭐⭐ | Quantum |
| 🥇 1 | `granite4.1:30b` | 35.8s | ⭐⭐⭐⭐⭐ | TCP vs UDP |
| 🥈 2 | `qwen2.5-coder:14b` | 8.7s | ⭐⭐⭐⭐⭐ | TCP vs UDP |

### Ranking Geral (melhor custo-benefício)

| Rank | Modelo | Pontos Fortes | Pontos Fracos | Melhor Uso |
|---|---|---|---|---|
| 🥇 1 | `qwen2.5-coder:14b` | Código que compila, rápido (9s), bom em conhecimento | Menos detalhado em história | **Dev + conhecimento técnico** |
| 🥈 2 | `qwen3.5:4b` | Rápido (5-18s), excelente em conhecimento geral | Código incompleto para tarefas complexas | **Conhecimento geral + tarefas simples** |
| 🥉 3 | `deepseek-coder-v2:16b` | Excelente explicação técnica, rápido (9s) | Não testado em código | **Explicações técnicas** |
| 4 | `granite4.1:30b` | Respostas técnicas detalhadas | Lento (36s) | **Quando qualidade > velocidade** |
| 5 | `gemma4:12b` | Respostas corretas, velocidade média | Menos detalhado | **Conhecimento geral** |
| 6 | `codegemma:7b` | Rápido (8s), boas analogias | Menos preciso | **Iniciantes** |

---

## Conclusões

1. **Para código que funciona**: `qwen2.5-coder:14b` é o melhor — gera código que compila e executa corretamente
2. **Para conhecimento geral**: `qwen3.5:4b` tem o melhor custo-benefício (rápido + detalhado)
3. **Para explicações técnicas**: `deepseek-coder-v2:16b` e `granite4.1:30b` produzem as respostas mais precisas
4. **Para velocidade**: `codegemma:7b` (7.6s) e `qwen3.5:4b` (5s) são os mais rápidos
5. **Para qualidade máxima**: `granite4.1:30b` mas com custo de 36s por resposta

### Recomendação de roteamento

| Tipo de Tarefa | Modelo Recomendado | Razão |
|---|---|---|
| Código que precisa compilar | `qwen2.5-coder:14b` | Gera código funcional |
| Perguntas de conhecimento geral | `qwen3.5:4b` | Rápido + detalhado |
| Explicações técnicas profundas | `deepseek-coder-v2:16b` | Preciso + rápido |
| Tarefas simples e rápidas | `codegemma:7b` | Mais rápido |
| Qualidade máxima (sem pressa) | `granite4.1:30b` | Mais detalhado |