import * as assert from 'assert'
import * as vscode from 'vscode'
import { Env } from './env'
// import * as myExtension from '../../extension'

suite('semantic tokens', function () {
  const env = new Env(this)

  test('wxml', async function () {
    await env.forEachWxmlCase(this, async (uri, expect) => {
      await vscode.commands.executeCommand('vscode.open', uri)
      const ret = await vscode.commands.executeCommand('vscode.executeDocumentSymbolProvider', uri)
      expect.snapshot(ret)
    })
  })
})
