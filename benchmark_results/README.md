# Benchmark Results

Indice central dos testes executados contra modelos Ollama no projeto AirLLM.

## Destaques atuais

- Melhor modelo na bateria Java local (media geral): `codegemma:7b`
- Melhor em `Hello World em Java`: `qwen2.5-coder:14b`
- Melhor em `Calculadora CLI em Java`: `codegemma:7b`

## Relatorios disponíveis

- `benchmark_report.md` — benchmark inicial da calculadora Java
- `java_battery_report.md` — relatorio mestre da bateria Java local
- `benchmark_results.json` — dados brutos do benchmark inicial
- `java_battery_results.json` — dados brutos da bateria Java local

## Scripts

- `run_benchmark.py` — benchmark da calculadora Java
- `run_java_battery.py` — bateria Java com rankings por atividade e media por modelo

## Escopo atual da bateria

Atividades executadas:
- Hello World em Java
- Calculadora CLI em Java

Escopo de modelos:
- Modelos locais presentes no Ollama
- Modelos cloud ficaram fora da bateria consolidada por limite de uso anterior e por foco no ambiente local

## Politica de artefatos

- Relatorios `.md`, dados `.json` e scripts `.py` devem ficar versionados
- Artefatos compilados `.class` e exemplos `.java` gerados pelos benchmarks sao considerados temporarios
- O `.gitignore` do repositorio foi atualizado para evitar que esses gerados poluam os commits

## Melhorias futuras sugeridas

- Refactor de codigo Java legado
- Geracao de testes JUnit
- Correcao de bug em projeto Java
- Endpoint REST em Java
- Repetir a bateria para Python, Rust e TypeScript
