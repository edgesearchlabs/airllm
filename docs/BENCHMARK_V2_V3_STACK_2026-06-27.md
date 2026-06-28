# Comparativo Tecnológico — AirLLM v2 (Python) vs AirLLM v3 (Rust) — 2026-06-27

> Este documento compara a pilha tecnológica do v2 com a do v3 usando o mesmo backend Ollama local.
> Importante: não é uma comparação funcional 1:1 de produto, porque o v2 original não foi desenhado para Ollama nem para multi-agente. O objetivo aqui é medir o custo relativo da pilha Python versus Rust no plano de controle local.

## Cenários

- `v2-style python list_models`: chamada HTTP direta em Python ao `/api/tags`
- `v3 python binding list_models`: chamada via binding Python do v3 com cache local
- `v2-style python chat`: chamada HTTP direta em Python ao `/api/chat`
- `v3 cli chat`: fluxo E2E pelo binário Rust `target/debug/airllm`
- `v2-style python code`: chamada HTTP direta em Python para prompt curto de código
- `v3 cli code`: fluxo E2E pelo binário Rust com orquestrador

Modelo usado: `qwen3.5:4b`

## Resultados

| Cenário | Execuções (s) | Média (s) |
|---|---|---:|
| v2-style python list_models | 0.0030, 0.0009, 0.0008 | 0.0015 |
| v3 python binding list_models | 0.0061, 0.0018, 0.0020 | 0.0033 |
| v2-style python chat | 1.4465, 1.4580, 1.4656 | 1.4567 |
| v3 cli chat | 1.9487, 2.0735, 0.9408 | 1.6543 |
| v2-style python code | 0.9089, 0.8962, 0.8936 | 0.8996 |
| v3 cli code | 0.9869, 0.8987, 0.9652 | 0.9503 |

## Leitura prática

- A descoberta de modelos ainda não mostrou ganho claro no v3 nesta máquina; o cache precisa de mais tuning.
- Em `chat` e `code` curtos, a chamada Python direta tende a ser mais barata como tecnologia pura, porque o v3 adiciona o custo do orchestrator e, no caso da CLI, o custo do processo binário.
- O valor do v3 está em orquestração, caching, prewarm, multiagente e integração de ferramentas, não em vencer uma chamada HTTP Python minimalista em toda situação.

## Conclusão

- Se o objetivo for apenas disparar uma requisição ao Ollama, a base Python mínima ainda é uma linha de base muito eficiente.
- Se o objetivo for uma plataforma de desenvolvimento local integrada, o v3 é superior em capacidade e mantém uma latência prática aceitável, especialmente em caminhos quentes e com prewarm.
- O próximo ganho real no v3 continua sendo reduzir `load_duration` e overhead do caminho E2E simples.

