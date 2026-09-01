#!/usr/bin/env bash
set -euo pipefail

root_dir=$(cd "$(dirname "$0")/.." && pwd)
manifest="$root_dir/reference/youtube-ejs-provider-0.8.0.json"
builder="$root_dir/scripts/build-youtube-ejs-adapter.sh"
tail_file="$root_dir/scripts/youtube-ejs-adapter.tail.js"
protocol_check="$root_dir/scripts/check-youtube-ejs-adapter.js"
source_dir="$root_dir/third_party/youtube-ejs/0.8.0"
package_dir="$root_dir/crates/mantle-media/assets/youtube-ejs/0.8.0"
packaged_adapter="$package_dir/mantle-youtube-ejs.js"
process_provider="$root_dir/crates/mantle-media/src/youtube_cipher_process.rs"

jq -e '
  .schema_version == 1 and
  .provider == "yt-dlp/ejs" and
  .version == "0.8.0" and
  .source_commit == "4fb477f4af56880cfd324c48bd4294a2d2294e50" and
  .inputs["yt.solver.core.js"].bytes == 12607 and
  .inputs["yt.solver.core.js"].sha256 == "ca259e4e3cdd37d92fc266d9af08d4fd66da8479e240f4d984f29da402c22ead" and
  .inputs["yt.solver.core.js"].sha3_512 == "c163a6f376db6ce3da47d516a28a8f2a0554ae95c58dc766f0a6e2b3894f2cef1ee07fa84beb442fa471aac4f300985added1657c7c94c4d1cfefe68920ab599" and
  .inputs["yt.solver.lib.js"].bytes == 366728 and
  .inputs["yt.solver.lib.js"].sha256 == "770831df5c46474fbff06732315b28f4fb090e427ca669a51da61e2457d41c82" and
  .inputs["yt.solver.lib.js"].sha3_512 == "1ee3753a8222fc855f5c39db30a9ccbb7967dbe1fb810e86dc9a89aa073a0907f294c720e9b65427d560a35aa1ce6af19ef854d9126a05ca00afe03f72047733" and
  .package.path == "crates/mantle-media/assets/youtube-ejs/0.8.0/mantle-youtube-ejs.js" and
  .package.bytes == 382691 and
  .package.sha256 == "1092864f066a3b243bcf7bed212a5807d06327f244693f5ab175a3b534aaeefe" and
  .package.sha3_512 == "2872a0596fdc33b49251b2aab1caf58c614a9064cac43a4aa2875fd5f9566b75757f25955b0a0e1f2b8cf60ff1a19aa034bd94bc3d5b991b966f0de8bf3c387a" and
  .package.license_path == "crates/mantle-media/assets/youtube-ejs/0.8.0/LICENSE" and
  .runtime.name == "Deno" and
  .runtime.minimum_version == "2.9.5" and
  .runtime.tested_version == "2.9.5" and
  .runtime.permissions_granted == [] and
  .runtime.permissions_denied == ["read", "write", "net", "env", "sys", "run", "ffi", "import"] and
  (.runtime.remote_modules | not) and
  (.runtime.npm_modules | not) and
  (.runtime.inherited_environment | not)
' "$manifest" >/dev/null

check_sha256() {
  local file=$1
  local expected=$2
  local actual
  actual=$(sha256sum "$file" | awk '{print $1}')
  if [[ "$actual" != "$expected" ]]; then
    echo "SHA-256 mismatch for $file" >&2
    exit 1
  fi
}

check_sha3_512() {
  local file=$1
  local expected=$2
  local actual
  actual=$(openssl dgst -sha3-512 "$file" | awk '{print $NF}')
  if [[ "$actual" != "$expected" ]]; then
    echo "SHA3-512 mismatch for $file" >&2
    exit 1
  fi
}

check_sha256 "$source_dir/yt.solver.core.js" \
  ca259e4e3cdd37d92fc266d9af08d4fd66da8479e240f4d984f29da402c22ead
check_sha256 "$source_dir/yt.solver.lib.js" \
  770831df5c46474fbff06732315b28f4fb090e427ca669a51da61e2457d41c82
check_sha3_512 "$source_dir/yt.solver.core.js" \
  c163a6f376db6ce3da47d516a28a8f2a0554ae95c58dc766f0a6e2b3894f2cef1ee07fa84beb442fa471aac4f300985added1657c7c94c4d1cfefe68920ab599
check_sha3_512 "$source_dir/yt.solver.lib.js" \
  1ee3753a8222fc855f5c39db30a9ccbb7967dbe1fb810e86dc9a89aa073a0907f294c720e9b65427d560a35aa1ce6af19ef854d9126a05ca00afe03f72047733
check_sha256 "$packaged_adapter" \
  1092864f066a3b243bcf7bed212a5807d06327f244693f5ab175a3b534aaeefe
check_sha3_512 "$packaged_adapter" \
  2872a0596fdc33b49251b2aab1caf58c614a9064cac43a4aa2875fd5f9566b75757f25955b0a0e1f2b8cf60ff1a19aa034bd94bc3d5b991b966f0de8bf3c387a
[[ $(wc -c < "$packaged_adapter") -eq 382691 ]]
cmp "$source_dir/LICENSE" "$package_dir/LICENSE"
grep -F 'This is free and unencumbered software released into the public domain.' \
  "$package_dir/LICENSE" >/dev/null
grep -F 'Name: meriyah' "$packaged_adapter" >/dev/null
grep -F 'Name: astring' "$packaged_adapter" >/dev/null

grep -F 'core_sha3=c163a6f376db6ce3da47d516a28a8f2a0554ae95c58dc766f0a6e2b3894f2cef1ee07fa84beb442fa471aac4f300985added1657c7c94c4d1cfefe68920ab599' "$builder" >/dev/null
grep -F 'lib_sha3=1ee3753a8222fc855f5c39db30a9ccbb7967dbe1fb810e86dc9a89aa073a0907f294c720e9b65427d560a35aa1ce6af19ef854d9126a05ca00afe03f72047733' "$builder" >/dev/null
grep -F "MANTLE_YOUTUBE_CIPHER_V1\\t" "$tail_file" >/dev/null
grep -F "state !== 'denied'" "$tail_file" >/dev/null
for permission in read write net env sys run ffi import; do
  grep -F "OsString::from(\"--deny-$permission\")" "$process_provider" >/dev/null
done
bash -n "$builder"

temporary_dir=$(mktemp -d)
cleanup() {
  rm -rf "$temporary_dir"
}
trap cleanup EXIT
"$builder" "$source_dir" "$temporary_dir/mantle-youtube-ejs.js"
cmp "$temporary_dir/mantle-youtube-ejs.js" "$packaged_adapter"

if command -v bun >/dev/null 2>&1; then
  bun "$protocol_check"
elif command -v node >/dev/null 2>&1; then
  node --experimental-default-type=module "$protocol_check"
else
  echo "Bun or Node is required for the pure EJS adapter protocol check" >&2
  exit 1
fi

cleanup
trap - EXIT

echo "YouTube EJS provider evidence check passed"
