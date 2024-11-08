import * as vscode from 'vscode'
import { Env } from './env'

suite('semantic tokens', function () {
  const env = new Env(this)

  test('wxml (full)', async function () {
    await env.forEachWxmlCase(this, async (uri, expect) => {
      await vscode.commands.executeCommand('vscode.open', uri)
      const ret = await vscode.commands.executeCommand('vscode.provideDocumentSemanticTokens', uri)
      expect.snapshot(ret)
    })
  })

  test('wxml (range)', async function () {
    await env.forEachWxmlCase(this, async (uri, expect) => {
      await vscode.commands.executeCommand('vscode.open', uri)
      const ret = await vscode.commands.executeCommand(
        'vscode.provideDocumentRangeSemanticTokens',
        uri,
        new vscode.Range(new vscode.Position(1, 2), new vscode.Position(2, 1)),
      )
      expect.snapshot(ret)
    })
  })
})
