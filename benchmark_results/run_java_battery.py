#!/usr/bin/env python3
"""Java battery benchmark for local Ollama models.

Activities:
- hello_world_java
- calculator_java

Outputs:
- java_battery_results.json
- java_battery_report.md
"""

from __future__ import annotations

import json
import os
import re
import subprocess
import time
from dataclasses import dataclass
from datetime import datetime
from pathlib import Path
from typing import Any
from urllib import error as urlerror
from urllib import request as urlrequest

OLLAMA_URL = "http://localhost:11434"
OUTPUT_DIR = Path("/home/eriktonon/airllm/benchmark_results")
LOCAL_MODELS = [
    "gemma4:31b",
    "ornith:35b",
    "gemma4:12b",
    "codegemma:7b",
    "codestral:22b",
    "deepseek-coder-v2:16b",
    "qwen2.5-coder:14b",
    "granite4.1:30b",
    "nemotron-3-nano:30b",
    "qwen3.6:27b",
    "qwen3.5:4b",
    "jaahas/crow:9b",
    "qwen3-coder-next:q8_0",
]


@dataclass(frozen=True)
class Activity:
    key: str
    title: str
    timeout_seconds: int
    prompt: str


ACTIVITIES = [
    Activity(
        key="hello_world_java",
        title="Hello World em Java",
        timeout_seconds=70,
        prompt=(
            "Write a complete, minimal, compilable Java program in a file named HelloWorld.java. "
            "Requirements: use a public class HelloWorld with a main method that prints exactly 'Hello, World!'. "
            "Return only the Java code in a single code block."
        ),
    ),
    Activity(
        key="calculator_java",
        title="Calculadora CLI em Java",
        timeout_seconds=120,
        prompt=(
            "Write a complete, working Java calculator application. Requirements:\n\n"
            "1. Create a file called Calculator.java\n"
            "2. Implement a command-line calculator that:\n"
            "   - Supports addition, subtraction, multiplication, division\n"
            "   - Handles division by zero gracefully\n"
            "   - Has a clean interactive REPL interface (reads from stdin)\n"
            "   - Supports 'quit' command to exit\n"
            "   - Shows a help menu with 'help' command\n"
            "3. Include proper error handling\n"
            "4. Include input validation\n"
            "5. Make it compile and run with: javac Calculator.java && java Calculator\n\n"
            "Return ONLY the Java code in a single code block. No explanations outside the code block."
        ),
    ),
]


def call_ollama(model: str, prompt: str, timeout_seconds: int) -> tuple[str, float, str | None]:
    payload = {
        "model": model,
        "messages": [
            {
                "role": "system",
                "content": "You are an expert Java developer. Return only clean compilable Java code.",
            },
            {"role": "user", "content": prompt},
        ],
        "stream": False,
        "options": {"temperature": 0.2, "top_p": 0.9, "num_ctx": 8192},
    }

    req = urlrequest.Request(
        f"{OLLAMA_URL}/api/chat",
        data=json.dumps(payload).encode("utf-8"),
        headers={"Content-Type": "application/json"},
    )

    started = time.time()
    try:
        with urlrequest.urlopen(req, timeout=timeout_seconds) as resp:
            data = json.loads(resp.read().decode("utf-8"))
            return data.get("message", {}).get("content", ""), time.time() - started, None
    except urlerror.HTTPError as exc:
        body = exc.read().decode("utf-8", errors="replace")
        return "", time.time() - started, f"HTTP {exc.code}: {body[:240]}"
    except Exception as exc:  # noqa: BLE001
        return "", time.time() - started, str(exc)


def extract_java_code(text: str) -> str:
    match = re.search(r"```java\s*\n(.*?)```", text, re.DOTALL | re.IGNORECASE)
    if match:
        return match.group(1).strip()

    match = re.search(r"```\s*\n(.*?)```", text, re.DOTALL)
    if match:
        return match.group(1).strip()

    lines = text.splitlines()
    for idx, line in enumerate(lines):
        if line.strip().startswith("public class") or line.strip().startswith("import "):
            return "\n".join(lines[idx:]).strip()
    return text.strip()


def sanitize_name(name: str) -> str:
    return re.sub(r"[^A-Za-z0-9_]", "_", name)


def compile_and_run(activity: Activity, model: str, code: str) -> tuple[bool, bool, str, str]:
    safe_model = sanitize_name(model)
    if activity.key == "hello_world_java":
        base_name = f"HelloWorld_{safe_model}"
        filename = OUTPUT_DIR / f"{base_name}.java"
        if not code or "class" not in code:
            return False, False, "", "No valid Java code/class found in model output"
        code = re.sub(r"public\s+class\s+\w+", f"public class {base_name}", code, count=1)
        code = re.sub(r"class\s+\w+\s*\{", f"class {base_name} {{", code, count=1)
        filename.write_text(code, encoding="utf-8")
        compile_result = subprocess.run(
            ["javac", str(filename)], capture_output=True, text=True, timeout=30, cwd=OUTPUT_DIR
        )
        if compile_result.returncode != 0:
            return False, False, "", compile_result.stderr[:500]
        run_result = subprocess.run(
            ["java", "-cp", str(OUTPUT_DIR), base_name],
            capture_output=True,
            text=True,
            timeout=10,
            cwd=OUTPUT_DIR,
        )
        return True, run_result.returncode == 0, run_result.stdout[:300], run_result.stderr[:300]

    base_name = f"Calculator_{safe_model}"
    filename = OUTPUT_DIR / f"{base_name}.java"
    if not code or "class" not in code:
        return False, False, "", "No valid Java code/class found in model output"
    code = re.sub(r"public\s+class\s+\w+", f"public class {base_name}", code, count=1)
    code = re.sub(r"class\s+\w+\s*\{", f"class {base_name} {{", code, count=1)
    filename.write_text(code, encoding="utf-8")
    compile_result = subprocess.run(
        ["javac", str(filename)], capture_output=True, text=True, timeout=30, cwd=OUTPUT_DIR
    )
    if compile_result.returncode != 0:
        return False, False, "", compile_result.stderr[:500]
    run_result = subprocess.run(
        ["java", "-cp", str(OUTPUT_DIR), base_name],
        input="5 + 3\nquit\n",
        capture_output=True,
        text=True,
        timeout=10,
        cwd=OUTPUT_DIR,
    )
    return True, run_result.returncode == 0, run_result.stdout[:300], run_result.stderr[:300]


def score_hello_world(code: str, compiles: bool, runs: bool, stdout: str) -> tuple[int, list[str]]:
    score = 0
    reasons: list[str] = []
    if compiles:
        score += 40
        reasons.append("Compila (+40)")
    if runs:
        score += 30
        reasons.append("Executa (+30)")
    if "Hello, World!" in stdout or "Hello, World!" in code:
        score += 20
        reasons.append("Imprime Hello, World! (+20)")
    if "public static void main" in code:
        score += 10
        reasons.append("Tem main method (+10)")
    return score, reasons


def score_calculator(code: str, compiles: bool, runs: bool, stdout: str) -> tuple[int, list[str]]:
    score = 0
    reasons: list[str] = []
    if compiles:
        score += 25
        reasons.append("Compila (+25)")
    if runs:
        score += 20
        reasons.append("Executa (+20)")
    for token, label in [("+", "adição"), ("-", "subtração"), ("*", "multiplicação"), ("/", "divisão")]:
        if token in code:
            score += 5
            reasons.append(f"Suporta {label} (+5)")
    if "zero" in code.lower() or "ArithmeticException" in code:
        score += 10
        reasons.append("Trata divisão por zero (+10)")
    if "help" in code.lower():
        score += 5
        reasons.append("Tem help (+5)")
    if "quit" in code.lower() or "exit" in code.lower():
        score += 5
        reasons.append("Tem quit/exit (+5)")
    if "try" in code and "catch" in code:
        score += 10
        reasons.append("Tem try/catch (+10)")
    if "Scanner" in code and ("while" in code or "for" in code):
        score += 10
        reasons.append("Tem REPL (+10)")
    return min(score, 100), reasons


def evaluate(activity: Activity, code: str, compiles: bool, runs: bool, stdout: str) -> tuple[int, list[str]]:
    if activity.key == "hello_world_java":
        return score_hello_world(code, compiles, runs, stdout)
    return score_calculator(code, compiles, runs, stdout)


def run_battery() -> list[dict[str, Any]]:
    OUTPUT_DIR.mkdir(parents=True, exist_ok=True)
    results: list[dict[str, Any]] = []
    print("=" * 80)
    print("AirLLM v3.0 — Bateria Java de Modelos Locais")
    print(f"Data: {datetime.now().strftime('%Y-%m-%d %H:%M:%S')}")
    print(f"Modelos locais: {len(LOCAL_MODELS)}")
    print(f"Atividades: {len(ACTIVITIES)}")
    print("=" * 80)

    total = len(LOCAL_MODELS) * len(ACTIVITIES)
    idx = 0
    for activity in ACTIVITIES:
        print(f"\n=== Atividade: {activity.title} ===")
        for model in LOCAL_MODELS:
            idx += 1
            print(f"[{idx}/{total}] {model} -> {activity.key}")
            response, elapsed, err = call_ollama(model, activity.prompt, activity.timeout_seconds)
            if err:
                print(f"  ERRO: {err[:120]}")
                results.append(
                    {
                        "model": model,
                        "activity": activity.key,
                        "activity_title": activity.title,
                        "time_seconds": round(elapsed, 2),
                        "error": err,
                        "code": "",
                        "compiles": False,
                        "runs": False,
                        "score": 0,
                        "reasons": ["Erro na chamada"],
                    }
                )
                continue

            code = extract_java_code(response)
            compiles, runs, stdout, stderr = compile_and_run(activity, model, code)
            score, reasons = evaluate(activity, code, compiles, runs, stdout)
            print(f"  tempo={elapsed:.1f}s compila={'sim' if compiles else 'nao'} executa={'sim' if runs else 'nao'} score={score}")
            results.append(
                {
                    "model": model,
                    "activity": activity.key,
                    "activity_title": activity.title,
                    "time_seconds": round(elapsed, 2),
                    "error": None,
                    "code": code,
                    "compiles": compiles,
                    "runs": runs,
                    "score": score,
                    "reasons": reasons,
                    "stdout": stdout,
                    "stderr": stderr,
                    "response_length": len(response),
                }
            )
    return results


def write_report(results: list[dict[str, Any]]) -> None:
    json_path = OUTPUT_DIR / "java_battery_results.json"
    json_path.write_text(json.dumps(results, indent=2, ensure_ascii=False), encoding="utf-8")

    lines: list[str] = []
    lines.append("# Bateria Java — Modelos Locais do Ollama")
    lines.append("")
    lines.append(f"> **Data**: {datetime.now().strftime('%Y-%m-%d %H:%M:%S')}")
    lines.append(f"> **Modelos locais testados**: {len(LOCAL_MODELS)}")
    lines.append(f"> **Atividades**: {', '.join(a.title for a in ACTIVITIES)}")
    lines.append("")

    # documentation audit
    lines.append("## Auditoria da Documentação")
    lines.append("")
    lines.append("- O teste anterior da calculadora já estava documentado em `benchmark_results/benchmark_report.md`.")
    lines.append("- Faltava um relatório consolidado da bateria e um ranking por atividade/media.")
    lines.append("- Este arquivo fecha essa lacuna e passa a ser o relatório mestre dos testes Java locais.")
    lines.append("")

    # per activity ranking
    averages: dict[str, list[int]] = {m: [] for m in LOCAL_MODELS}
    for activity in ACTIVITIES:
        lines.append(f"## Ranking — {activity.title}")
        lines.append("")
        lines.append("| # | Modelo | Tempo | Compila? | Executa? | Score |")
        lines.append("|---|---|---:|:---:|:---:|---:|")
        filtered = [r for r in results if r["activity"] == activity.key]
        filtered.sort(key=lambda r: (-r["score"], r["time_seconds"]))
        for pos, row in enumerate(filtered, 1):
            averages[row["model"]].append(row["score"])
            lines.append(
                f"| {pos} | `{row['model']}` | {row['time_seconds']:.1f}s | {'✅' if row['compiles'] else '❌'} | {'✅' if row['runs'] else '❌'} | **{row['score']}** |"
            )
        lines.append("")

    # average ranking
    lines.append("## Média de Precisão por Modelo")
    lines.append("")
    lines.append("| # | Modelo | Média | Hello World | Calculadora |")
    lines.append("|---|---|---:|---:|---:|")
    model_rows = []
    for model in LOCAL_MODELS:
        scores = averages.get(model, [])
        avg = sum(scores) / len(scores) if scores else 0.0
        hello = next((r["score"] for r in results if r["model"] == model and r["activity"] == "hello_world_java"), 0)
        calc = next((r["score"] for r in results if r["model"] == model and r["activity"] == "calculator_java"), 0)
        model_rows.append((model, avg, hello, calc))
    model_rows.sort(key=lambda item: (-item[1], item[0]))
    for pos, (model, avg, hello, calc) in enumerate(model_rows, 1):
        lines.append(f"| {pos} | `{model}` | **{avg:.1f}** | {hello} | {calc} |")
    lines.append("")

    # activity-specialist ranking
    lines.append("## Ranking por Tipo de Atividade")
    lines.append("")
    lines.append("- **Melhor em Hello World**: `" + max((r for r in results if r['activity'] == 'hello_world_java'), key=lambda r: (r['score'], -r['time_seconds']))['model'] + "`")
    lines.append("- **Melhor em Calculadora**: `" + max((r for r in results if r['activity'] == 'calculator_java'), key=lambda r: (r['score'], -r['time_seconds']))['model'] + "`")
    lines.append("- **Melhor média geral**: `" + model_rows[0][0] + "`")
    lines.append("")

    lines.append("## Bateria Completa de Testes")
    lines.append("")
    lines.append("Atividades atualmente executadas nesta bateria:")
    lines.append("")
    lines.append("1. **Hello World em Java** — mede aderência mínima, compilação e execução básicas.")
    lines.append("2. **Calculadora CLI em Java** — mede implementação com múltiplas operações, REPL, tratamento de erro e validade prática.")
    lines.append("")
    lines.append("Próximas atividades recomendadas para ampliar a bateria:")
    lines.append("")
    lines.append("3. Refactor de código Java legado")
    lines.append("4. Geração de testes unitários em Java/JUnit")
    lines.append("5. Correção de bug em projeto Java")
    lines.append("6. Implementação de endpoint REST em Java")
    lines.append("")

    md_path = OUTPUT_DIR / "java_battery_report.md"
    md_path.write_text("\n".join(lines), encoding="utf-8")


def main() -> None:
    results = run_battery()
    write_report(results)
    print("\nArquivos gerados:")
    print(f"- {OUTPUT_DIR / 'java_battery_results.json'}")
    print(f"- {OUTPUT_DIR / 'java_battery_report.md'}")


if __name__ == "__main__":
    main()
