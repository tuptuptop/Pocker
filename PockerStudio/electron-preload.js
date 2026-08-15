import { contextBridge, ipcRenderer } from 'electron'

contextBridge.exposeInMainWorld('pockerAPI', {
  getVersion: () => ipcRenderer.invoke('get-version'),
  platform: process.platform,
})
