import * as vscode from 'vscode'
import { Env } from './env'

suite('document color', function () {
  const env = new Env(this)

  test('wxml', async function () {
    await env.forEachWxmlCase(this, async (uri, expect) => {
      await vscode.commands.executeCommand('vscode.open', uri)
      const ret = await vscode.commands.executeCommand('vscode.executeDocumentColorProvider', uri)
      expect.snapshot(ret)
    })
  })

  test('wxss', async function () {
    await env.forEachWxssCase(this, async (uri, expect) => {
      await vscode.commands.executeCommand('vscode.open', uri)
      const ret = await vscode.commands.executeCommand('vscode.executeDocumentColorProvider', uri)
      expect.snapshot(ret)
    })
  })
})
