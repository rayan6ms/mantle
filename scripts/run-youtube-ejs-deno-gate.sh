#!/usr/bin/env bash
set -euo pipefail

root_dir=$(cd "$(dirname "$0")/.." && pwd)
manifest="$root_dir/reference/youtube-ejs-provider-0.8.0.json"

if ! command -v deno >/dev/null 2>&1; then
  echo "Deno is required for the YouTube EJS no-permission gate" >&2
  exit 1
fi

expected_version=$(jq -r '.runtime.tested_version' "$manifest")
actual_version=$(deno --version | awk 'NR == 1 {print $2}')
if [[ "$actual_version" != "$expected_version" ]]; then
  printf 'Deno version mismatch: expected %s, got %s\n' \
    "$expected_version" "$actual_version" >&2
  exit 1
fi

"$root_dir/scripts/check-youtube-ejs-provider.sh"

deno_bin=$(command -v deno)
if [[ "$deno_bin" != /* ]]; then
  deno_bin=$(cd "$(dirname "$deno_bin")" && pwd)/$(basename "$deno_bin")
fi

MANTLE_DENO_BIN="$deno_bin" cargo test --locked -p mantle-media \
  --test phase12_youtube \
  deno_process_cipher_provider_executes_packaged_adapter_without_permissions \
  -- --ignored --exact

echo "YouTube EJS Deno no-permission gate passed"
