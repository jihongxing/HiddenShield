import assert from "node:assert/strict";
import {
  DISPOSABLE_DATABASE_MARKER,
  assertDisposablePostgresDatabaseUrl,
} from "./ai-transparency-postgres-qa-contract.mjs";

const acceptedUrls = [
  "postgres://postgres:password@127.0.0.1:5432/hiddenshield_migrate_smoke_ai_transparency_qa",
  "postgresql://postgres:password@localhost/hiddenshield_migrate_smoke_provider_qa",
];
const rejectedUrls = [
  "https://example.invalid/hiddenshield_migrate_smoke_ai_transparency_qa",
  "postgres://postgres:password@127.0.0.1:5432/hiddenshield",
  "not-a-database-url",
];

for (const databaseUrl of acceptedUrls) {
  assert.equal(assertDisposablePostgresDatabaseUrl(databaseUrl), databaseUrl);
}
for (const databaseUrl of rejectedUrls) {
  assert.throws(() => assertDisposablePostgresDatabaseUrl(databaseUrl));
}

console.log(
  JSON.stringify({
    ok: true,
    databaseMarker: DISPOSABLE_DATABASE_MARKER,
    accepted: acceptedUrls.length,
    rejected: rejectedUrls.length,
  }),
);
