const { app, BrowserWindow, Menu, ipcMain } = require('electron')
const path = require('node:path')
const { spawn } = require('node:child_process')
const fs = require('node:fs')

let mainWindow = null
let serverProcess = null

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

function startEmbeddedServer() {
  // Start the Node.js SSR server (server-entry.js)
  const serverPath = path.join(__dirname, 'server-entry.js')
  const port = process.env.PORT || '3000'

  serverProcess = spawn(process.execPath, [serverPath], {
    env: { ...process.env, PORT: port },
    stdio: 'pipe',
  })

  serverProcess.stdout.on('data', (data) => {
    console.log(`[server] ${data.toString().trim()}`)
  })

  serverProcess.stderr.on('data', (data) => {
    console.error(`[server] ${data.toString().trim()}`)
  })
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
}

// IPC handlers
ipcMain.handle('get-version', () => app.getVersion())

app.whenReady().then(() => {
  const isDev = process.argv.includes('--dev')

  if (!isDev) {
    startEmbeddedServer()
    startEngine()
    setTimeout(() => {
      createWindow()
    }, 3000)
  } else {
    createWindow()
  }

  app.on('activate', () => {
    if (BrowserWindow.getAllWindows().length === 0) {
      createWindow()
    }
  })
})

app.on('window-all-closed', () => {
  if (serverProcess) {
    serverProcess.kill()
    serverProcess = null
  }

  if (process.platform !== 'darwin') {
    app.quit()
  }
})

app.on('before-quit', () => {
  if (serverProcess) {
    serverProcess.kill()
    serverProcess = null
  }
})
