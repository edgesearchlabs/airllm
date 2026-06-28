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


@dataclass(frozen=True)
class Probe:
    key: str
    title: str
    inputs: tuple[str, ...]
    expected_tokens: tuple[str, ...] = ()
    expected_number: str | None = None


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

CALCULATOR_PROBES = (
    Probe(
        key="quit",
        title="Encerra com quit",
        inputs=("quit\n",),
    ),
    Probe(
        key="help",
        title="Mostra ajuda",
        inputs=("help\nquit\n",),
        expected_tokens=("help", "commands", "operations", "usage", "quit"),
    ),
    Probe(
        key="addition",
        title="Soma 5 + 3",
        inputs=("5 + 3\nquit\n", "add 5 3\nquit\n"),
        expected_number="8",
    ),
    Probe(
        key="subtraction",
        title="Subtrai 9 - 4",
        inputs=("9 - 4\nquit\n", "sub 9 4\nquit\n", "subtract 9 4\nquit\n"),
        expected_number="5",
    ),
    Probe(
        key="multiplication",
        title="Multiplica 6 * 7",
        inputs=("6 * 7\nquit\n", "mul 6 7\nquit\n", "multiply 6 7\nquit\n"),
        expected_number="42",
    ),
    Probe(
        key="division",
        title="Divide 8 / 2",
        inputs=("8 / 2\nquit\n", "div 8 2\nquit\n", "divide 8 2\nquit\n"),
        expected_number="4",
    ),
    Probe(
        key="division_by_zero",
        title="Trata divisão por zero",
        inputs=("10 / 0\nquit\n", "div 10 0\nquit\n", "divide 10 0\nquit\n"),
        expected_tokens=("division by zero", "divide by zero", "cannot divide", "arithmeticexception", "/ by zero"),
    ),
    Probe(
        key="invalid_input",
        title="Valida entrada inválida",
        inputs=("banana\nquit\n",),
        expected_tokens=("invalid", "error", "unknown", "usage", "help"),
    ),
)

CALCULATOR_BEHAVIOR_WEIGHTS = {
    "compiles": 20,
    "quit": 10,
    "help": 10,
    "addition": 10,
    "subtraction": 10,
    "multiplication": 10,
    "division": 10,
    "division_by_zero": 10,
    "invalid_input": 10,
}


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


def compile_and_run_process(class_name: str, user_input: str) -> tuple[bool, str, str]:
    run_result = subprocess.run(
        ["java", "-cp", str(OUTPUT_DIR), class_name],
        input=user_input,
        capture_output=True,
        text=True,
        timeout=10,
        cwd=OUTPUT_DIR,
    )
    return run_result.returncode == 0, run_result.stdout[:600], run_result.stderr[:600]


def has_expected_output(stdout: str, stderr: str, probe: Probe) -> bool:
    combined = f"{stdout}\n{stderr}".lower()
    if probe.expected_number is not None:
        return re.search(rf"(?<!\d){re.escape(probe.expected_number)}(?:\.0+)?(?!\d)", combined) is not None
    if probe.expected_tokens:
        return any(token in combined for token in probe.expected_tokens)
    return True


def run_calculator_probes(class_name: str) -> dict[str, Any]:
    probe_results: dict[str, Any] = {}
    stdout_samples: list[str] = []
    stderr_samples: list[str] = []
    for probe in CALCULATOR_PROBES:
        passed = False
        matched_input = None
        matched_stdout = ""
        matched_stderr = ""
        for user_input in probe.inputs:
            exited_cleanly, stdout, stderr = compile_and_run_process(class_name, user_input)
            stdout_samples.append(stdout)
            stderr_samples.append(stderr)
            if exited_cleanly and has_expected_output(stdout, stderr, probe):
                passed = True
                matched_input = user_input.strip()
                matched_stdout = stdout
                matched_stderr = stderr
                break
        probe_results[probe.key] = {
            "passed": passed,
            "matched_input": matched_input,
            "stdout": matched_stdout[:300],
            "stderr": matched_stderr[:300],
        }
    return {
        "checks": probe_results,
        "checks_passed": sum(1 for result in probe_results.values() if result["passed"]),
        "checks_total": len(CALCULATOR_PROBES),
        "sample_stdout": next((sample for sample in stdout_samples if sample.strip()), "")[:300],
        "sample_stderr": next((sample for sample in stderr_samples if sample.strip()), "")[:300],
    }


def compile_and_run(activity: Activity, model: str, code: str) -> tuple[bool, bool, str, str, dict[str, Any] | None]:
    safe_model = sanitize_name(model)
    if activity.key == "hello_world_java":
        base_name = f"HelloWorld_{safe_model}"
        filename = OUTPUT_DIR / f"{base_name}.java"
        if not code or "class" not in code:
            return False, False, "", "No valid Java code/class found in model output", None
        code = re.sub(r"public\s+class\s+\w+", f"public class {base_name}", code, count=1)
        code = re.sub(r"class\s+\w+\s*\{", f"class {base_name} {{", code, count=1)
        filename.write_text(code, encoding="utf-8")
        compile_result = subprocess.run(
            ["javac", str(filename)], capture_output=True, text=True, timeout=30, cwd=OUTPUT_DIR
        )
        if compile_result.returncode != 0:
            return False, False, "", compile_result.stderr[:500], None
        exited_cleanly, stdout, stderr = compile_and_run_process(base_name, "")
        return True, exited_cleanly, stdout[:300], stderr[:300], None

    base_name = f"Calculator_{safe_model}"
    filename = OUTPUT_DIR / f"{base_name}.java"
    if not code or "class" not in code:
        return False, False, "", "No valid Java code/class found in model output", None
    code = re.sub(r"public\s+class\s+\w+", f"public class {base_name}", code, count=1)
    code = re.sub(r"class\s+\w+\s*\{", f"class {base_name} {{", code, count=1)
    filename.write_text(code, encoding="utf-8")
    compile_result = subprocess.run(
        ["javac", str(filename)], capture_output=True, text=True, timeout=30, cwd=OUTPUT_DIR
    )
    if compile_result.returncode != 0:
        return False, False, "", compile_result.stderr[:500], None
    behavior = run_calculator_probes(base_name)
    runs = behavior["checks"]["quit"]["passed"]
    return True, runs, behavior["sample_stdout"], behavior["sample_stderr"], behavior


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


def score_calculator(compiles: bool, behavior: dict[str, Any] | None) -> tuple[int, list[str]]:
    score = 0
    reasons: list[str] = []
    if compiles:
        score += CALCULATOR_BEHAVIOR_WEIGHTS["compiles"]
        reasons.append(f"Compila (+{CALCULATOR_BEHAVIOR_WEIGHTS['compiles']})")
    if not behavior:
        return min(score, 100), reasons

    for probe in CALCULATOR_PROBES:
        result = behavior["checks"][probe.key]
        if result["passed"]:
            weight = CALCULATOR_BEHAVIOR_WEIGHTS[probe.key]
            score += weight
            reasons.append(f"{probe.title} (+{weight})")
    return min(score, 100), reasons


def evaluate(
    activity: Activity,
    code: str,
    compiles: bool,
    runs: bool,
    stdout: str,
    behavior: dict[str, Any] | None,
) -> tuple[int, list[str]]:
    if activity.key == "hello_world_java":
        return score_hello_world(code, compiles, runs, stdout)
    return score_calculator(compiles, behavior)


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
            compiles, runs, stdout, stderr, behavior = compile_and_run(activity, model, code)
            score, reasons = evaluate(activity, code, compiles, runs, stdout, behavior)
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
                    "behavior": behavior,
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
        if activity.key == "calculator_java":
            lines.append("| # | Modelo | Tempo | Compila? | Executa? | Checks | Score |")
            lines.append("|---|---|---:|:---:|:---:|---:|---:|")
        else:
            lines.append("| # | Modelo | Tempo | Compila? | Executa? | Score |")
            lines.append("|---|---|---:|:---:|:---:|---:|")
        filtered = [r for r in results if r["activity"] == activity.key]
        filtered.sort(key=lambda r: (-r["score"], r["time_seconds"]))
        for pos, row in enumerate(filtered, 1):
            averages[row["model"]].append(row["score"])
            if activity.key == "calculator_java":
                checks = row.get("behavior", {})
                checks_label = f"{checks.get('checks_passed', 0)}/{checks.get('checks_total', len(CALCULATOR_PROBES))}"
                lines.append(
                    f"| {pos} | `{row['model']}` | {row['time_seconds']:.1f}s | {'✅' if row['compiles'] else '❌'} | {'✅' if row['runs'] else '❌'} | {checks_label} | **{row['score']}** |"
                )
            else:
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
    lines.append("2. **Calculadora CLI em Java** — mede comportamento real com sondas de `help`, `quit`, operações aritméticas, divisão por zero e entrada inválida.")
    lines.append("")
    lines.append("Checks atualmente usados na calculadora:")
    lines.append("")
    for probe in CALCULATOR_PROBES:
        lines.append(f"- {probe.title}")
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
