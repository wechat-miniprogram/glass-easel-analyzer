import * as vscode from 'vscode'
import { Client } from './client'

let languageServer: Client | null = null

export async function activate(context: vscode.ExtensionContext) {
  // start language server
  languageServer = new Client(context)
  await languageServer.start()

  // commands
  const disposable = vscode.commands.registerCommand('glass-easel-analyzer.restart', async () => {
    if (!languageServer) return
    await languageServer.stop()
    languageServer = new Client(context)
    await languageServer.start()
    await vscode.window.showInformationMessage('glass-easel language server restarted')
  })
  context.subscriptions.push(disposable)
}

export async function deactivate() {
  await languageServer?.stop()
}
