#!/usr/bin/env python3
"""Summarize statistically significant Criterion changes for a pull request."""

import json
import os
from pathlib import Path

REGRESSION = 0.05
IMPROVEMENT = -0.10
MARKER = "<!-- lxr-performance -->"

root = Path(os.environ.get("CRITERION_HOME", "lxr/target/criterion"))
regressions = []
improvements = []
controls = []
for estimate in root.glob("**/change/estimates.json"):
    mean = json.loads(estimate.read_text(encoding="utf-8"))["mean"]
    interval = mean["confidence_interval"]
    name = estimate.parent.parent.relative_to(root).as_posix()
    row = (name, mean["point_estimate"], interval["lower_bound"], interval["upper_bound"])
    if name.endswith("/logos"):
        controls.append(row)
    elif name.endswith("/lxr") and row[2] > REGRESSION:
        regressions.append(row)
    elif name.endswith("/lxr") and row[3] < IMPROVEMENT:
        improvements.append(row)

def percent(value):
    return f"{value:+.1%}"

lines = [MARKER]
if regressions:
    lines += ["## Performance regression detected", "", "These LXR benchmarks are at least 5% slower with 95% confidence:", ""]
elif improvements:
    lines += ["## Significant performance improvement", "", "These LXR benchmarks are at least 10% faster with 95% confidence:", ""]
else:
    lines += ["No reportable performance change."]

reported = regressions + improvements
if reported:
    lines += ["| Benchmark | Estimated change | 95% confidence interval |", "|---|---:|---:|"]
    lines += [f"| `{name.removesuffix('/lxr')}` | {percent(point)} | {percent(low)} to {percent(high)} |" for name, point, low, high in reported]
if regressions and improvements:
    lines += ["", "The same pull request also contains significant improvements."]
if reported and any(low > REGRESSION or high < IMPROVEMENT for _, _, low, high in controls):
    lines += ["", "> Logos control benchmarks also moved significantly. Treat this result cautiously because the runner may have changed speed."]

Path(os.environ["BENCHMARK_COMMENT"]).write_text("\n".join(lines) + "\n", encoding="utf-8")
with Path(os.environ["GITHUB_OUTPUT"]).open("a", encoding="utf-8") as output:
    output.write(f"comment={'true' if reported else 'false'}\n")
    output.write(f"regression={'true' if regressions else 'false'}\n")
