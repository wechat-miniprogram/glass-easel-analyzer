import * as vscode from 'vscode'
import { Env } from './env'

const defWxmlCases = [
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
  { name: 'static-style', args: [new vscode.Position(0, 15)] },
  { name: 'let-var', args: [new vscode.Position(1, 5)] },
  { name: 'slot-value', args: [new vscode.Position(3, 9), new vscode.Position(3, 17)] },
  {
    name: 'wxs',
    args: [new vscode.Position(9, 11), new vscode.Position(10, 11), new vscode.Position(11, 11)],
  },
]

const defWxssCases = [
  {
    name: 'style-rule',
    args: [
      new vscode.Position(0, 1),
      new vscode.Position(0, 4),
      new vscode.Position(0, 6),
      new vscode.Position(1, 7),
      new vscode.Position(4, 3),
      new vscode.Position(4, 12),
      new vscode.Position(5, 4),
    ],
  },
  {
    name: 'media',
    args: [
      new vscode.Position(0, 9),
      new vscode.Position(2, 5),
      new vscode.Position(6, 12),
      new vscode.Position(8, 28),
      new vscode.Position(8, 49),
      new vscode.Position(10, 18),
    ],
  },
]

suite('hover', function () {
  const env = new Env(this)

  test('wxml', async function () {
    await env.wxmlCasesWith(this, defWxmlCases, async (uri, list, expect) => {
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

  test('wxss', async function () {
    await env.wxssCasesWith(this, defWxssCases, async (uri, list, expect) => {
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
