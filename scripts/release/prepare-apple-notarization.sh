#!/usr/bin/env bash
set -euo pipefail

if [[ -n "${APPLE_API_KEY_CONTENT:-}" && -n "${APPLE_API_KEY:-}" && -n "${APPLE_API_ISSUER:-}" ]]; then
  KEY_PATH="${RUNNER_TEMP:-/tmp}/AuthKey_${APPLE_API_KEY}.p8"
  printf "%s" "$APPLE_API_KEY_CONTENT" > "$KEY_PATH"
  {
    echo "APPLE_API_KEY_PATH=$KEY_PATH"
  } >> "$GITHUB_ENV"
  echo "Configured notarization via App Store Connect API key"
  exit 0
fi

missing=()
for var in APPLE_ID APPLE_PASSWORD APPLE_TEAM_ID; do
  if [[ -z "${!var:-}" ]]; then
    missing+=("$var")
  fi
done

if (( ${#missing[@]} > 0 )); then
  echo "Missing macOS notarization credentials. Provide either APPLE_API_KEY_CONTENT + APPLE_API_KEY + APPLE_API_ISSUER, or APPLE_ID + APPLE_PASSWORD + APPLE_TEAM_ID." >&2
  exit 1
fi

echo "Configured notarization via Apple ID credentials"
