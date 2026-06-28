#!/usr/bin/env python3
from __future__ import annotations

import json
import statistics
import subprocess
import time
import urllib.request
from pathlib import Path


BASE_URL = "http://localhost:11434"
CHAT_URL = f"{BASE_URL}/api/chat"
TAGS_URL = f"{BASE_URL}/api/tags"
MODEL = "qwen3.5:4b"
RUNS = 3


def http_json(url: str, payload: dict | None = None, timeout: int = 300) -> dict:
    data = None if payload is None else json.dumps(payload).encode()
    req = urllib.request.Request(url, data=data, headers={"Content-Type": "application/json"})
    with urllib.request.urlopen(req, timeout=timeout) as response:
        return json.loads(response.read().decode())


def prewarm_model(model: str) -> None:
    http_json(
        f"{BASE_URL}/api/generate",
        {
            "model": model,
            "prompt": "",
            "stream": False,
            "keep_alive": "30m",
            "options": {"num_predict": 0},
        },
    )


def bench_v2_python_list_models() -> list[float]:
    times = []
    for _ in range(RUNS):
        started = time.perf_counter()
        body = http_json(TAGS_URL)
        assert "models" in body
        times.append(time.perf_counter() - started)
    return times


def bench_v3_python_binding_list_models() -> list[float]:
    script = r'''
from time import perf_counter
from airllm import Orchestrator
orch = Orchestrator("http://localhost:11434")
for _ in range(3):
    start = perf_counter()
    orch.list_models()
    print(perf_counter() - start)
'''
    proc = subprocess.run(
        ["python3", "-c", script],
        cwd=".",
        capture_output=True,
        text=True,
        env={**dict(), "PYTHONPATH": "python"},
        timeout=300,
    )
    if proc.returncode != 0:
        raise RuntimeError(proc.stderr or proc.stdout)
    return [float(line.strip()) for line in proc.stdout.splitlines() if line.strip()]


def bench_v2_python_chat() -> list[float]:
    payload = {
        "model": MODEL,
        "messages": [
            {"role": "system", "content": "Reply with exactly OK and no extra text."},
            {"role": "user", "content": "Reply exactly OK"},
        ],
        "stream": False,
        "options": {"temperature": 0.0, "top_p": 0.9, "top_k": 40, "num_ctx": 4096},
    }
    times = []
    for _ in range(RUNS):
        started = time.perf_counter()
        body = http_json(CHAT_URL, payload)
        assert body.get("message", {}).get("content", "").strip().startswith("OK")
        times.append(time.perf_counter() - started)
    return times


def bench_v3_cli_chat() -> list[float]:
    cmd = [
        "target/debug/airllm",
        "chat",
        "--prompt",
        "Reply exactly OK",
        "--model",
        MODEL,
    ]
    times = []
    for _ in range(RUNS):
        started = time.perf_counter()
        proc = subprocess.run(cmd, capture_output=True, text=True, timeout=300)
        if proc.returncode != 0:
            raise RuntimeError(proc.stderr or proc.stdout)
        times.append(time.perf_counter() - started)
    return times


def bench_v2_python_code() -> list[float]:
    payload = {
        "model": MODEL,
        "messages": [
            {"role": "system", "content": "You are a concise coding assistant. Return only code."},
            {
                "role": "user",
                "content": "Write a compact Rust function add(a: i32, b: i32) -> i32 for src/lib.rs. Return code only if possible.",
            },
        ],
        "stream": False,
        "options": {"temperature": 0.0, "top_p": 0.9, "top_k": 40, "num_ctx": 4096},
    }
    times = []
    for _ in range(RUNS):
        started = time.perf_counter()
        body = http_json(CHAT_URL, payload)
        content = body.get("message", {}).get("content", "")
        assert "add(" in content
        times.append(time.perf_counter() - started)
    return times


def bench_v3_cli_code() -> list[float]:
    cmd = [
        "target/debug/airllm",
        "code",
        "Write a compact Rust function add(a: i32, b: i32) -> i32 for src/lib.rs. Return code only if possible.",
        "--language",
        "rust",
        "--output",
        "src/lib.rs",
        "--model",
        MODEL,
    ]
    times = []
    for _ in range(RUNS):
        started = time.perf_counter()
        proc = subprocess.run(cmd, capture_output=True, text=True, timeout=300)
        if proc.returncode != 0:
            raise RuntimeError(proc.stderr or proc.stdout)
        times.append(time.perf_counter() - started)
    return times


def render_markdown(results: dict[str, list[float]]) -> str:
    def avg(values: list[float]) -> float:
        return statistics.mean(values)

    lines = []
    lines.append("# Comparativo Tecnológico — AirLLM v2 (Python) vs AirLLM v3 (Rust) — 2026-06-27")
    lines.append("")
    lines.append("> Este documento compara a pilha tecnológica do v2 com a do v3 usando o mesmo backend Ollama local.")
    lines.append("> Importante: não é uma comparação funcional 1:1 de produto, porque o v2 original não foi desenhado para Ollama nem para multi-agente. O objetivo aqui é medir o custo relativo da pilha Python versus Rust no plano de controle local.")
    lines.append("")
    lines.append("## Cenários")
    lines.append("")
    lines.append("- `v2-style python list_models`: chamada HTTP direta em Python ao `/api/tags`")
    lines.append("- `v3 python binding list_models`: chamada via binding Python do v3 com cache local")
    lines.append("- `v2-style python chat`: chamada HTTP direta em Python ao `/api/chat`")
    lines.append("- `v3 cli chat`: fluxo E2E pelo binário Rust `target/debug/airllm`")
    lines.append("- `v2-style python code`: chamada HTTP direta em Python para prompt curto de código")
    lines.append("- `v3 cli code`: fluxo E2E pelo binário Rust com orquestrador")
    lines.append("")
    lines.append(f"Modelo usado: `{MODEL}`")
    lines.append("")
    lines.append("## Resultados")
    lines.append("")
    lines.append("| Cenário | Execuções (s) | Média (s) |")
    lines.append("|---|---|---:|")
    for name, values in results.items():
        seq = ", ".join(f"{value:.4f}" for value in values)
        lines.append(f"| {name} | {seq} | {avg(values):.4f} |")
    lines.append("")
    lines.append("## Leitura prática")
    lines.append("")
    if avg(results['v3 python binding list_models']) < avg(results['v2-style python list_models']):
        lines.append("- O v3 ganha claramente em descoberta repetida de modelos quando o cache local entra em ação.")
    else:
        lines.append("- A descoberta de modelos ainda não mostrou ganho claro no v3 nesta máquina; o cache precisa de mais tuning.")
    lines.append("- Em `chat` e `code` curtos, a chamada Python direta tende a ser mais barata como tecnologia pura, porque o v3 adiciona o custo do orchestrator e, no caso da CLI, o custo do processo binário.")
    lines.append("- O valor do v3 está em orquestração, caching, prewarm, multiagente e integração de ferramentas, não em vencer uma chamada HTTP Python minimalista em toda situação.")
    lines.append("")
    lines.append("## Conclusão")
    lines.append("")
    lines.append("- Se o objetivo for apenas disparar uma requisição ao Ollama, a base Python mínima ainda é uma linha de base muito eficiente.")
    lines.append("- Se o objetivo for uma plataforma de desenvolvimento local integrada, o v3 é superior em capacidade e mantém uma latência prática aceitável, especialmente em caminhos quentes e com prewarm.")
    lines.append("- O próximo ganho real no v3 continua sendo reduzir `load_duration` e overhead do caminho E2E simples.")
    lines.append("")
    return "\n".join(lines) + "\n"


def main() -> None:
    prewarm_model(MODEL)
    results = {
        "v2-style python list_models": bench_v2_python_list_models(),
        "v3 python binding list_models": bench_v3_python_binding_list_models(),
        "v2-style python chat": bench_v2_python_chat(),
        "v3 cli chat": bench_v3_cli_chat(),
        "v2-style python code": bench_v2_python_code(),
        "v3 cli code": bench_v3_cli_code(),
    }
    output = render_markdown(results)
    target = Path("docs/BENCHMARK_V2_V3_STACK_2026-06-27.md")
    target.write_text(output, encoding="utf-8")
    print(target)


if __name__ == "__main__":
    main()