import * as vscode from 'vscode'
import { Client } from './client'

let languageServer: Client | null = null

export function activate(context: vscode.ExtensionContext) {
	// start language server
	const serverPath = vscode.workspace.getConfiguration('glass-easel-analyzer').get('serverPath') as string
	const clientOptions = {
		serverPath, 
	}
	languageServer = new Client(context, clientOptions)
	languageServer.start()

	// commands
	const disposable = vscode.commands.registerCommand('glass-easel-analyzer.restart', async () => {
		if (!languageServer) return
		await languageServer.stop()
		await languageServer.start()
		vscode.window.showInformationMessage('glass-easel language server restarted')
	})
	context.subscriptions.push(disposable)
}

export function deactivate() {
	languageServer?.stop()
}
