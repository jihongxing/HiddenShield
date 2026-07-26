import assert from "node:assert/strict";
import test from "node:test";
import { createAbxTrials, summarizeAbx } from "./abx.ts";

function deterministicRandom(values: number[]) {
  let index = 0;
  return () => values[index++ % values.length];
}

test("ABX trials balance source and candidate X identities", () => {
  const trials = createAbxTrials(
    20,
    deterministicRandom([0.01, 0.81, 0.23, 0.67, 0.42, 0.93]),
  );
  const sourceCount = trials.filter((trial) => trial[trial.x] === "source").length;
  const candidateCount = trials.filter((trial) => trial[trial.x] === "candidate").length;
  assert.equal(sourceCount, 10);
  assert.equal(candidateCount, 10);
});

test("ABX summary requires review for statistically strong discrimination", () => {
  const trials = createAbxTrials(20, () => 0.75);
  trials.forEach((trial, index) => {
    trial.answer = index < 15 ? trial.x : trial.x === "a" ? "b" : "a";
  });
  const summary = summarizeAbx(trials, false);
  assert.equal(summary.correct, 15);
  assert.equal(summary.conclusion, "review_required");
  assert.ok(summary.pValue < 0.05);
});

test("ABX summary avoids imperceptibility claims at chance performance", () => {
  const trials = createAbxTrials(10, () => 0.25);
  trials.forEach((trial, index) => {
    trial.answer = index < 5 ? trial.x : trial.x === "a" ? "b" : "a";
  });
  const summary = summarizeAbx(trials, false);
  assert.equal(summary.conclusion, "no_stable_evidence");
});
