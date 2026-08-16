import { spawn } from 'node:child_process'
import { existsSync } from 'node:fs'
import { dirname, resolve } from 'node:path'
import { homedir } from 'node:os'

const HERMES_HEALTH_TIMEOUT_MS = 2_000
const HERMES_START_PORT = 8642

let startPromise: Promise<StartHermesAgentResult> | null = null

export type StartHermesAgentResult =
  | {
      ok: true
      message: string
      pid?: number
    }
  | {
      ok: false
      error: string
    }

/**
 * Resolve the Pocker agent backend binary.
 *
 * The agent backend is now the Rust-native `pocker-agent` (or `pocker agent
 * serve` subcommand) — the Python Hermes sidecar has been removed. The binary
 * is located via `POCKER_AGENT_BIN`, then a sibling `pocker-agent` dir, then
 * the system `pocker` CLI on PATH.
 */
export function resolveHermesAgentBin(): string | null {
  if (process.env.POCKER_AGENT_BIN?.trim()) {
    return process.env.POCKER_AGENT_BIN.trim()
  }

  const workspaceRoot = dirname(resolve('.'))
  const candidates = [
    resolve(workspaceRoot, 'pocker-agent', 'target', 'release', 'pocker-agent'),
    resolve(workspaceRoot, 'pocker-agent', 'target', 'debug', 'pocker-agent'),
    resolve(workspaceRoot, '..', 'pocker-agent', 'target', 'release', 'pocker-agent'),
    'pocker',
  ]
  for (const c of candidates) {
    if (existsSync(c)) return c
  }
  return 'pocker'
}

export async function isHermesAgentHealthy(
  port = HERMES_START_PORT,
): Promise<boolean> {
  try {
    const response = await fetch(`http://127.0.0.1:${port}/health`, {
      signal: AbortSignal.timeout(HERMES_HEALTH_TIMEOUT_MS),
    })
    return response.ok
  } catch {
    return false
  }
}

async function sleep(ms: number): Promise<void> {
  return new Promise((r) => setTimeout(r, ms))
}

export async function startHermesAgent(): Promise<StartHermesAgentResult> {
  if (await isHermesAgentHealthy()) {
    return { ok: true, message: 'already running' }
  }

  if (startPromise) {
    return startPromise
  }

  startPromise = (async () => {
    try {
      const bin = resolveHermesAgentBin()
      const child = spawn(
        bin,
        ['agent', 'serve', '--port', String(HERMES_START_PORT)],
        {
          cwd: process.cwd(),
          detached: true,
          stdio: 'ignore',
          env: process.env,
        },
      )
      child.unref()

      for (let attempt = 0; attempt < 15; attempt += 1) {
        await sleep(1_000)
        if (await isHermesAgentHealthy()) {
          return {
            ok: true,
            pid: child.pid,
            message: 'started',
          }
        }
      }

      return {
        ok: true,
        pid: child.pid,
        message: 'starting',
      }
    } catch (error) {
      return {
        ok: false,
        error: error instanceof Error ? error.message : String(error),
      }
    }
  })()

  try {
    return await startPromise
  } finally {
    startPromise = null
  }
}
