import * as vscode from 'vscode'
import { Env } from './env'

const defCases = [
  {
    name: 'core-attribute',
    args: [new vscode.Position(4, 3)],
  },
  {
    name: 'import',
    args: [new vscode.Position(0, 13), new vscode.Position(5, 20), new vscode.Position(3, 21)],
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
      new vscode.Position(3, 9),
      new vscode.Position(3, 13),
      new vscode.Position(3, 17),
      new vscode.Position(2, 16),
      new vscode.Position(2, 23),
      new vscode.Position(2, 26),
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

suite('find references', function () {
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
