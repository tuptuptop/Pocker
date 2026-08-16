/**
 * Terminal sessions using node-pty — a real PTY with no Python dependency.
 *
 * This replaces the previous `pty-helper.py` Python sidecar. node-pty gives
 * us a true PTY (echo, colors, resize) entirely in Node, so the terminal
 * feature no longer requires a system Python interpreter.
 */
import { randomUUID } from 'node:crypto'
import { homedir } from 'node:os'
import EventEmitter from 'node:events'
import type { IPty } from 'node-pty'
import * as pty from 'node-pty'

export type TerminalSessionEvent = {
  event: string
  payload: unknown
}

export type TerminalSession = {
  id: string
  createdAt: number
  emitter: EventEmitter
  sendInput: (data: string) => void
  resize: (cols: number, rows: number) => void
  close: () => void
}

const sessions = new Map<string, TerminalSession>()

/** Pick a sensible default login shell per platform. */
function defaultShell(): { shell: string; args: string[] } {
  if (process.platform === 'win32') {
    return { shell: 'powershell.exe', args: [] }
  }
  if (process.platform === 'darwin') {
    return { shell: '/bin/zsh', args: ['-i', '-l'] }
  }
  return { shell: process.env.SHELL || '/bin/bash', args: ['-i'] }
}

export function createTerminalSession(params: {
  command?: Array<string>
  cwd?: string
  env?: Record<string, string>
  cols?: number
  rows?: number
}): TerminalSession {
  const emitter = new EventEmitter()
  const sessionId = randomUUID()

  const home = process.env.HOME ?? homedir() ?? '/tmp'
  const cols = params.cols ?? 80
  const rows = params.rows ?? 24

  let cwd = params.cwd ?? home
  if (cwd.startsWith('~')) cwd = cwd.replace('~', home)

  // Buffer early output before any listener registers
  const earlyBuffer: Array<TerminalSessionEvent> = []
  let hasListeners = false

  emitter.on('newListener', (eventName) => {
    if (eventName === 'event' && !hasListeners) {
      hasListeners = true
      process.nextTick(() => {
        for (const evt of earlyBuffer) {
          emitter.emit('event', evt)
        }
        earlyBuffer.length = 0
      })
    }
  })

  const pushEvent = (evt: TerminalSessionEvent) => {
    if (hasListeners) {
      emitter.emit('event', evt)
    } else {
      earlyBuffer.push(evt)
    }
  }

  const { shell, args } = defaultShell()
  const requested = params.command
  const spawnFile = requested?.[0] ?? shell
  const spawnArgs = requested && requested.length > 1 ? requested.slice(1) : args

  const term: IPty = pty.spawn(spawnFile, spawnArgs, {
    name: 'xterm-256color',
    cols,
    rows,
    cwd,
    env: {
      ...process.env,
      ...params.env,
      TERM: 'xterm-256color',
      COLORTERM: 'truecolor',
    } as Record<string, string>,
  })

  term.onData((data: string) => {
    pushEvent({ event: 'data', payload: { data } })
  })

  term.onExit(({ exitCode, signal }) => {
    pushEvent({
      event: 'exit',
      payload: { exitCode, signal: signal ?? undefined },
    })
    emitter.emit('close')
    sessions.delete(sessionId)
  })

  const session: TerminalSession = {
    id: sessionId,
    createdAt: Date.now(),
    emitter,

    sendInput(data: string) {
      try {
        term.write(data)
      } catch {
        /* ignore writes after exit */
      }
    },

    resize(newCols: number, newRows: number) {
      try {
        term.resize(newCols, newRows)
      } catch {
        /* ignore resize failures */
      }
    },

    close() {
      try {
        term.kill()
      } catch {
        /* ignore */
      }
      sessions.delete(sessionId)
    },
  }

  sessions.set(sessionId, session)
  return session
}

export function getTerminalSession(id: string): TerminalSession | null {
  return sessions.get(id) ?? null
}

export function closeTerminalSession(id: string): void {
  const session = sessions.get(id)
  if (!session) return
  session.close()
}
