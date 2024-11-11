import * as vscode from 'vscode'
import { Env } from './env'

suite('go to declaration', function () {
  const env = new Env(this)

  test('wxml', async function () {
    const cases = [
      { name: 'import', args: [new vscode.Position(0, 13), new vscode.Position(4, 20)] },
      { name: 'template', args: [new vscode.Position(4, 16)] },
      {
        name: 'wxs',
        args: [
          new vscode.Position(9, 11),
          new vscode.Position(10, 11),
          new vscode.Position(11, 11),
          new vscode.Position(6, 39),
        ],
      },
      { name: 'wx-for', args: [new vscode.Position(1, 11), new vscode.Position(1, 22)] },
      {
        name: 'slot-value',
        args: [new vscode.Position(2, 7), new vscode.Position(2, 11), new vscode.Position(2, 15)],
      },
    ]
    await env.wxmlCasesWith(this, cases, async (uri, list, expect) => {
      await vscode.commands.executeCommand('vscode.open', uri)
      for (const position of list) {
        const ret = await vscode.commands.executeCommand(
          'vscode.executeDeclarationProvider',
          uri,
          position,
        )
        expect.snapshot(ret)
      }
    })
  })
})
