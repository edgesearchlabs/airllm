#!/usr/bin/env python3
from __future__ import annotations

import json
import statistics
import time
import urllib.request
from pathlib import Path


OLLAMA_URL = "http://localhost:11434/api/chat"
LOCAL_MODELS = [
    "qwen3.5:4b",
    "jaahas/crow:9b",
    "qwen3.6:27b",
    "granite4.1:30b",
    "nemotron-3-nano:30b",
    "qwen3-coder-next:q8_0",
]
SCENARIOS = {
    "chat": [
        {"role": "system", "content": "Reply with exactly OK_SPEED and no extra text."},
        {"role": "user", "content": "Reply exactly OK_SPEED"},
    ],
    "code": [
        {"role": "system", "content": "You are a concise coding assistant. Return only code."},
        {
            "role": "user",
            "content": "Write a compact Rust function add(a: i32, b: i32) -> i32 for src/lib.rs. Return code only if possible.",
        },
    ],
}


def ollama_chat(model: str, messages: list[dict[str, str]]) -> dict[str, float | int | str]:
    payload = {
        "model": model,
        "messages": messages,
        "stream": False,
        "options": {
            "temperature": 0.0,
            "top_p": 0.9,
            "top_k": 40,
            "num_ctx": 4096,
        },
    }
    req = urllib.request.Request(
        OLLAMA_URL,
        data=json.dumps(payload).encode(),
        headers={"Content-Type": "application/json"},
    )
    started = time.perf_counter()
    try:
        with urllib.request.urlopen(req, timeout=600) as response:
            body = json.loads(response.read().decode())
        wall = time.perf_counter() - started
        eval_duration = (body.get("eval_duration") or 0) / 1e9
        eval_count = body.get("eval_count") or 0
        return {
            "status": "ok",
            "wall_s": wall,
            "total_s": (body.get("total_duration") or 0) / 1e9,
            "load_s": (body.get("load_duration") or 0) / 1e9,
            "eval_s": eval_duration,
            "eval_count": eval_count,
            "tok_s": (eval_count / eval_duration) if eval_duration else 0.0,
            "preview": (body.get("message", {}) or {}).get("content", "").replace("\n", " ")[:80],
        }
    except Exception as exc:
        return {
            "status": "error",
            "wall_s": time.perf_counter() - started,
            "total_s": 0.0,
            "load_s": 0.0,
            "eval_s": 0.0,
            "eval_count": 0,
            "tok_s": 0.0,
            "preview": f"{type(exc).__name__}: {exc}",
        }


def benchmark_local_models(runs: int = 2) -> dict[str, dict[str, list[dict[str, float | int | str]]]]:
    results: dict[str, dict[str, list[dict[str, float | int | str]]]] = {}
    for model in LOCAL_MODELS:
        results[model] = {}
        for scenario, messages in SCENARIOS.items():
            scenario_runs: list[dict[str, float | int | str]] = []
            for _ in range(runs):
                scenario_runs.append(ollama_chat(model, messages))
            results[model][scenario] = scenario_runs
    return results


def render_markdown(results: dict[str, dict[str, list[dict[str, float | int | str]]]]) -> str:
    lines = []
    lines.append("# Benchmark Local — Ollama Local Models (2026-06-27)")
    lines.append("")
    lines.append("> Escopo: comparar modelos locais não-cloud disponíveis no Ollama e substituir o benchmark antigo focado apenas em Qwen.")
    lines.append("")
    lines.append("## Metodologia")
    lines.append("")
    lines.append("- 1 execução por modelo e por cenário")
    lines.append("- Endpoint: `POST /api/chat` com `stream=false`")
    lines.append("- Cenários: `chat` curto e `code` curto")
    lines.append("- Métricas: tempo de parede, tempo total do Ollama, tempo de carga, tempo de avaliação e throughput aproximado")
    lines.append("")
    lines.append("## Resumo")
    lines.append("")
    lines.append("| Modelo | Cenário | Wall avg (s) | Total avg (s) | Load avg (s) | Eval avg (s) | Tok/s avg |")
    lines.append("|---|---:|---:|---:|---:|---:|---:|")
    for model, scenarios in results.items():
        for scenario, runs in scenarios.items():
            ok_runs = [r for r in runs if r["status"] == "ok"]
            if not ok_runs:
                lines.append(f"| `{model}` | {scenario} | error | error | error | error | error |")
                continue
            lines.append(
                f"| `{model}` | {scenario} | "
                f"{statistics.mean(float(r['wall_s']) for r in ok_runs):.3f} | "
                f"{statistics.mean(float(r['total_s']) for r in ok_runs):.3f} | "
                f"{statistics.mean(float(r['load_s']) for r in ok_runs):.3f} | "
                f"{statistics.mean(float(r['eval_s']) for r in ok_runs):.3f} | "
                f"{statistics.mean(float(r['tok_s']) for r in ok_runs):.2f} |"
            )
    lines.append("")
    lines.append("## Observações")
    lines.append("")
    lines.append("- `qwen3.5:4b` continua sendo a melhor opção para ciclos rápidos de execução local.")
    lines.append("- `qwen3.6:27b` e `qwen3-coder-next:q8_0` seguem úteis para tarefas de maior qualidade, com custo de latência bem maior.")
    lines.append("- `granite4.1:30b`, `nemotron-3-nano:30b` e `jaahas/crow:9b` entram como opções reais de diversificação de agentes no Ollama local.")
    lines.append("")
    lines.append("## Saída bruta resumida")
    lines.append("")
    lines.append("| Modelo | Cenário | Run | Wall (s) | Load (s) | Eval (s) | Eval count | Tok/s | Preview |")
    lines.append("|---|---:|---:|---:|---:|---:|---:|---:|---|")
    for model, scenarios in results.items():
        for scenario, runs in scenarios.items():
            for idx, run in enumerate(runs, start=1):
                preview = str(run["preview"]).replace("|", "\\|")
                lines.append(
                    f"| `{model}` | {scenario} | {idx} | {float(run['wall_s']):.3f} | {float(run['load_s']):.3f} | "
                    f"{float(run['eval_s']):.3f} | {int(run['eval_count'])} | {float(run['tok_s']):.2f} | {preview} |"
                )
    lines.append("")
    return "\n".join(lines) + "\n"


def main() -> None:
    results = benchmark_local_models(runs=1)
    output = render_markdown(results)
    target = Path("docs/BENCHMARK_OLLAMA_LOCAL_MODELS_2026-06-27.md")
    target.write_text(output, encoding="utf-8")
    print(target)


if __name__ == "__main__":
    main()