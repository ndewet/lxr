#!/usr/bin/env node

import { appendFileSync, readFileSync, readdirSync } from "node:fs";
import { basename, dirname, relative, resolve, sep } from "node:path";
import { pathToFileURL } from "node:url";

function findEstimateFiles(directory) {
  const estimates = [];
  for (const entry of readdirSync(directory, { withFileTypes: true })) {
    const path = resolve(directory, entry.name);
    if (entry.isDirectory()) {
      estimates.push(...findEstimateFiles(path));
    } else if (entry.name === "estimates.json" && basename(directory) === "new") {
      estimates.push(path);
    }
  }
  return estimates;
}

export function loadResults(criterionDirectory) {
  return findEstimateFiles(criterionDirectory)
    .sort()
    .map((estimatePath) => {
      const benchmarkDirectory = dirname(dirname(estimatePath));
      const name = relative(criterionDirectory, benchmarkDirectory).split(sep).join("/");
      const changePath = resolve(benchmarkDirectory, "change", "estimates.json");
      let change;
      try {
        change = JSON.parse(readFileSync(changePath, "utf8")).mean;
      } catch (error) {
        if (error.code === "ENOENT") {
          return { name, estimate: null, lowerBound: null, upperBound: null };
        }
        throw error;
      }
      return {
        name,
        estimate: Number(change.point_estimate),
        lowerBound: Number(change.confidence_interval.lower_bound),
        upperBound: Number(change.confidence_interval.upper_bound),
      };
    });
}

function percentage(value) {
  if (value === null) {
    return "new";
  }
  const sign = value >= 0 ? "+" : "";
  return `${sign}${(value * 100).toFixed(2)}%`;
}

export function makeReport(results, threshold) {
  const regressions = results.filter(
    (result) => result.lowerBound !== null && result.lowerBound > threshold,
  );
  const lines = [
    "## Benchmark comparison",
    "",
    `The regression limit is ${(threshold * 100).toFixed(1)}%.`,
    "A benchmark fails only when its complete 99% confidence interval exceeds the limit.",
    "",
    "| Benchmark | Estimate | 99% confidence interval | Result |",
    "| --- | ---: | ---: | --- |",
  ];
  for (const result of results) {
    const isNew = result.estimate === null;
    const interval = isNew
      ? "new benchmark"
      : `${percentage(result.lowerBound)} to ${percentage(result.upperBound)}`;
    const status = isNew
      ? "Not compared"
      : regressions.includes(result)
        ? "Regression"
        : "Pass";
    lines.push(`| \`${result.name}\` | ${percentage(result.estimate)} | ${interval} | ${status} |`);
  }
  return { report: `${lines.join("\n")}\n`, regressions };
}

function main() {
  const args = process.argv.slice(2);
  const thresholdIndex = args.indexOf("--threshold");
  const threshold = thresholdIndex === -1 ? 0.05 : Number(args[thresholdIndex + 1]);
  const criterionDirectory = resolve(args[0]);
  const results = loadResults(criterionDirectory);
  if (results.length === 0) {
    throw new Error(`no Criterion results found in ${criterionDirectory}`);
  }
  if (!results.some((result) => result.estimate !== null)) {
    throw new Error("Criterion did not compare any benchmarks to the baseline");
  }

  const { report, regressions } = makeReport(results, threshold);
  process.stdout.write(report);
  if (process.env.GITHUB_STEP_SUMMARY) {
    appendFileSync(process.env.GITHUB_STEP_SUMMARY, report, "utf8");
  }
  process.exitCode = regressions.length === 0 ? 0 : 1;
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  main();
}
