import * as vscode from 'vscode'
import { Client } from './client'

let languageServer: Client | null = null

const startLanguageServer = async () => {
  const serverPath = vscode.workspace
    .getConfiguration('glass-easel-analyzer')
    .get('serverPath') as string
  const backendConfigPath = vscode.workspace
    .getConfiguration('glass-easel-analyzer')
    .get('backendConfigurationPath') as string
  const ignorePaths = vscode.workspace
    .getConfiguration('glass-easel-analyzer')
    .get('ignorePaths') as string[]
  const analyzeOtherStylesheets = vscode.workspace
    .getConfiguration('glass-easel-analyzer')
    .get('analyzeOtherStylesheets') as boolean
  const preferredTypescriptVersion = vscode.workspace
    .getConfiguration('glass-easel-analyzer')
    .get('preferredTypescriptVersion') as string
  const localTypescriptNodeModulePath = vscode.workspace
    .getConfiguration('glass-easel-analyzer')
    .get('localTypescriptNodeModulePath') as string
  const options = {
    serverPath,
    backendConfigPath,
    ignorePaths,
    analyzeOtherStylesheets,
    preferredTypescriptVersion,
    localTypescriptNodeModulePath,
  }
  languageServer = new Client(options)
  await languageServer.start()
}

const stopLanguageServer = async () => {
  await languageServer?.stop()
}

export async function activate(context: vscode.ExtensionContext) {
  // commands
  const disposable = vscode.commands.registerCommand('glass-easel-analyzer.restart', async () => {
    if (!languageServer) return
    await stopLanguageServer()
    await startLanguageServer()
    await vscode.window.showInformationMessage('glass-easel language server restarted')
  })
  context.subscriptions.push(disposable)

  // events
  const disposable2 = vscode.workspace.onDidChangeConfiguration((ev) => {
    const changed =
      ev.affectsConfiguration('glass-easel-analyzer.serverPath') ||
      ev.affectsConfiguration('glass-easel-analyzer.backendConfigurationPath') ||
      ev.affectsConfiguration('glass-easel-analyzer.ignorePaths') ||
      ev.affectsConfiguration('glass-easel-analyzer.analyzeOtherStylesheets') ||
      ev.affectsConfiguration('glass-easel-analyzer.preferredTypescriptVersion') ||
      ev.affectsConfiguration('glass-easel-analyzer.localTypescriptNodeModulePath')
    if (changed) {
      // eslint-disable-next-line promise/catch-or-return
      vscode.window
        .showWarningMessage(
          'glass-easel-analyzer needs a restart due to configuration changed',
          'Restart',
        )
        .then(async (sel) => {
          if (sel === 'Restart') {
            await stopLanguageServer()
            await startLanguageServer()
          }
          return undefined
        })
    }
  })
  context.subscriptions.push(disposable2)

  // start
  await startLanguageServer()
}

export async function deactivate() {
  await stopLanguageServer()
}
