/*
 * SPDX-License-Identifier: MIT
 * Mantle protocol adapter appended to a reviewed, self-contained yt-dlp EJS bundle.
 */

const RESPONSE_PREFIX = 'MANTLE_YOUTUBE_CIPHER_V1\t';
const PROTOCOL_VERSION = 1;
const MAX_CHALLENGE_BYTES = 64 * 1024;
const DENO_PERMISSION_DESCRIPTORS = [
  { name: 'read' },
  { name: 'write' },
  { name: 'net' },
  { name: 'env' },
  { name: 'sys', kind: 'hostname' },
  { name: 'run' },
  { name: 'ffi' },
  { name: 'import' },
];

function fail() {
  throw new Error('invalid Mantle YouTube cipher request or solver response');
}

function challengeBytes(value) {
  if (value === undefined || value === null) {
    return null;
  }
  if (
    typeof value !== 'string' ||
    value.length === 0 ||
    new TextEncoder().encode(value).length > MAX_CHALLENGE_BYTES
  ) {
    fail();
  }
  return value;
}

function solvedValue(response, challenge) {
  if (
    !response ||
    response.type !== 'result' ||
    !response.data ||
    typeof response.data !== 'object' ||
    Array.isArray(response.data)
  ) {
    fail();
  }
  const value = response.data[challenge];
  if (typeof value !== 'string') {
    fail();
  }
  return value;
}

function solveMantleYoutubeCipher(request, solver = jsc) {
  if (
    !request ||
    typeof request !== 'object' ||
    Array.isArray(request) ||
    request.version !== PROTOCOL_VERSION ||
    typeof request.playerScriptUrl !== 'string' ||
    typeof request.playerScript !== 'string' ||
    request.playerScript.length === 0 ||
    !Number.isSafeInteger(request.maxOutputBytes) ||
    request.maxOutputBytes <= 0 ||
    request.maxOutputBytes > MAX_CHALLENGE_BYTES
  ) {
    fail();
  }

  const signature = challengeBytes(request.signature);
  const nParameter = challengeBytes(request.nParameter);
  if (signature === null && nParameter === null) {
    fail();
  }

  const requests = [];
  if (signature !== null) {
    requests.push({ type: 'sig', challenges: [signature] });
  }
  if (nParameter !== null) {
    requests.push({ type: 'n', challenges: [nParameter] });
  }
  const result = solver({
    type: 'player',
    player: request.playerScript,
    requests,
    output_preprocessed: false,
  });
  if (
    !result ||
    result.type !== 'result' ||
    !Array.isArray(result.responses) ||
    result.responses.length !== requests.length
  ) {
    fail();
  }

  let responseIndex = 0;
  const solvedSignature =
    signature === null
      ? null
      : solvedValue(result.responses[responseIndex++], signature);
  const solvedN =
    nParameter === null
      ? null
      : solvedValue(result.responses[responseIndex], nParameter);
  return {
    version: PROTOCOL_VERSION,
    signature: solvedSignature,
    nParameter: solvedN,
  };
}

async function main() {
  try {
    for (const descriptor of DENO_PERMISSION_DESCRIPTORS) {
      if ((await Deno.permissions.query(descriptor)).state !== 'denied') {
        fail();
      }
    }
    const requestText = await new Response(Deno.stdin.readable).text();
    const response = solveMantleYoutubeCipher(JSON.parse(requestText));
    console.log(`${RESPONSE_PREFIX}${JSON.stringify(response)}`);
  } catch {
    Deno.exit(1);
  }
}

if (import.meta.main) {
  await main();
}

export { solveMantleYoutubeCipher };
