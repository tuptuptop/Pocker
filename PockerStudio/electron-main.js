import { app, BrowserWindow, Menu, ipcMain } from 'electron'
import path from 'node:path'
import { spawn } from 'node:child_process'
import fs from 'node:fs'

const __dirname = import.meta.dirname

let mainWindow = null
let httpServerHandle = null

// Prevent multiple instances (avoids duplicate windows / stray processes).
if (!app.requestSingleInstanceLock()) {
  app.quit()
} else {
  app.on('second-instance', () => {
    if (mainWindow) {
      if (mainWindow.isMinimized()) mainWindow.restore()
      mainWindow.focus()
    }
  })
}

/**
 * Find the Pocker engine binary.
 * In dev: looks in ../target/debug or ../target/release
 * In production (packaged): looks in resources/engine/
 */
function findEngineBinary() {
  const isDev = !app.isPackaged

  if (isDev) {
    const candidates = [
      path.join(__dirname, '..', 'target', 'debug', 'pocker-studio.exe'),
      path.join(__dirname, '..', 'target', 'debug', 'pocker-studio'),
      path.join(__dirname, '..', 'target', 'release', 'pocker-studio.exe'),
      path.join(__dirname, '..', 'target', 'release', 'pocker-studio'),
    ]
    for (const c of candidates) {
      if (fs.existsSync(c)) return c
    }
    return null
  }

  // Packaged: look in resources/engine/
  const engineDir = path.join(process.resourcesPath, 'engine')
  if (fs.existsSync(engineDir)) {
    const files = fs.readdirSync(engineDir)
    const engineFile = files.find(f => f.startsWith('pocker-studio'))
    if (engineFile) {
      return path.join(engineDir, engineFile)
    }
  }
  return null
}

function startEngine() {
  const engineBinary = findEngineBinary()
  if (!engineBinary) {
    console.warn('[pocker-engine] Binary not found. Running in frontend-only mode.')
    return
  }

  console.log(`[pocker-engine] Starting: ${engineBinary}`)
  const engineProc = spawn(engineBinary, [], {
    stdio: 'pipe',
    env: { ...process.env },
  })

  engineProc.stdout.on('data', (data) => {
    console.log(`[engine] ${data.toString().trim()}`)
  })

  engineProc.stderr.on('data', (data) => {
    console.error(`[engine] ${data.toString().trim()}`)
  })

  engineProc.on('exit', (code) => {
    console.log(`[pocker-engine] Exited with code ${code}`)
  })

  return engineProc
}

function createWindow() {
  mainWindow = new BrowserWindow({
    width: 1400,
    height: 900,
    minWidth: 800,
    minHeight: 600,
    title: 'Pocker Studio',
    icon: path.join(__dirname, 'public', 'favicon.png'),
    webPreferences: {
      preload: path.join(__dirname, 'electron-preload.js'),
      contextIsolation: true,
      nodeIntegration: false,
    },
  })

  const isDev = process.argv.includes('--dev')

  if (isDev) {
    mainWindow.loadURL('http://localhost:3000')
    mainWindow.webContents.openDevTools()
  } else {
    const port = process.env.PORT || '3000'
    mainWindow.loadURL(`http://localhost:${port}`)
  }

  Menu.setApplicationMenu(null)

  mainWindow.on('closed', () => {
    mainWindow = null
  })

  mainWindow.webContents.on('did-fail-load', (_event, errorCode, errorDescription) => {
    console.error(`[renderer] failed to load: ${errorCode} ${errorDescription}`)
  })
}

// IPC handlers
ipcMain.handle('get-version', () => app.getVersion())

app.whenReady().then(async () => {
  const isDev = process.argv.includes('--dev')

  if (!isDev) {
    // Run the SSR server in-process (do NOT spawn a second Electron binary,
    // which previously caused duplicate windows and an unreliable server).
    process.env.PORT = process.env.PORT || '3000'
    try {
      const { startServer } = await import('./server-entry.js')
      httpServerHandle = await startServer({ port: parseInt(process.env.PORT, 10) })
    } catch (err) {
      console.error('[pocker-server] Failed to start embedded SSR server:', err)
    }
    startEngine()
  }

  createWindow()

  app.on('activate', () => {
    if (BrowserWindow.getAllWindows().length === 0) {
      createWindow()
    }
  })
})

app.on('window-all-closed', () => {
  if (httpServerHandle) {
    httpServerHandle.close()
    httpServerHandle = null
  }

  if (process.platform !== 'darwin') {
    app.quit()
  }
})

app.on('before-quit', () => {
  if (httpServerHandle) {
    httpServerHandle.close()
    httpServerHandle = null
  }
})
