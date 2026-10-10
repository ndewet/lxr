import assert from "node:assert/strict";
import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { resolve } from "node:path";
import test from "node:test";

import { loadResults, makeReport } from "./check_benchmarks.mjs";

function writeResult(root, name, estimate, lowerBound = 0, upperBound = 0) {
  const benchmark = resolve(root, name);
  const newDirectory = resolve(benchmark, "new");
  mkdirSync(newDirectory, { recursive: true });
  writeFileSync(resolve(newDirectory, "estimates.json"), "{}", "utf8");
  if (estimate === null) {
    return;
  }
  const changeDirectory = resolve(benchmark, "change");
  mkdirSync(changeDirectory);
  const change = {
    mean: {
      point_estimate: estimate,
      confidence_interval: { lower_bound: lowerBound, upper_bound: upperBound },
    },
  };
  writeFileSync(resolve(changeDirectory, "estimates.json"), JSON.stringify(change), "utf8");
}

function withTemporaryDirectory(callback) {
  const directory = mkdtempSync(resolve(tmpdir(), "runeweaver-benchmarks-"));
  try {
    callback(directory);
  } finally {
    rmSync(directory, { recursive: true });
  }
}

test("only confident changes above the limit are reported", () => {
  withTemporaryDirectory((root) => {
    writeResult(root, "clear-regression", 0.08, 0.06, 0.1);
    writeResult(root, "uncertain-change", 0.08, 0.04, 0.12);
    writeResult(root, "improvement", -0.08, -0.1, -0.06);

    const { report, changes, regressions } = makeReport(loadResults(root), 0.05);

    assert.deepEqual(
      regressions.map((result) => result.name),
      ["clear-regression"],
    );
    assert.deepEqual(
      changes.map((result) => [result.name, result.status]),
      [
        ["clear-regression", "Regression"],
        ["improvement", "Improvement"],
      ],
    );
    assert.doesNotMatch(report, /uncertain-change/);
    assert.match(report, /clear-regression/);
    assert.match(report, /improvement/);
  });
});

test("new and unchanged benchmarks are omitted", () => {
  withTemporaryDirectory((root) => {
    writeResult(root, "new-case", null);
    writeResult(root, "unchanged-case", 0.01, -0.01, 0.03);

    const { report, changes, regressions } = makeReport(loadResults(root), 0.05);

    assert.deepEqual(changes, []);
    assert.deepEqual(regressions, []);
    assert.doesNotMatch(report, /new-case/);
    assert.doesNotMatch(report, /unchanged-case/);
    assert.match(report, /No improvements or regressions were found/);
  });
});
