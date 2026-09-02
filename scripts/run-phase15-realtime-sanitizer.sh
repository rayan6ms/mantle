#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
readonly ROOT
readonly RESULT_ROOT="${PHASE15_RESULTS_ROOT:-$ROOT/target/phase15/realtime-sanitizer}"
readonly IMAGE="localhost/mantle-phase15-rtsan:fedora44-clang22"
readonly IMAGE_ID="7f06e2cb3103e8e13f41f0e949c09a04ecbfcb5f724b76c671b7fc60641e2bff"
readonly ITERATIONS=8192

unset APPIMAGE APPDIR
mkdir -p "$RESULT_ROOT"

podman build --pull=never -t "$IMAGE" \
  -f "$ROOT/scripts/phase15-realtime-sanitizer.Containerfile" "$ROOT" \
  >"$RESULT_ROOT/container-build.log" 2>&1

actual_image_id="$(podman image inspect "$IMAGE" --format '{{.Id}}')"
if [[ "$actual_image_id" != "$IMAGE_ID" ]]; then
  printf 'Unexpected Phase 15 RTSan image ID: %s\n' "$actual_image_id" >&2
  exit 1
fi

(cd "$ROOT" && cargo build --locked --release -p mantle-audio \
  --example phase15-rtsan-probe --features phase15-rtsan-probe) \
  >"$RESULT_ROOT/rust-build.log" 2>&1

podman run --rm --network none --security-opt label=disable \
  -v "$ROOT:/workspace" -w /workspace "$IMAGE" sh -lc '
    set -euo pipefail
    clang++ -x c++ -std=c++20 -O1 -g -fno-omit-frame-pointer \
      -fsanitize=realtime -no-pie \
      scripts/phase15-realtime-sanitizer.cpp.txt \
      -x none target/release/examples/libphase15_rtsan_probe.a \
      -ldl -lpthread -lm -o target/phase15/realtime-sanitizer/probe
    clang++ --version >target/phase15/realtime-sanitizer/clang-version.log
    rpm -q compiler-rt >target/phase15/realtime-sanitizer/compiler-rt-version.log
    RTSAN_OPTIONS="color=never:halt_on_error=true:print_stats_on_exit=true" \
      timeout 30s target/phase15/realtime-sanitizer/probe positive \
      >target/phase15/realtime-sanitizer/positive.log 2>&1
    set +e
    RTSAN_OPTIONS="color=never:halt_on_error=true:print_stats_on_exit=true" \
      timeout 30s target/phase15/realtime-sanitizer/probe negative \
      >target/phase15/realtime-sanitizer/negative.log 2>&1
    negative_status=$?
    set -e
    test "$negative_status" -ne 0
    test "$negative_status" -ne 124
    printf "%s\n" "$negative_status" >target/phase15/realtime-sanitizer/negative-exit.txt
  '

grep --fixed-strings "probe_status=PASS iterations=$ITERATIONS" "$RESULT_ROOT/positive.log" >/dev/null
grep --fixed-strings 'Total error count: 0' "$RESULT_ROOT/positive.log" >/dev/null
grep --fixed-strings 'Unique error count: 0' "$RESULT_ROOT/positive.log" >/dev/null
grep --fixed-strings 'ERROR: RealtimeSanitizer: unsafe-library-call' "$RESULT_ROOT/negative.log" >/dev/null
grep --fixed-strings "unsafe function \`malloc\`" "$RESULT_ROOT/negative.log" >/dev/null
grep --fixed-strings 'Total error count: 1' "$RESULT_ROOT/negative.log" >/dev/null
grep --fixed-strings 'Unique error count: 1' "$RESULT_ROOT/negative.log" >/dev/null

checksum="$(sed -n 's/.*checksum=\([0-9][0-9]*\).*/\1/p' "$RESULT_ROOT/positive.log")"
negative_exit="$(cat "$RESULT_ROOT/negative-exit.txt")"
rust_version="$(rustc --version | awk '{print $2}')"

jq -n \
  --arg rust "$rust_version" \
  --arg image "$IMAGE" \
  --arg image_id "$actual_image_id" \
  --arg checksum "$checksum" \
  --argjson iterations "$ITERATIONS" \
  --argjson negative_exit "$negative_exit" \
  '{
    schema_version: 1,
    status: "PASS",
    slice: "phase15-realtime-sanitizer",
    plan: "compatibility/phase15-realtime-sanitizer.json",
    toolchain: {rust: $rust, clang: "22.1.8", compiler_rt: "22.1.8",
      container: $image, container_image_id: $image_id},
    hot_path: {status: "PASS", iterations: $iterations, queue_capacity: 8, packet_bytes: 3,
      checksum: $checksum, realtime_errors: 0, unique_errors: 0},
    negative_control: {status: "DETECTED", exit_code: $negative_exit,
      finding: "unsafe-library-call", unsafe_function: "malloc", realtime_errors: 1,
      unique_errors: 1},
    active_blockers: []
  }' >"$RESULT_ROOT/result.json"

printf 'Phase 15 RealtimeSanitizer probe passed with an allocating negative control.\n'
