import * as vscode from 'vscode'
import { Env } from './env'

suite('folding range', function () {
  const env = new Env(this)

  test('wxml', async function () {
    await env.forEachWxmlCase(this, async (uri, expect) => {
      await vscode.commands.executeCommand('vscode.open', uri)
      const ret = await vscode.commands.executeCommand('vscode.executeFoldingRangeProvider', uri)
      expect.snapshot(ret)
    })
  })
})
