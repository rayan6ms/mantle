import { solveMantleYoutubeCipher } from './youtube-ejs-adapter.tail.js';

function assert(condition) {
  if (!condition) {
    throw new Error('YouTube EJS adapter check failed');
  }
}

const request = {
  version: 1,
  playerScriptUrl: 'https://www.youtube.com/s/player/fixture/base.js',
  playerScript: 'fixture-player-source',
  signature: 'signature-input',
  nParameter: 'n-input',
  maxOutputBytes: 65536,
};
const response = solveMantleYoutubeCipher(request, (input) => {
  assert(input.type === 'player');
  assert(input.player === request.playerScript);
  assert(input.output_preprocessed === false);
  assert(JSON.stringify(input.requests) === JSON.stringify([
    { type: 'sig', challenges: ['signature-input'] },
    { type: 'n', challenges: ['n-input'] },
  ]));
  return {
    type: 'result',
    responses: [
      { type: 'result', data: { 'signature-input': 'signature-output' } },
      { type: 'result', data: { 'n-input': 'n-output' } },
    ],
  };
});
assert(response.version === 1);
assert(response.signature === 'signature-output');
assert(response.nParameter === 'n-output');

for (const invalid of [
  { ...request, version: 2 },
  { ...request, signature: null, nParameter: null },
  { ...request, maxOutputBytes: 65537 },
]) {
  let rejected = false;
  try {
    solveMantleYoutubeCipher(invalid, () => ({ type: 'result', responses: [] }));
  } catch {
    rejected = true;
  }
  assert(rejected);
}

let rejectedSolverError = false;
try {
  solveMantleYoutubeCipher(request, () => ({
    type: 'result',
    responses: [
      { type: 'error', error: 'secret-containing upstream diagnostic' },
      { type: 'result', data: { 'n-input': 'n-output' } },
    ],
  }));
} catch (error) {
  rejectedSolverError = true;
  assert(!String(error).includes('secret-containing'));
}
assert(rejectedSolverError);

console.log('YouTube EJS adapter protocol check passed');
