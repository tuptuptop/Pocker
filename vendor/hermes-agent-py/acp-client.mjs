// Minimal ACP (Agent Client Protocol) stdio client for Hermes Agent.
// Spawns `hermes-acp` as a sidecar and performs the initialize handshake
// over JSON-RPC on stdin/stdout (logs go to stderr on the agent side).
//
// Usage:
//   node acp-client.mjs                 # just handshake
//   PROMPT="hello" node acp-client.mjs  # handshake + send one prompt
//
// This is the reference client Pocker will use to drive the Python Hermes
// sidecar. It intentionally keeps zero deps so it runs on plain Node 22.

import { spawn } from 'node:child_process';
import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { dirname, join } from 'node:path';

const __dirname = dirname(fileURLToPath(import.meta.url));

// Resolve the hermes-acp executable inside the isolated venv.
const acpExe = join(__dirname, '.venv', 'Scripts', 'hermes-acp.exe');

const child = spawn(acpExe, ['--check'] === process.argv && false ? ['--check'] : [], {
  stdio: ['pipe', 'pipe', 'pipe'],
  windowsHide: true,
});

let buf = '';
let nextId = 1;
const pending = new Map();

function send(method, params) {
  const id = nextId++;
  const msg = { jsonrpc: '2.0', id, method, params: params ?? {} };
  child.stdin.write(JSON.stringify(msg) + '\n');
  return new Promise((resolve, reject) => {
    pending.set(id, { resolve, reject });
  });
}

child.stdout.on('data', (chunk) => {
  buf += chunk.toString();
  let idx;
  while ((idx = buf.indexOf('\n')) >= 0) {
    const line = buf.slice(0, idx).trim();
    buf = buf.slice(idx + 1);
    if (!line) continue;
    let rpc;
    try {
      rpc = JSON.parse(line);
    } catch {
      continue;
    }
    if (rpc.id !== undefined && pending.has(rpc.id)) {
      const { resolve, reject } = pending.get(rpc.id);
      pending.delete(rpc.id);
      if (rpc.error) reject(new Error(JSON.stringify(rpc.error)));
      else resolve(rpc.result);
    }
  }
});

child.stderr.on('data', (chunk) => {
  // Agent logs to stderr; surface but don't treat as protocol traffic.
  process.stderr.write('[hermes] ' + chunk.toString());
});

child.on('exit', (code) => {
  console.error(`[hermes] process exited with code ${code}`);
});

const timeout = (p, ms, label) =>
  Promise.race([
    p,
    new Promise((_, rej) => setTimeout(() => rej(new Error(`timeout: ${label}`)), ms)),
  ]);

async function main() {
  try {
    // 1) initialize handshake — required first call for ACP.
    const init = await timeout(
      send('initialize', {
        protocolVersion: 1,
        capabilities: {},
        clientInfo: { name: 'pocker-sidecar-test', version: '0.0.1' },
      }),
      120000,
      'initialize'
    );
    console.log('INIT_OK', JSON.stringify(init).slice(0, 200));

    // 2) optional: notify initialized
    child.stdin.write(
      JSON.stringify({ jsonrpc: '2.0', method: 'notifications/initialized' }) + '\n'
    );

    const prompt = process.env.PROMPT;
    if (prompt) {
      const resp = await timeout(
        send('prompt', { prompt, context: [] }),
        30000,
        'prompt'
      );
      console.log('PROMPT_RESP', JSON.stringify(resp).slice(0, 500));
    } else {
      console.log('NO_PROMPT: handshake verified; set PROMPT=... to test model call');
    }
    // Give the agent a moment to flush, then close.
    setTimeout(() => child.kill('SIGTERM'), 800);
  } catch (err) {
    console.error('CLIENT_ERR', err.message);
    child.kill('SIGTERM');
    process.exit(1);
  }
}

main();
