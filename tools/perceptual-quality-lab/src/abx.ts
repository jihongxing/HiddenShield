import type { AbxIdentity, AbxSummary, AbxTrial } from "./types";

export function createAbxTrials(count: number, random = secureRandom): AbxTrial[] {
  if (count <= 0 || count % 2 !== 0) {
    throw new Error("ABX trial count must be a positive even number");
  }
  const xIdentities: AbxIdentity[] = [
    ...Array.from({ length: count / 2 }, () => "source" as const),
    ...Array.from({ length: count / 2 }, () => "candidate" as const),
  ];
  shuffle(xIdentities, random);
  return xIdentities.map((xIdentity, index) => {
    const a = random() >= 0.5 ? "source" : "candidate";
    const b = a === "source" ? "candidate" : "source";
    return {
      index,
      a,
      b,
      x: xIdentity === a ? "a" : "b",
      confidence: 3,
      perceivedDifference: "none",
      notes: "",
    };
  });
}

export function summarizeAbx(
  trials: AbxTrial[],
  stableDifferenceObserved: boolean,
): AbxSummary {
  const correct = trials.filter((trial) => trial.answer === trial.x).length;
  const total = trials.length;
  const correctRate = total ? correct / total : 0;
  const pValue = binomialUpperTail(total, correct, 0.5);
  let conclusion: AbxSummary["conclusion"];
  if (stableDifferenceObserved || correctRate > 0.7 || pValue < 0.05) {
    conclusion = "review_required";
  } else if (correctRate > 0.6) {
    conclusion = "inconclusive";
  } else {
    conclusion = "no_stable_evidence";
  }
  return { correct, total, correctRate, pValue, conclusion };
}

export function binomialUpperTail(n: number, k: number, probability: number) {
  let total = 0;
  for (let successes = k; successes <= n; successes += 1) {
    total +=
      combination(n, successes) *
      probability ** successes *
      (1 - probability) ** (n - successes);
  }
  return Math.min(1, total);
}

function shuffle<T>(items: T[], random: () => number) {
  for (let index = items.length - 1; index > 0; index -= 1) {
    const swap = Math.floor(random() * (index + 1));
    [items[index], items[swap]] = [items[swap], items[index]];
  }
}

function combination(n: number, k: number) {
  const effective = Math.min(k, n - k);
  let value = 1;
  for (let index = 1; index <= effective; index += 1) {
    value = (value * (n - effective + index)) / index;
  }
  return value;
}

function secureRandom() {
  const buffer = new Uint32Array(1);
  crypto.getRandomValues(buffer);
  return buffer[0] / 0x1_0000_0000;
}
