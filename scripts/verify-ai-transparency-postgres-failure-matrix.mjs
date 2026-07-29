import assert from "node:assert/strict";
import { readFile, readdir } from "node:fs/promises";

const matrix = await readFile("docs/AI生成内容标识PostgreSQL_QA故障注入矩阵.md", "utf8");
const runnerNames = (await readdir("feedback-backend/src/bin"))
  .filter((name) => /^ai_transparency_.*_qa\.rs$/.test(name))
  .map((name) => name.replace(/\.rs$/, ""))
  .sort();
assert.ok(runnerNames.length > 0);
for (const runner of runnerNames) {
  assert.match(matrix, new RegExp("`" + runner + "`"));
}
for (const criterion of ["并发", "回放", "audit 故障", "外部/读取故障", "恢复"]) {
  assert.match(matrix, new RegExp(criterion));
}
assert.match(matrix, /已覆盖\/委托/);
assert.match(matrix, /外部配置 Gate/);

console.log(JSON.stringify({ ok: true, runners: runnerNames.length }));
