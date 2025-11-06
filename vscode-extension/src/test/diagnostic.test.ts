import * as vscode from 'vscode'
import { Env } from './env'

suite('diagnostic', function () {
  const env = new Env(this)

  test('wxml', async function () {
    await env.forEachWxmlCase(this, async (uri, expect) => {
      await vscode.commands.executeCommand('vscode.open', uri)
      const ret = vscode.languages.getDiagnostics(uri)
      expect.snapshot(ret)
    })
  })

  test('wxss', async function () {
    await env.forEachWxssCase(this, async (uri, expect) => {
      await vscode.commands.executeCommand('vscode.open', uri)
      const ret = vscode.languages.getDiagnostics(uri)
      expect.snapshot(ret)
    })
  })

  test('wxml-ts', async function () {
    await env.forEachWxmlTsCase(this, async (uri, expect) => {
      await vscode.commands.executeCommand('vscode.open', uri)
      await new Promise((resolve) => {
        setTimeout(resolve, 200)
      })
      const ret = vscode.languages.getDiagnostics(uri)
      expect.snapshot(ret)
    })
  })
})
