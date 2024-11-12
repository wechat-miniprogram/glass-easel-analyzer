import * as vscode from 'vscode'
import { Env } from './env'

const defCases = [
  {
    name: 'import',
    args: [new vscode.Position(0, 13), new vscode.Position(4, 20), new vscode.Position(2, 21)],
  },
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
    args: [
      new vscode.Position(1, 3),
      new vscode.Position(2, 7),
      new vscode.Position(2, 11),
      new vscode.Position(2, 15),
    ],
  },
]

suite('go to declaration', function () {
  const env = new Env(this)

  test('wxml', async function () {
    await env.wxmlCasesWith(this, defCases, async (uri, list, expect) => {
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

suite('go to definition', function () {
  const env = new Env(this)

  test('wxml', async function () {
    await env.wxmlCasesWith(this, defCases, async (uri, list, expect) => {
      await vscode.commands.executeCommand('vscode.open', uri)
      for (const position of list) {
        const ret = await vscode.commands.executeCommand(
          'vscode.executeDefinitionProvider',
          uri,
          position,
        )
        expect.snapshot(ret)
      }
    })
  })
})
