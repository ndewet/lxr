import assert from "node:assert/strict";
import test from "node:test";

import { bumpVersion, classifyCommit, selectLargestChange } from "./prepare_release.mjs";

test("version bumps reset less significant parts", () => {
  assert.deepEqual(bumpVersion([1, 1, 1], "patch"), [1, 1, 2]);
  assert.deepEqual(bumpVersion([1, 1, 1], "minor"), [1, 2, 0]);
  assert.deepEqual(bumpVersion([1, 1, 1], "major"), [2, 0, 0]);
});

test("feat commits select a minor release", () => {
  assert.equal(classifyCommit("feat(parser): add modes", ""), "minor");
});

test("fix commits select a patch release", () => {
  assert.equal(classifyCommit("fix: reject empty rules", ""), "patch");
});

test("a bang selects a major release for any conventional type", () => {
  assert.equal(classifyCommit("refactor(core)!: new API", ""), "major");
});

test("a breaking change footer selects a major release", () => {
  const body = "Some explanation.\n\nBREAKING CHANGE: scanners now own their source";
  assert.equal(classifyCommit("feat: stream input", body), "major");
});

test("unrelated commits have no release effect", () => {
  assert.equal(classifyCommit("docs: clarify examples", ""), "none");
});

test("only the largest change across all commits is selected", () => {
  assert.equal(
    selectLargestChange([
      { subject: "fix: one", body: "" },
      { subject: "feat: two", body: "" },
      { subject: "fix: three", body: "" },
    ]),
    "minor",
  );
  assert.equal(
    selectLargestChange([
      { subject: "feat: one", body: "" },
      { subject: "fix!: break the API", body: "" },
    ]),
    "major",
  );
});
