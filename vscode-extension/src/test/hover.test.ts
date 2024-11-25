import * as vscode from 'vscode'
import { Env } from './env'

const defCases = [
  {
    name: 'attribute',
    args: [
      new vscode.Position(0, 1),
      new vscode.Position(0, 8),
      new vscode.Position(0, 19),
      new vscode.Position(0, 39),
      new vscode.Position(2, 2),
      new vscode.Position(2, 8),
      new vscode.Position(2, 18),
      new vscode.Position(2, 34),
      new vscode.Position(2, 52),
      new vscode.Position(2, 65),
      new vscode.Position(4, 5),
      new vscode.Position(4, 15),
      new vscode.Position(6, 1),
      new vscode.Position(6, 12),
      new vscode.Position(8, 1),
      new vscode.Position(8, 8),
    ],
  },
  { name: 'slot-value', args: [new vscode.Position(3, 9), new vscode.Position(3, 17)] },
  {
    name: 'wxs',
    args: [new vscode.Position(9, 11), new vscode.Position(10, 11), new vscode.Position(11, 11)],
  },
]

suite('hover', function () {
  const env = new Env(this)

  test('wxml', async function () {
    await env.wxmlCasesWith(this, defCases, async (uri, list, expect) => {
      await vscode.commands.executeCommand('vscode.open', uri)
      for (const position of list) {
        const ret = await vscode.commands.executeCommand(
          'vscode.executeHoverProvider',
          uri,
          position,
        )
        expect.snapshot(ret)
      }
    })
  })
})
