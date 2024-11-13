import * as vscode from 'vscode'
import { Client } from './client'

let languageServer: Client | null = null

export async function activate(context: vscode.ExtensionContext) {
  // start language server
  const serverPath = vscode.workspace
    .getConfiguration('glass-easel-analyzer')
    .get('serverPath') as string
  const backendConfigPath = vscode.workspace
    .getConfiguration('glass-easel-analyzer')
    .get('backendConfigurationPath') as string
  const clientOptions = {
    serverPath,
    backendConfigPath,
  }
  languageServer = new Client(context, clientOptions)
  await languageServer.start()

  // commands
  const disposable = vscode.commands.registerCommand('glass-easel-analyzer.restart', async () => {
    if (!languageServer) return
    await languageServer.stop()
    await languageServer.start()
    await vscode.window.showInformationMessage('glass-easel language server restarted')
  })
  context.subscriptions.push(disposable)
}

export async function deactivate() {
  await languageServer?.stop()
}
