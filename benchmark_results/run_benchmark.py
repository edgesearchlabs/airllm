#!/usr/bin/env python3
"""
AirLLM v3.0 — Benchmark de Modelos Ollama
Gera uma calculadora em Java com cada modelo e mede tempo + qualidade.
"""

import subprocess
import json
import time
import os
import sys
import re
from datetime import datetime

OLLAMA_URL = "http://localhost:11434"
REQUEST_TIMEOUT_SECONDS = 120

PROMPT = """Write a complete, working Java calculator application. Requirements:

1. Create a file called Calculator.java
2. Implement a command-line calculator that:
   - Supports addition, subtraction, multiplication, division
   - Handles division by zero gracefully
   - Has a clean interactive REPL interface (reads from stdin)
   - Supports 'quit' command to exit
   - Shows a help menu with 'help' command
3. Include proper error handling
4. Include input validation
5. Make it compile and run with: javac Calculator.java && java Calculator

Return ONLY the Java code in a single code block. No explanations outside the code block.
"""

# All models to test
MODELS = [
    # Local models
    ("qwen3.5:4b", "local", "4.7B"),
    ("qwen3.6:27b", "local", "27.8B"),
    ("qwen3-coder-next:q8_0", "local", "79.7B"),
    ("granite4.1:30b", "local", "30B"),
    ("nemotron-3-nano:30b", "local", "30B"),
    ("jaahas/crow:9b", "local", "9B"),
    # Cloud models
    ("qwen3.5:397b-cloud", "cloud", "397B"),
    ("kimi-k2.7-code:cloud", "cloud", "1T"),
    ("glm-5.2:cloud", "cloud", "756B"),
    ("kimi-k2.6:cloud", "cloud", "—"),
    ("minimax-m2.7:cloud", "cloud", "—"),
]

OUTPUT_DIR = "/home/eriktonon/airllm/benchmark_results"

def call_ollama(model, prompt):
    """Call Ollama API and return (response_text, elapsed_seconds, error)."""
    payload = {
        "model": model,
        "messages": [
            {"role": "system", "content": "You are an expert Java programmer. Write clean, compilable code."},
            {"role": "user", "content": prompt}
        ],
        "stream": False,
        "options": {
            "temperature": 0.3,
            "top_p": 0.9,
            "num_ctx": 8192
        }
    }

    import urllib.request
    import urllib.error

    data = json.dumps(payload).encode("utf-8")
    req = urllib.request.Request(
        f"{OLLAMA_URL}/api/chat",
        data=data,
        headers={"Content-Type": "application/json"},
    )

    start = time.time()
    try:
        with urllib.request.urlopen(req, timeout=REQUEST_TIMEOUT_SECONDS) as resp:
            result = json.loads(resp.read().decode("utf-8"))
            elapsed = time.time() - start
            content = result.get("message", {}).get("content", "")
            return content, elapsed, None
    except urllib.error.HTTPError as e:
        elapsed = time.time() - start
        body = e.read().decode("utf-8", errors="replace")
        return "", elapsed, f"HTTP {e.code}: {body[:200]}"
    except Exception as e:
        elapsed = time.time() - start
        return "", elapsed, str(e)


def extract_java_code(text):
    """Extract Java code from markdown code blocks."""
    # Try to find ```java ... ``` block
    match = re.search(r'```java\s*\n(.*?)```', text, re.DOTALL)
    if match:
        return match.group(1).strip()

    # Try generic ``` ... ``` block
    match = re.search(r'```\s*\n(.*?)```', text, re.DOTALL)
    if match:
        code = match.group(1).strip()
        if "class" in code and "public" in code:
            return code

    # Try to find raw Java code (starts with import or public class)
    lines = text.split("\n")
    code_lines = []
    in_code = False
    for line in lines:
        if line.strip().startswith("import ") or line.strip().startswith("public class"):
            in_code = True
        if in_code:
            code_lines.append(line)
            if line.strip() == "}" and len(code_lines) > 5:
                break

    if code_lines:
        return "\n".join(code_lines)

    return text  # return raw if nothing found


def compile_and_test_java(code, model_name):
    """Try to compile and run the Java code. Return (compiles, runs, output, error)."""
    safe_name = re.sub(r"[^A-Za-z0-9_]", "_", model_name)
    java_file = os.path.join(OUTPUT_DIR, f"Calculator_{safe_name}.java")

    if not code or not code.strip() or "class" not in code:
        return False, False, "", "No valid Java code/class found in model output"

    # Rename class to match file
    code_renamed = re.sub(r'public\s+class\s+\w+', f'public class Calculator_{safe_name}', code)
    code_renamed = re.sub(r'class\s+\w+\s*\{', f'class Calculator_{safe_name} {{', code_renamed, count=1)

    with open(java_file, "w") as f:
        f.write(code_renamed)

    # Try to compile
    compile_result = subprocess.run(
        ["javac", java_file],
        capture_output=True,
        text=True,
        timeout=30,
        cwd=OUTPUT_DIR
    )

    if compile_result.returncode != 0:
        return False, False, "", compile_result.stderr[:500]

    # Try to run with a simple test: "5 + 3" then "quit"
    class_name = f"Calculator_{safe_name}"
    try:
        run_result = subprocess.run(
            ["java", "-cp", OUTPUT_DIR, class_name],
            input="5 + 3\nquit\n",
            capture_output=True,
            text=True,
            timeout=10,
        )
        return True, run_result.returncode == 0, run_result.stdout[:500], run_result.stderr[:500]
    except subprocess.TimeoutExpired:
        return True, False, "", "Timeout (10s)"
    except Exception as e:
        return True, False, "", str(e)


def evaluate_quality(code, compiles, runs, output):
    """Score the code quality on a scale of 0-100."""
    score = 0
    reasons = []

    # Compiles?
    if compiles:
        score += 25
        reasons.append("Compila sem erros (+25)")
    else:
        reasons.append("Não compila (+0)")

    # Runs?
    if runs:
        score += 20
        reasons.append("Executa sem crash (+20)")
    else:
        reasons.append("Não executa ou crash (+0)")

    # Has the 4 operations?
    for op, name in [("+", "adição"), ("-", "subtração"), ("*", "multiplicação"), ("/", "divisão")]:
        if op in code:
            score += 5
            reasons.append(f"Suporta {name} (+5)")

    # Division by zero handling
    if "zero" in code.lower() or "arithmetic" in code.lower() or "by zero" in code.lower():
        score += 10
        reasons.append("Trata divisão por zero (+10)")

    # Has help command
    if "help" in code.lower():
        score += 5
        reasons.append("Tem comando help (+5)")

    # Has quit/exit command
    if "quit" in code.lower() or "exit" in code.lower():
        score += 5
        reasons.append("Tem comando quit/exit (+5)")

    # Error handling (try-catch)
    if "try" in code and "catch" in code:
        score += 10
        reasons.append("Tem try-catch para erros (+10)")

    # Input validation
    if "valid" in code.lower() or "matches" in code.lower() or "scanner" in code.lower():
        score += 5
        reasons.append("Validação de input (+5)")

    # Interactive REPL (Scanner + loop)
    if "scanner" in code.lower() and ("while" in code.lower() or "for" in code.lower()):
        score += 10
        reasons.append("REPL interativo com loop (+10)")

    # Code cleanliness (indentation)
    lines = code.split("\n")
    indented = sum(1 for l in lines if l.startswith("    ") or l.startswith("\t"))
    if len(lines) > 10 and indented > len(lines) * 0.3:
        score += 5
        reasons.append("Código bem indentado (+5)")

    return min(score, 100), reasons


def main():
    os.makedirs(OUTPUT_DIR, exist_ok=True)

    print("=" * 80)
    print("AirLLM v3.0 — Benchmark de Modelos Ollama")
    print(f"Data: {datetime.now().strftime('%Y-%m-%d %H:%M:%S')}")
    print(f"Prompt: Calculadora em Java")
    print(f"Modelos: {len(MODELS)}")
    print("=" * 80)
    print()

    results = []

    for i, (model, model_type, size) in enumerate(MODELS, 1):
        if model_type == "cloud":
            print(f"[{i}/{len(MODELS)}] Pulando: {model} ({model_type}) — solicitado sem cloud")
            continue

        print(f"[{i}/{len(MODELS)}] Testando: {model} ({model_type}, {size})...")

        # Call model
        response, elapsed, error = call_ollama(model, PROMPT)

        if error:
            print(f"  ❌ ERRO: {error[:100]}")
            print(f"  Tempo: {elapsed:.1f}s")
            results.append({
                "model": model,
                "type": model_type,
                "size": size,
                "time_seconds": round(elapsed, 2),
                "error": error,
                "code": "",
                "compiles": False,
                "runs": False,
                "quality_score": 0,
                "quality_reasons": ["Erro na chamada"],
                "response_length": 0,
            })
            print()
            continue

        # Extract code
        code = extract_java_code(response)
        print(f"  📝 Resposta: {len(response)} chars, código: {len(code)} chars")
        print(f"  ⏱️  Tempo: {elapsed:.1f}s")

        # Compile and test
        compiles, runs, output, compile_error = compile_and_test_java(code, model)

        if compiles:
            print(f"  ✅ Compila: SIM")
        else:
            print(f"  ❌ Compila: NÃO — {compile_error[:80]}")

        if runs:
            print(f"  ✅ Executa: SIM — output: {output[:60]}")
        else:
            print(f"  ❌ Executa: NÃO — {compile_error[:60] if not compiles else output[:60]}")

        # Evaluate quality
        score, reasons = evaluate_quality(code, compiles, runs, output)
        print(f"  🏆 Qualidade: {score}/100")

        results.append({
            "model": model,
            "type": model_type,
            "size": size,
            "time_seconds": round(elapsed, 2),
            "error": None,
            "code": code,
            "compiles": compiles,
            "runs": runs,
            "quality_score": score,
            "quality_reasons": reasons,
            "response_length": len(response),
            "compile_error": compile_error if not compiles else None,
            "run_output": output if runs else None,
        })

        print()

    # Save raw results
    results_file = os.path.join(OUTPUT_DIR, "benchmark_results.json")
    with open(results_file, "w") as f:
        json.dump(results, f, indent=2, ensure_ascii=False)

    # Generate report
    generate_report(results)

    print(f"\n📊 Resultados salvos em: {OUTPUT_DIR}/")
    print(f"   - benchmark_results.json (dados completos)")
    print(f"   - benchmark_report.md (relatório)")


def generate_report(results):
    """Generate markdown report with rankings."""
    report = []
    report.append("# Benchmark AirLLM v3.0 — Calculadora em Java")
    report.append("")
    report.append(f"> **Data**: {datetime.now().strftime('%Y-%m-%d %H:%M:%S')}")
    report.append(f"> **Modelos testados**: {len(results)}")
    report.append(f"> **Tarefa**: Gerar uma calculadora Java completa e funcional")
    report.append("")
    report.append("---")
    report.append("")

    # Summary table
    report.append("## 📊 Ranking Geral")
    report.append("")
    report.append("| # | Modelo | Tipo | Tamanho | Tempo | Compila? | Executa? | Qualidade |")
    report.append("|---|--------|------|---------|-------|----------|----------|-----------|")

    # Sort by quality score (desc), then by time (asc)
    sorted_results = sorted(results, key=lambda r: (-r["quality_score"], r["time_seconds"]))

    for i, r in enumerate(sorted_results, 1):
        time_str = f'{r["time_seconds"]:.1f}s'
        compiles_str = "✅" if r["compiles"] else "❌"
        runs_str = "✅" if r["runs"] else "❌"
        score_str = f'{r["quality_score"]}/100'
        error_note = f' ⚠️ {r["error"][:30]}' if r.get("error") else ""
        report.append(
            f'| {i} | `{r["model"]}` | {r["type"]} | {r["size"]} | {time_str} | {compiles_str} | {runs_str} | **{score_str}**{error_note} |'
        )

    report.append("")
    report.append("---")
    report.append("")

    # Speed ranking
    report.append("## ⚡ Ranking por Velocidade")
    report.append("")
    report.append("| # | Modelo | Tempo | Qualidade |")
    report.append("|---|--------|-------|-----------|")

    speed_sorted = sorted(
        [r for r in results if not r.get("error")],
        key=lambda r: r["time_seconds"]
    )
    for i, r in enumerate(speed_sorted, 1):
        report.append(f'| {i} | `{r["model"]}` | {r["time_seconds"]:.1f}s | {r["quality_score"]}/100 |')

    report.append("")
    report.append("---")
    report.append("")

    # Quality ranking
    report.append("## 🏆 Ranking por Qualidade")
    report.append("")
    report.append("| # | Modelo | Score | Tempo | Detalhes |")
    report.append("|---|--------|-------|-------|----------|")

    quality_sorted = sorted(results, key=lambda r: -r["quality_score"])
    for i, r in enumerate(quality_sorted, 1):
        reasons = ", ".join(r["quality_reasons"][:3])
        report.append(f'| {i} | `{r["model"]}` | {r["quality_score"]}/100 | {r["time_seconds"]:.1f}s | {reasons} |')

    report.append("")
    report.append("---")
    report.append("")

    # Best overall
    best = sorted_results[0] if sorted_results else None
    fastest = speed_sorted[0] if speed_sorted else None

    report.append("## 🎯 Conclusões")
    report.append("")
    if best:
        report.append(f"### 🏆 Melhor Qualidade Geral")
        report.append(f"- **Modelo**: `{best['model']}` ({best['type']}, {best['size']})")
        report.append(f"- **Score**: {best['quality_score']}/100")
        report.append(f"- **Tempo**: {best['time_seconds']:.1f}s")
        report.append(f"- **Compila**: {'✅' if best['compiles'] else '❌'}")
        report.append(f"- **Executa**: {'✅' if best['runs'] else '❌'}")
        report.append("")

    if fastest:
        report.append(f"### ⚡ Mais Rápido")
        report.append(f"- **Modelo**: `{fastest['model']}` ({fastest['type']}, {fastest['size']})")
        report.append(f"- **Tempo**: {fastest['time_seconds']:.1f}s")
        report.append(f"- **Qualidade**: {fastest['quality_score']}/100")
        report.append("")

    # Best cost-benefit (quality / time)
    cost_benefit = sorted(
        [r for r in results if not r.get("error") and r["time_seconds"] > 0],
        key=lambda r: r["quality_score"] / r["time_seconds"],
        reverse=True
    )
    if cost_benefit:
        cb = cost_benefit[0]
        report.append(f"### 💰 Melhor Custo-Benefício (Qualidade/Tempo)")
        report.append(f"- **Modelo**: `{cb['model']}` ({cb['type']}, {cb['size']})")
        report.append(f"- **Score**: {cb['quality_score']}/100 em {cb['time_seconds']:.1f}s")
        ratio = cb["quality_score"] / cb["time_seconds"]
        report.append(f"- **Ratio**: {ratio:.1f} pontos/segundo")
        report.append("")

    # Local vs Cloud comparison
    local_results = [r for r in results if r["type"] == "local" and not r.get("error")]
    cloud_results = [r for r in results if r["type"] == "cloud" and not r.get("error")]

    if local_results and cloud_results:
        local_avg_time = sum(r["time_seconds"] for r in local_results) / len(local_results)
        cloud_avg_time = sum(r["time_seconds"] for r in cloud_results) / len(cloud_results)
        local_avg_score = sum(r["quality_score"] for r in local_results) / len(local_results)
        cloud_avg_score = sum(r["quality_score"] for r in cloud_results) / len(cloud_results)

        report.append("### 🌐 Local vs Cloud")
        report.append("")
        report.append("| Categoria | Tempo Médio | Qualidade Média |")
        report.append("|-----------|-------------|-----------------|")
        report.append(f"| Local ({len(local_results)} modelos) | {local_avg_time:.1f}s | {local_avg_score:.0f}/100 |")
        report.append(f"| Cloud ({len(cloud_results)} modelos) | {cloud_avg_time:.1f}s | {cloud_avg_score:.0f}/100 |")
        report.append("")

    report.append("---")
    report.append("")
    report.append("## 📝 Detalhes por Modelo")
    report.append("")

    for r in sorted_results:
        report.append(f"### `{r['model']}` ({r['type']}, {r['size']})")
        report.append("")
        report.append(f"- **Tempo**: {r['time_seconds']:.1f}s")
        report.append(f"- **Compila**: {'✅' if r['compiles'] else '❌'}")
        report.append(f"- **Executa**: {'✅' if r['runs'] else '❌'}")
        report.append(f"- **Qualidade**: {r['quality_score']}/100")
        if r.get("error"):
            report.append(f"- **Erro**: {r['error']}")
        if r.get("compile_error"):
            report.append(f"- **Erro de compilação**: `{r['compile_error'][:100]}`")
        report.append(f"- **Tamanho da resposta**: {r['response_length']} chars")
        report.append(f"- **Critérios de qualidade**:")
        for reason in r["quality_reasons"]:
            report.append(f"  - {reason}")
        report.append("")

    report_file = os.path.join(OUTPUT_DIR, "benchmark_report.md")
    with open(report_file, "w") as f:
        f.write("\n".join(report))


if __name__ == "__main__":
    main()