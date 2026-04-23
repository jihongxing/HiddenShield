#!/usr/bin/env bash
set -euo pipefail

: "${APPLE_CERTIFICATE:?APPLE_CERTIFICATE is required}"
: "${APPLE_CERTIFICATE_PASSWORD:?APPLE_CERTIFICATE_PASSWORD is required}"
: "${KEYCHAIN_PASSWORD:?KEYCHAIN_PASSWORD is required}"

CERT_PATH="${RUNNER_TEMP:-/tmp}/hiddenshield-certificate.p12"
KEYCHAIN_PATH="${RUNNER_TEMP:-/tmp}/build.keychain-db"

echo "$APPLE_CERTIFICATE" | base64 --decode > "$CERT_PATH"

security create-keychain -p "$KEYCHAIN_PASSWORD" "$KEYCHAIN_PATH"
security default-keychain -s "$KEYCHAIN_PATH"
security unlock-keychain -p "$KEYCHAIN_PASSWORD" "$KEYCHAIN_PATH"
security set-keychain-settings -lut 21600 "$KEYCHAIN_PATH"
security import "$CERT_PATH" -k "$KEYCHAIN_PATH" -P "$APPLE_CERTIFICATE_PASSWORD" -T /usr/bin/codesign -T /usr/bin/security
security set-key-partition-list -S apple-tool:,apple:,codesign: -s -k "$KEYCHAIN_PASSWORD" "$KEYCHAIN_PATH"

CERT_ID="$(
  security find-identity -v -p codesigning "$KEYCHAIN_PATH" \
    | grep -E 'Developer ID Application|Apple Distribution|Apple Development' \
    | head -n 1 \
    | awk -F'"' '{print $2}'
)"

if [[ -z "${CERT_ID}" ]]; then
  echo "Unable to resolve a usable Apple signing identity from imported certificate" >&2
  exit 1
fi

{
  echo "APPLE_SIGNING_IDENTITY=$CERT_ID"
  echo "APPLE_KEYCHAIN_PATH=$KEYCHAIN_PATH"
} >> "$GITHUB_ENV"

echo "Imported Apple signing identity: $CERT_ID"
