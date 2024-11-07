import * as assert from 'assert'
import * as vscode from 'vscode'
import { forEachWxmlCase } from './env'
// import * as myExtension from '../../extension'

const NAMESPACE = __filename.endsWith('.test.ts') ? __filename.slice(0, -8) : __filename

forEachWxmlCase(NAMESPACE, 'semantic tokens', async (uri, expect) => {
	await vscode.commands.executeCommand(
		'vscode.open',
		uri,
	)
	const ret = await vscode.commands.executeCommand(
		'vscode.executeDocumentSymbolProvider',
		uri,
	)
	expect.snapshot(ret)
})
