#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 2 ]]; then
  echo "usage: $0 <reviewed-ejs-directory> <output-adapter.js>" >&2
  exit 2
fi

source_dir=$1
output_file=$2
core_file="$source_dir/yt.solver.core.js"
lib_file="$source_dir/yt.solver.lib.js"
tail_file="$(cd "$(dirname "$0")" && pwd)/youtube-ejs-adapter.tail.js"

core_sha3=c163a6f376db6ce3da47d516a28a8f2a0554ae95c58dc766f0a6e2b3894f2cef1ee07fa84beb442fa471aac4f300985added1657c7c94c4d1cfefe68920ab599
lib_sha3=1ee3753a8222fc855f5c39db30a9ccbb7967dbe1fb810e86dc9a89aa073a0907f294c720e9b65427d560a35aa1ce6af19ef854d9126a05ca00afe03f72047733

if [[ ! -f "$core_file" || ! -f "$lib_file" || ! -f "$tail_file" ]]; then
  echo "missing reviewed EJS 0.8.0 core, self-contained library, or Mantle adapter tail" >&2
  exit 1
fi

actual_core_sha3=$(openssl dgst -sha3-512 "$core_file" | awk '{print $NF}')
actual_lib_sha3=$(openssl dgst -sha3-512 "$lib_file" | awk '{print $NF}')
if [[ "$actual_core_sha3" != "$core_sha3" || "$actual_lib_sha3" != "$lib_sha3" ]]; then
  echo "EJS input hash does not match the reviewed 0.8.0 release" >&2
  exit 1
fi

output_dir=$(dirname "$output_file")
if [[ ! -d "$output_dir" ]]; then
  echo "output directory does not exist" >&2
  exit 1
fi

temporary_file=$(mktemp "$output_dir/.youtube-ejs-adapter.XXXXXX")
cleanup() {
  rm -f "$temporary_file"
}
trap cleanup EXIT

{
  printf '%s\n' '/* Generated from hash-pinned yt-dlp EJS 0.8.0 inputs; do not edit. */'
  sed '/^export { lib };$/d' "$lib_file"
  printf '%s\n' 'Object.assign(globalThis, lib);'
  sed '/^export { jsc };$/d' "$core_file"
  printf '\n'
  sed -n '1,$p' "$tail_file"
} > "$temporary_file"

chmod 0444 "$temporary_file"
mv -f "$temporary_file" "$output_file"
trap - EXIT
