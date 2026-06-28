# Benchmark AirLLM v3.0 — Calculadora em Java

> **Data**: 2026-06-28 00:58:37
> **Modelos testados**: 6
> **Tarefa**: Gerar uma calculadora Java completa e funcional

---

## 📊 Ranking Geral

| # | Modelo | Tipo | Tamanho | Tempo | Compila? | Executa? | Qualidade |
|---|--------|------|---------|-------|----------|----------|-----------|
| 1 | `nemotron-3-nano:30b` | local | 30B | 33.3s | ✅ | ✅ | **100/100** |
| 2 | `granite4.1:30b` | local | 30B | 77.1s | ✅ | ✅ | **100/100** |
| 3 | `jaahas/crow:9b` | local | 9B | 74.2s | ❌ | ❌ | **70/100** |
| 4 | `qwen3.5:4b` | local | 4.7B | 99.0s | ❌ | ❌ | **0/100** |
| 5 | `qwen3-coder-next:q8_0` | local | 79.7B | 125.1s | ❌ | ❌ | **0/100** ⚠️ timed out |
| 6 | `qwen3.6:27b` | local | 27.8B | 125.2s | ❌ | ❌ | **0/100** ⚠️ timed out |

---

## ⚡ Ranking por Velocidade

| # | Modelo | Tempo | Qualidade |
|---|--------|-------|-----------|
| 1 | `nemotron-3-nano:30b` | 33.3s | 100/100 |
| 2 | `jaahas/crow:9b` | 74.2s | 70/100 |
| 3 | `granite4.1:30b` | 77.1s | 100/100 |
| 4 | `qwen3.5:4b` | 99.0s | 0/100 |

---

## 🏆 Ranking por Qualidade

| # | Modelo | Score | Tempo | Detalhes |
|---|--------|-------|-------|----------|
| 1 | `granite4.1:30b` | 100/100 | 77.1s | Compila sem erros (+25), Executa sem crash (+20), Suporta adição (+5) |
| 2 | `nemotron-3-nano:30b` | 100/100 | 33.3s | Compila sem erros (+25), Executa sem crash (+20), Suporta adição (+5) |
| 3 | `jaahas/crow:9b` | 70/100 | 74.2s | Não compila (+0), Não executa ou crash (+0), Suporta adição (+5) |
| 4 | `qwen3.5:4b` | 0/100 | 99.0s | Não compila (+0), Não executa ou crash (+0) |
| 5 | `qwen3.6:27b` | 0/100 | 125.2s | Erro na chamada |
| 6 | `qwen3-coder-next:q8_0` | 0/100 | 125.1s | Erro na chamada |

---

## 🎯 Conclusões

### 🏆 Melhor Qualidade Geral
- **Modelo**: `nemotron-3-nano:30b` (local, 30B)
- **Score**: 100/100
- **Tempo**: 33.3s
- **Compila**: ✅
- **Executa**: ✅

### ⚡ Mais Rápido
- **Modelo**: `nemotron-3-nano:30b` (local, 30B)
- **Tempo**: 33.3s
- **Qualidade**: 100/100

### 💰 Melhor Custo-Benefício (Qualidade/Tempo)
- **Modelo**: `nemotron-3-nano:30b` (local, 30B)
- **Score**: 100/100 em 33.3s
- **Ratio**: 3.0 pontos/segundo

---

## 📝 Detalhes por Modelo

### `nemotron-3-nano:30b` (local, 30B)

- **Tempo**: 33.3s
- **Compila**: ✅
- **Executa**: ✅
- **Qualidade**: 100/100
- **Tamanho da resposta**: 3131 chars
- **Critérios de qualidade**:
  - Compila sem erros (+25)
  - Executa sem crash (+20)
  - Suporta adição (+5)
  - Suporta subtração (+5)
  - Suporta multiplicação (+5)
  - Suporta divisão (+5)
  - Trata divisão por zero (+10)
  - Tem comando help (+5)
  - Tem comando quit/exit (+5)
  - Tem try-catch para erros (+10)
  - Validação de input (+5)
  - REPL interativo com loop (+10)
  - Código bem indentado (+5)

### `granite4.1:30b` (local, 30B)

- **Tempo**: 77.1s
- **Compila**: ✅
- **Executa**: ✅
- **Qualidade**: 100/100
- **Tamanho da resposta**: 2739 chars
- **Critérios de qualidade**:
  - Compila sem erros (+25)
  - Executa sem crash (+20)
  - Suporta adição (+5)
  - Suporta subtração (+5)
  - Suporta multiplicação (+5)
  - Suporta divisão (+5)
  - Trata divisão por zero (+10)
  - Tem comando help (+5)
  - Tem comando quit/exit (+5)
  - Tem try-catch para erros (+10)
  - Validação de input (+5)
  - REPL interativo com loop (+10)
  - Código bem indentado (+5)

### `jaahas/crow:9b` (local, 9B)

- **Tempo**: 74.2s
- **Compila**: ❌
- **Executa**: ❌
- **Qualidade**: 70/100
- **Erro de compilação**: `/home/eriktonon/airllm/benchmark_results/Calculator_jaahas_crow_9b.java:44: error: cannot find symbo`
- **Tamanho da resposta**: 2462 chars
- **Critérios de qualidade**:
  - Não compila (+0)
  - Não executa ou crash (+0)
  - Suporta adição (+5)
  - Suporta subtração (+5)
  - Suporta multiplicação (+5)
  - Suporta divisão (+5)
  - Trata divisão por zero (+10)
  - Tem comando help (+5)
  - Tem comando quit/exit (+5)
  - Tem try-catch para erros (+10)
  - Validação de input (+5)
  - REPL interativo com loop (+10)
  - Código bem indentado (+5)

### `qwen3.5:4b` (local, 4.7B)

- **Tempo**: 99.0s
- **Compila**: ❌
- **Executa**: ❌
- **Qualidade**: 0/100
- **Erro de compilação**: `No valid Java code/class found in model output`
- **Tamanho da resposta**: 0 chars
- **Critérios de qualidade**:
  - Não compila (+0)
  - Não executa ou crash (+0)

### `qwen3-coder-next:q8_0` (local, 79.7B)

- **Tempo**: 125.1s
- **Compila**: ❌
- **Executa**: ❌
- **Qualidade**: 0/100
- **Erro**: timed out
- **Tamanho da resposta**: 0 chars
- **Critérios de qualidade**:
  - Erro na chamada

### `qwen3.6:27b` (local, 27.8B)

- **Tempo**: 125.2s
- **Compila**: ❌
- **Executa**: ❌
- **Qualidade**: 0/100
- **Erro**: timed out
- **Tamanho da resposta**: 0 chars
- **Critérios de qualidade**:
  - Erro na chamada
