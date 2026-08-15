import { URL, fileURLToPath } from 'node:url'
import { existsSync, mkdirSync, copyFileSync } from 'node:fs'
import { resolve } from 'node:path'

import { tanstackStart } from '@tanstack/react-start/plugin/vite'
import viteReact from '@vitejs/plugin-react'
import tailwindcss from '@tailwindcss/vite'
import { defineConfig, loadEnv } from 'vite'
import viteTsConfigPaths from 'vite-tsconfig-paths'

// ---------------------------------------------------------------------------
// Pocker Engine auto-start helper
// ---------------------------------------------------------------------------

/** Resolve the Pocker engine binary path.
 *  Priority:
 *  1. POCKER_ENGINE_PATH env var
 *  2. ../target/debug/pocker (Rust workspace sibling)
 *  3. ../target/release/pocker
 *  4. System pocker in PATH
 */
function resolvePockerEngine(env: Record<string, string>): string | null {
  if (env.POCKER_ENGINE_PATH?.trim()) {
    return env.POCKER_ENGINE_PATH.trim()
  }

  const workspaceRoot = resolve('..')
  const candidates = [
    resolve(workspaceRoot, 'target', 'debug', 'pocker.exe'),
    resolve(workspaceRoot, 'target', 'debug', 'pocker'),
    resolve(workspaceRoot, 'target', 'release', 'pocker.exe'),
    resolve(workspaceRoot, 'target', 'release', 'pocker'),
  ]

  for (const candidate of candidates) {
    if (existsSync(candidate)) return candidate
  }

  return null
}

const config = defineConfig(({ mode }) => {
  const env = loadEnv(mode, process.cwd(), '')
  const pockerApiUrl = env.POCKER_API_URL?.trim() || 'http://127.0.0.1:3080'

  let proxyTarget = 'http://127.0.0.1:3080'
  try {
    const parsed = new URL(pockerApiUrl)
    parsed.protocol = parsed.protocol === 'wss:' ? 'https:' : 'http:'
    parsed.pathname = ''
    proxyTarget = parsed.toString().replace(/\/$/, '')
  } catch {
    // fallback
  }

  return {
    define: {},
    resolve: {
      alias: {
        '@': fileURLToPath(new URL('./src', import.meta.url)),
      },
    },
    ssr: {
      external: [
        'better-sqlite3',
      ],
    },
    optimizeDeps: {
      exclude: [
        'better-sqlite3',
      ],
    },
    server: {
      host: '0.0.0.0',
      port: 3000,
      strictPort: false,
      allowedHosts: true,
      watch: {
        ignored: ['**/routeTree.gen.ts'],
      },
      proxy: {
        // WebSocket proxy for Pocker engine
        '/ws-pocker': {
          target: proxyTarget.replace('http', 'ws'),
          changeOrigin: false,
          ws: true,
          rewrite: (path) => path.replace(/^\/ws-pocker/, ''),
        },
        // REST API proxy for Pocker engine
        '/api/pocker': {
          target: proxyTarget,
          changeOrigin: true,
          rewrite: (path) => path.replace(/^\/api\/pocker/, ''),
        },
      },
    },
    plugins: [
      viteTsConfigPaths({
        projects: ['./tsconfig.json'],
      }),
      tailwindcss(),
      tanstackStart(),
      viteReact(),
      {
        name: 'pocker-health-check',
        configureServer(server) {
          server.middlewares.use(async (req, res, next) => {
            const requestPath = req.url?.split('?')[0]

            if (req.method === 'GET' && requestPath === '/api/healthcheck') {
              res.statusCode = 200
              res.setHeader('content-type', 'application/json')
              res.end(JSON.stringify({ ok: true, service: 'pocker-studio' }))
              return
            }

            // Check Pocker engine connection status
            if (req.method === 'GET' && requestPath === '/api/connection-status') {
              try {
                const healthRes = await fetch(`${pockerApiUrl}/api/health`, {
                  signal: AbortSignal.timeout(3000),
                })
                res.statusCode = healthRes.ok ? 200 : 502
                res.setHeader('content-type', 'application/json')
                res.end(
                  JSON.stringify({
                    ok: healthRes.ok,
                    backend: pockerApiUrl,
                    engine: resolvePockerEngine(env),
                  }),
                )
              } catch {
                res.statusCode = 502
                res.setHeader('content-type', 'application/json')
                res.end(
                  JSON.stringify({
                    ok: false,
                    backend: pockerApiUrl,
                    engine: resolvePockerEngine(env),
                  }),
                )
              }
              return
            }

            next()
          })
        },
      },
      // Client-only: replace process.env references in client bundles
      {
        name: 'client-process-env',
        enforce: 'pre',
        transform(code, _id) {
          const envName = this.environment?.name
          if (envName !== 'client') return null
          if (!code.includes('process.env') && !code.includes('process.platform')) return null

          let result = code
          result = result.replace(
            /process\.env\.POCKER_API_URL/g,
            JSON.stringify(pockerApiUrl),
          )
          result = result.replace(
            /process\.env\.HERMES_API_URL/g,
            JSON.stringify(pockerApiUrl),
          )
          result = result.replace(
            /process\.env\.NODE_ENV/g,
            JSON.stringify(mode),
          )
          result = result.replace(/process\.env/g, '{}')
          result = result.replace(/process\.platform/g, '"browser"')
          return result
        },
      },
      // Copy pty-helper.py into the server assets directory after build
      {
        name: 'copy-pty-helper',
        closeBundle() {
          const src = resolve('src/server/pty-helper.py')
          const destDir = resolve('dist/server/assets')
          const dest = resolve(destDir, 'pty-helper.py')
          if (existsSync(src)) {
            mkdirSync(destDir, { recursive: true })
            copyFileSync(src, dest)
          }
        },
      },
    ],
  }
})

export default config
