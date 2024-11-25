import * as vscode from 'vscode'
import { Env } from './env'

const defCases = [
  {
    name: 'attribute',
    args: [
      new vscode.Position(0, 5),
      new vscode.Position(0, 6),
      new vscode.Position(0, 18),
      new vscode.Position(2, 1),
      new vscode.Position(2, 19),
      new vscode.Position(2, 64),
      new vscode.Position(4, 1),
      new vscode.Position(4, 9),
      new vscode.Position(4, 14),
      new vscode.Position(6, 1),
      new vscode.Position(6, 6),
      new vscode.Position(6, 11),
      new vscode.Position(8, 1),
      new vscode.Position(8, 8),
    ],
  },
  {
    name: 'import',
    args: [new vscode.Position(3, 14)],
  },
  {
    name: 'template',
    args: [new vscode.Position(4, 14)],
  },
  {
    name: 'wxs',
    args: [new vscode.Position(8, 6)],
  },
  {
    name: 'special',
    args: [new vscode.Position(1, 3), new vscode.Position(3, 4)],
  },
]

suite('completion', function () {
  const env = new Env(this)

  test('wxml', async function () {
    await env.wxmlCasesWith(this, defCases, async (uri, list, expect) => {
      await vscode.commands.executeCommand('vscode.open', uri)
      for (const position of list) {
        const ret = await vscode.commands.executeCommand(
          'vscode.executeCompletionItemProvider',
          uri,
          position,
        )
        expect.snapshot(ret)
      }
    })
  })
})
