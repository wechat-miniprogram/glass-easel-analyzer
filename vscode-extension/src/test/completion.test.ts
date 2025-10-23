import * as vscode from 'vscode'
import { Env } from './env'

const defWxmlCases = [
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
    name: 'core-attribute',
    args: [new vscode.Position(4, 6), new vscode.Position(0, 55), new vscode.Position(0, 63)],
  },
  {
    name: 'import',
    args: [new vscode.Position(3, 14)],
  },
  {
    name: 'static-class',
    args: [new vscode.Position(0, 13)],
  },
  {
    name: 'static-style',
    args: [new vscode.Position(0, 13), new vscode.Position(0, 22)],
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

const defWxssCases = [
  {
    name: 'style-rule',
    args: [
      new vscode.Position(1, 7),
      new vscode.Position(4, 5),
      new vscode.Position(4, 14),
      new vscode.Position(5, 4),
      new vscode.Position(11, 1),
      new vscode.Position(13, 1),
      new vscode.Position(14, 13),
    ],
  },
  {
    name: 'media',
    args: [
      new vscode.Position(0, 9),
      new vscode.Position(6, 12),
      new vscode.Position(8, 28),
      new vscode.Position(8, 49),
      new vscode.Position(10, 18),
    ],
  },
  {
    name: 'special',
    args: [new vscode.Position(1, 4)],
  },
]

const defWxmlTsCases = [
  {
    name: 'basic',
    args: [
      new vscode.Position(1, 9),
      new vscode.Position(1, 31),
      new vscode.Position(1, 42),
      new vscode.Position(1, 46),
    ],
  },
]

suite('completion', function () {
  const env = new Env(this)

  test('wxml', async function () {
    await env.wxmlCasesWith(this, defWxmlCases, async (uri, list, expect) => {
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

  test('wxss', async function () {
    await env.wxssCasesWith(this, defWxssCases, async (uri, list, expect) => {
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

  test('wxml-ts', async function () {
    await env.wxmlTsCasesWith(this, defWxmlTsCases, async (uri, list, expect) => {
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
