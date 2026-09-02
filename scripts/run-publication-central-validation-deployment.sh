#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
readonly ROOT
deployment_id=""
result="$ROOT/target/publication-central-validation-deployment/result.json"

usage() {
  printf 'Usage: %s --deployment-id UUID [--result PATH]\n' "$0" >&2
}

while (( $# > 0 )); do
  case "$1" in
    --deployment-id) (( $# >= 2 )) || { usage; exit 2; }; deployment_id="$2"; shift 2 ;;
    --result) (( $# >= 2 )) || { usage; exit 2; }; result="$2"; shift 2 ;;
    *) usage; exit 2 ;;
  esac
done

readonly DEPLOYMENT_ID="$deployment_id"
readonly RESULT="$result"
readonly CONTRACT="$ROOT/compatibility/publication-central-validation-deployment.json"
readonly SMOKE_TEMPLATE="$ROOT/scripts/central-validation-consumer-smoke.java.txt"
readonly CENTRAL_BASE='https://central.sonatype.com'

for command in base64 curl jar java javac jq mvn sha256sum unzip; do
  command -v "$command" >/dev/null || {
    printf 'Central validation deployment requires %s\n' "$command" >&2
    exit 1
  }
done
[[ -f "$CONTRACT" && -f "$SMOKE_TEMPLATE" ]] || {
  printf 'Central validation deployment contract or smoke template is missing.\n' >&2
  exit 1
}
[[ "$DEPLOYMENT_ID" =~ ^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$ ]] || {
  printf 'A lowercase UUID deployment ID is required.\n' >&2
  exit 1
}
[[ "$DEPLOYMENT_ID" == "$(jq -r '.deployment.id' "$CONTRACT")" ]] || {
  printf 'The requested deployment ID does not match the locked validation deployment.\n' >&2
  exit 1
}
[[ -n "${CENTRAL_PORTAL_TOKEN:-}" ]] || {
  printf 'CENTRAL_PORTAL_TOKEN is required.\n' >&2
  exit 1
}
decoded_token="$(printf '%s' "$CENTRAL_PORTAL_TOKEN" | base64 --decode 2>/dev/null)" || {
  printf 'CENTRAL_PORTAL_TOKEN is not valid base64.\n' >&2
  exit 1
}
[[ "$decoded_token" == *:* ]] || {
  printf 'CENTRAL_PORTAL_TOKEN does not decode to the required Portal credential pair.\n' >&2
  exit 1
}
unset decoded_token

work_root="$(mktemp -d)"
readonly work_root
trap 'rm -rf -- "$work_root"' EXIT
mkdir -p "$work_root/downloads" "$work_root/m2" "$work_root/classes"

status_response="$work_root/status.json"
printf 'header = "Authorization: Bearer %s"\n' "$CENTRAL_PORTAL_TOKEN" |
  curl --silent --show-error --fail --request POST --config - \
    --connect-timeout 10 --max-time 30 \
    --output "$status_response" \
    "$CENTRAL_BASE/api/v1/publisher/status?id=$DEPLOYMENT_ID"
jq --exit-status --arg id "$DEPLOYMENT_ID" '
  .deploymentId == $id and .deploymentState == "VALIDATED"
' "$status_response" >/dev/null || {
  printf 'Central deployment is not the expected VALIDATED deployment.\n' >&2
  exit 1
}

download_base="$CENTRAL_BASE/api/v1/publisher/deployment/$DEPLOYMENT_ID/download"
readonly download_base
while IFS=$'\t' read -r path expected_sha; do
  destination="$work_root/downloads/$path"
  mkdir -p "$(dirname "$destination")"
  printf 'header = "Authorization: Bearer %s"\n' "$CENTRAL_PORTAL_TOKEN" |
    curl --silent --show-error --fail --location --config - \
      --connect-timeout 10 --max-time 120 \
      --output "$destination" "$download_base/$path"
  actual_sha="$(sha256sum "$destination" | awk '{print $1}')"
  [[ "$actual_sha" == "$expected_sha" ]] || {
    printf 'Deployment artifact digest mismatch: %s\n' "$path" >&2
    exit 1
  }
done < <(jq -r '.deployables[] | [.path, .sha256] | @tsv' "$CONTRACT")

settings="$work_root/settings.xml"
cat >"$settings" <<EOF
<settings xmlns="http://maven.apache.org/SETTINGS/1.0.0">
  <servers>
    <server>
      <id>central.validation</id>
      <configuration>
        <httpHeaders>
          <property>
            <name>Authorization</name>
            <value>Bearer ${CENTRAL_PORTAL_TOKEN}</value>
          </property>
        </httpHeaders>
      </configuration>
    </server>
  </servers>
</settings>
EOF
chmod 600 "$settings"

pom="$work_root/pom.xml"
cat >"$pom" <<EOF
<project xmlns="http://maven.apache.org/POM/4.0.0">
  <modelVersion>4.0.0</modelVersion>
  <groupId>io.github.rayan6ms.validation</groupId>
  <artifactId>central-validation-consumer</artifactId>
  <version>1.0.0</version>
  <repositories>
    <repository>
      <id>central.validation</id>
      <url>${download_base}</url>
    </repository>
    <repository>
      <id>central</id>
      <url>https://repo.maven.apache.org/maven2</url>
    </repository>
  </repositories>
  <dependencies>
    <dependency>
      <groupId>io.github.rayan6ms</groupId>
      <artifactId>mantle-lavaplayer</artifactId>
      <version>1.0.0</version>
    </dependency>
    <dependency>
      <groupId>io.github.rayan6ms</groupId>
      <artifactId>mantle-native</artifactId>
      <version>1.0.0</version>
      <classifier>linux-x86_64</classifier>
      <type>jar</type>
    </dependency>
  </dependencies>
</project>
EOF

mvn --batch-mode --quiet --no-transfer-progress --settings "$settings" \
  --file "$pom" -Dmaven.repo.local="$work_root/m2" \
  dependency:build-classpath -Dmdep.outputFile="$work_root/classpath.txt"
classpath="$(<"$work_root/classpath.txt")"
readonly classpath
mantle_jar="$(tr ':' '\n' <<<"$classpath" | awk '/\/mantle-lavaplayer-1\.0\.0\.jar$/ {print; exit}')"
native_jar="$(tr ':' '\n' <<<"$classpath" | awk '/\/mantle-native-1\.0\.0-linux-x86_64\.jar$/ {print; exit}')"
[[ -f "$mantle_jar" && -f "$native_jar" ]] || {
  printf 'Maven did not resolve both Mantle coordinates from the deployment.\n' >&2
  exit 1
}
[[ "$(sha256sum "$mantle_jar" | awk '{print $1}')" == \
  "$(jq -r '.deployables[] | select(.path | endswith("mantle-lavaplayer-1.0.0.jar")) | .sha256' "$CONTRACT")" ]]
[[ "$(sha256sum "$native_jar" | awk '{print $1}')" == \
  "$(jq -r '.deployables[] | select(.path | endswith("mantle-native-1.0.0-linux-x86_64.jar")) | .sha256' "$CONTRACT")" ]]

unzip -q "$native_jar" native/libmantle_jvm.so -d "$work_root/native"
native_library="$work_root/native/native/libmantle_jvm.so"
readonly native_library
[[ -f "$native_library" ]]
cp "$SMOKE_TEMPLATE" "$work_root/CentralValidationConsumerSmoke.java"
javac --release 11 -cp "$classpath" -d "$work_root/classes" \
  "$work_root/CentralValidationConsumerSmoke.java"
smoke_output="$(java --enable-native-access=ALL-UNNAMED -Xverify:all \
  -cp "$work_root/classes:$classpath" CentralValidationConsumerSmoke "$native_library")"
[[ "$smoke_output" == "central-validation-consumer-smoke-ok" ]] || {
  printf 'The deployment-scoped JVM/native consumer smoke did not pass.\n' >&2
  exit 1
}

mkdir -p "$(dirname "$RESULT")"
jq -n \
  --arg deployment_id "$DEPLOYMENT_ID" \
  --arg deployment_name "$(jq -r '.deploymentName' "$status_response")" \
  --arg download_base "$download_base" \
  --arg source_digest "$(jq -r '.source_digest' "$CONTRACT")" \
  --arg bundle_sha256 "$(jq -r '.bundle.sha256' "$CONTRACT")" \
  --arg classifier "$(jq -r '.consumer_gate.platform_classifier' "$CONTRACT")" \
  --argjson deployable_count "$(jq '.deployables | length' "$CONTRACT")" '
  {
    schema_version: 1,
    status: "PASS",
    slice: "publication-central-validation-deployment",
    source_digest: $source_digest,
    deployment: {
      id: $deployment_id,
      name: $deployment_name,
      state: "VALIDATED",
      download_base: $download_base
    },
    bundle_sha256: $bundle_sha256,
    deployables: {downloaded: $deployable_count, sha256_match: true},
    consumer: {
      maven_resolution: "PASS",
      classifier: $classifier,
      jvm_verification: "PASS",
      native_loader_smoke: "PASS"
    },
    release_policy: {
      artifact_publication_performed: false,
      publish_action_invoked: false
    },
    next_slice: "publication-explicit-release-decision"
  }' >"$RESULT"

printf 'Central validation deployment passed: all locked deployables matched and the deployment-scoped Maven/JVM/native consumer succeeded; nothing was published.\n'
