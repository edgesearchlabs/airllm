# Bateria Java — Modelos Locais do Ollama

> **Data**: 2026-06-28 02:29:57
> **Modelos locais testados**: 13
> **Atividades**: Hello World em Java, Calculadora CLI em Java

## Auditoria da Documentação

- O teste anterior da calculadora já estava documentado em `benchmark_results/benchmark_report.md`.
- Faltava um relatório consolidado da bateria e um ranking por atividade/media.
- Este arquivo fecha essa lacuna e passa a ser o relatório mestre dos testes Java locais.

## Ranking — Hello World em Java

| # | Modelo | Tempo | Compila? | Executa? | Score |
|---|---|---:|:---:|:---:|---:|
| 1 | `qwen2.5-coder:14b` | 7.2s | ✅ | ✅ | **100** |
| 2 | `deepseek-coder-v2:16b` | 10.6s | ✅ | ✅ | **100** |
| 3 | `gemma4:12b` | 10.9s | ✅ | ✅ | **100** |
| 4 | `codestral:22b` | 13.3s | ✅ | ✅ | **100** |
| 5 | `qwen3.5:4b` | 13.7s | ✅ | ✅ | **100** |
| 6 | `jaahas/crow:9b` | 15.1s | ✅ | ✅ | **100** |
| 7 | `codegemma:7b` | 20.4s | ✅ | ✅ | **100** |
| 8 | `granite4.1:30b` | 30.3s | ✅ | ✅ | **100** |
| 9 | `nemotron-3-nano:30b` | 34.7s | ✅ | ✅ | **100** |
| 10 | `qwen3.6:27b` | 35.3s | ✅ | ✅ | **100** |
| 11 | `ornith:35b` | 52.4s | ✅ | ✅ | **100** |
| 12 | `gemma4:31b` | 72.5s | ❌ | ❌ | **0** |
| 13 | `qwen3-coder-next:q8_0` | 72.8s | ❌ | ❌ | **0** |

## Ranking — Calculadora CLI em Java

| # | Modelo | Tempo | Compila? | Executa? | Score |
|---|---|---:|:---:|:---:|---:|
| 1 | `codegemma:7b` | 8.7s | ✅ | ✅ | **100** |
| 2 | `deepseek-coder-v2:16b` | 11.4s | ✅ | ✅ | **100** |
| 3 | `qwen2.5-coder:14b` | 14.6s | ✅ | ✅ | **100** |
| 4 | `jaahas/crow:9b` | 21.1s | ✅ | ✅ | **100** |
| 5 | `nemotron-3-nano:30b` | 40.6s | ✅ | ✅ | **100** |
| 6 | `granite4.1:30b` | 65.2s | ✅ | ✅ | **100** |
| 7 | `ornith:35b` | 92.4s | ✅ | ✅ | **100** |
| 8 | `codestral:22b` | 21.3s | ✅ | ❌ | **85** |
| 9 | `gemma4:12b` | 87.7s | ❌ | ❌ | **60** |
| 10 | `qwen3.5:4b` | 54.2s | ❌ | ❌ | **0** |
| 11 | `gemma4:31b` | 125.3s | ❌ | ❌ | **0** |
| 12 | `qwen3-coder-next:q8_0` | 125.4s | ❌ | ❌ | **0** |
| 13 | `qwen3.6:27b` | 125.4s | ❌ | ❌ | **0** |

## Média de Precisão por Modelo

| # | Modelo | Média | Hello World | Calculadora |
|---|---|---:|---:|---:|
| 1 | `codegemma:7b` | **100.0** | 100 | 100 |
| 2 | `deepseek-coder-v2:16b` | **100.0** | 100 | 100 |
| 3 | `granite4.1:30b` | **100.0** | 100 | 100 |
| 4 | `jaahas/crow:9b` | **100.0** | 100 | 100 |
| 5 | `nemotron-3-nano:30b` | **100.0** | 100 | 100 |
| 6 | `ornith:35b` | **100.0** | 100 | 100 |
| 7 | `qwen2.5-coder:14b` | **100.0** | 100 | 100 |
| 8 | `codestral:22b` | **92.5** | 100 | 85 |
| 9 | `gemma4:12b` | **80.0** | 100 | 60 |
| 10 | `qwen3.5:4b` | **50.0** | 100 | 0 |
| 11 | `qwen3.6:27b` | **50.0** | 100 | 0 |
| 12 | `gemma4:31b` | **0.0** | 0 | 0 |
| 13 | `qwen3-coder-next:q8_0` | **0.0** | 0 | 0 |

## Ranking por Tipo de Atividade

- **Melhor em Hello World**: `qwen2.5-coder:14b`
- **Melhor em Calculadora**: `codegemma:7b`
- **Melhor média geral**: `codegemma:7b`

## Bateria Completa de Testes

Atividades atualmente executadas nesta bateria:

1. **Hello World em Java** — mede aderência mínima, compilação e execução básicas.
2. **Calculadora CLI em Java** — mede implementação com múltiplas operações, REPL, tratamento de erro e validade prática.

Próximas atividades recomendadas para ampliar a bateria:

3. Refactor de código Java legado
4. Geração de testes unitários em Java/JUnit
5. Correção de bug em projeto Java
6. Implementação de endpoint REST em Java
