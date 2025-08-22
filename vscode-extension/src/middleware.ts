import * as vscode from 'vscode'
import path from 'node:path'
import { type Middleware } from 'vscode-languageclient'
import {
  getCSSLanguageService,
  getLESSLanguageService,
  getSCSSLanguageService,
  type LanguageService,
} from 'vscode-css-languageservice'

const cssLangService = getCSSLanguageService()
const lessLangService = getLESSLanguageService()
const scssLangService = getSCSSLanguageService()

const getWxssDiagnostics = () =>
  vscode.workspace.getConfiguration('glass-easel-analyzer').get('wxssDiagnosticsMode') as string

const doCssValidation = async (uri: vscode.Uri, ls: LanguageService) => {
  const doc = await vscode.workspace.openTextDocument(uri)
  const sheet = ls.parseStylesheet(doc as any)
  return ls.doValidation(doc as any, sheet)
}

const doFormatting = async (
  uri: vscode.Uri,
  range: vscode.Range | undefined,
  options: vscode.FormattingOptions,
  ls: LanguageService,
) => {
  const doc = await vscode.workspace.openTextDocument(uri)
  return ls.format(doc as any, range, options) as unknown as vscode.TextEdit[]
}

const selectCssLanguageService = (): LanguageService | null => {
  const mode = getWxssDiagnostics()
  if (mode === 'CSS') return cssLangService
  if (mode === 'LESS') return lessLangService
  if (mode === 'SCSS') return scssLangService
  return null
}

type InlineWxsSegs = {
  index: number
  start: vscode.Position
  end: vscode.Position
  maskedContent: string
}

type InlineWxsScript = {
  startLine: number
  startColumn: number
  endLine: number
  endColumn: number
  content: string
}

const inlineWxsSegsMap = new Map<string, InlineWxsSegs[]>()

vscode.workspace.registerTextDocumentContentProvider('glass-easel-analyzer', {
  provideTextDocumentContent(uri) {
    if (uri.authority === 'wxs' && uri.path.endsWith('.js')) {
      const p = uri.path.slice(1, -3)
      const dotPos = p.lastIndexOf('.')
      const originalUri = p.slice(0, dotPos)
      const indexStr = p.slice(dotPos + 1)
      const decodedUri = decodeURIComponent(originalUri || '')
      const index = Number(indexStr) || 0
      const seg = inlineWxsSegsMap.get(decodedUri)?.[index]
      return seg?.maskedContent || null
    }
    return null
  },
})

const generateInlineWxsUri = (uri: vscode.Uri, index: number) =>
  vscode.Uri.parse(`glass-easel-analyzer://wxs/${encodeURIComponent(uri.toString())}.${index}.js`)

export const updateInlineWxsScripts = (info: { uri: string; list: InlineWxsScript[] }) => {
  const segs = info.list.map((item, index) => {
    const start = new vscode.Position(item.startLine, item.startColumn)
    const end = new vscode.Position(item.endLine, item.endColumn)
    const maskedContent = '\n'.repeat(start.line) + ' '.repeat(start.character) + item.content
    return { index, start, end, maskedContent }
  })
  inlineWxsSegsMap.set(info.uri, segs)
}

const searchInlineWxsScript = (uri: vscode.Uri, position: vscode.Position) => {
  const segs = inlineWxsSegsMap.get(uri.toString())
  if (!segs) return null
  for (const seg of segs) {
    if (position.isAfterOrEqual(seg.start) && position.isBeforeOrEqual(seg.end)) {
      return seg
    }
  }
  return null
}

const forEachInlineWxsScript = async (
  uri: vscode.Uri,
  f: (seg: InlineWxsSegs) => Promise<void>,
) => {
  const segs = inlineWxsSegsMap.get(uri.toString())
  if (!segs) return null
  for (const seg of segs) {
    await f(seg)
  }
  return null
}

export const middleware: Middleware = {
  async didClose(document, next) {
    inlineWxsSegsMap.delete(document.uri.toString())
    await next(document)
  },

  handleDiagnostics(uri, diagnostics, next) {
    if (path.extname(uri.path) === '.wxss') {
      const ls = selectCssLanguageService()
      if (ls) {
        // eslint-disable-next-line @typescript-eslint/no-floating-promises
        doCssValidation(uri, ls)
          // eslint-disable-next-line promise/no-callback-in-promise
          .then((diag) => next(uri, diag as any))
          .catch(() => {
            // eslint-disable-next-line @typescript-eslint/no-floating-promises
            vscode.window.showErrorMessage('Failed to get CSS diagnostics')
          })
      } else {
        next(uri, diagnostics)
      }
      return
    }
    next(uri, diagnostics)
  },

  provideDocumentFormattingEdits(document, options, token, next) {
    if (path.extname(document.uri.path) === '.wxss') {
      const ls = selectCssLanguageService()
      if (ls) {
        // eslint-disable-next-line @typescript-eslint/no-floating-promises
        const ret = doFormatting(document.uri, undefined, options, ls)
          // eslint-disable-next-line promise/no-callback-in-promise
          .catch(() => {
            // eslint-disable-next-line @typescript-eslint/no-floating-promises
            vscode.window.showErrorMessage('Failed to format CSS')
            return []
          })
        return ret
      }
    }
    return next(document, options, token)
  },

  async provideHover(document, position, token, next) {
    if (document.languageId === 'wxml') {
      const script = searchInlineWxsScript(document.uri, position)
      if (script) {
        const ret = await vscode.commands.executeCommand(
          'vscode.exec',
          generateInlineWxsUri(document.uri, script.index),
          position,
        )
        // eslint-disable-next-line @typescript-eslint/no-unsafe-return
        return (ret as any[])?.[0]
      }
    }
    const ret = await next(document, position, token)
    return ret
  },

  async provideCompletionItem(document, position, context, token, next) {
    if (document.languageId === 'wxml') {
      const script = searchInlineWxsScript(document.uri, position)
      if (script) {
        const ret = await vscode.commands.executeCommand(
          'vscode.executeCompletionItemProvider',
          generateInlineWxsUri(document.uri, script.index),
          position,
          context.triggerCharacter,
        )
        // eslint-disable-next-line @typescript-eslint/no-unsafe-return
        return ret as any
      }
    }
    const ret = await next(document, position, context, token)
    return ret
  },
}

export default middleware
