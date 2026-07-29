export const DISPOSABLE_DATABASE_MARKER = "hiddenshield_migrate_smoke";

export function assertDisposablePostgresDatabaseUrl(databaseUrl) {
  let parsedUrl;
  try {
    parsedUrl = new URL(databaseUrl);
  } catch {
    throw new Error(
      "ai-transparency:postgres-qa requires a valid PostgreSQL test database URL.",
    );
  }

  if (!["postgres:", "postgresql:"].includes(parsedUrl.protocol)) {
    throw new Error(
      "ai-transparency:postgres-qa requires a PostgreSQL test database URL.",
    );
  }

  const databaseName = decodeURIComponent(parsedUrl.pathname).replace(/^\/+/, "");
  if (!databaseName.includes(DISPOSABLE_DATABASE_MARKER)) {
    throw new Error(
      `ai-transparency:postgres-qa refuses a database URL unless its database name contains ${DISPOSABLE_DATABASE_MARKER}.`,
    );
  }

  return databaseUrl;
}
