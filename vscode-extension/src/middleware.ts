import * as vscode from 'vscode'
import path from 'node:path'
import { type Middleware } from 'vscode-languageclient'
import {
  getCSSLanguageService,
  getLESSLanguageService,
  getSCSSLanguageService,
  type LanguageService,
} from 'vscode-css-languageservice'

const MANAGED_URI_SCHEME = 'glass-easel-analyzer'

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

vscode.workspace.registerTextDocumentContentProvider(MANAGED_URI_SCHEME, {
  provideTextDocumentContent(uri) {
    const parsedUri = parseInlineWxsUri(uri)
    if (parsedUri) {
      const [decodedUri, index] = parsedUri
      const seg = inlineWxsSegsMap.get(decodedUri.toString())?.[index]
      return seg?.maskedContent || null
    }
    return null
  },
})

const generateInlineWxsUri = (uri: vscode.Uri, index: number) =>
  vscode.Uri.parse(`${MANAGED_URI_SCHEME}://wxs/${encodeURIComponent(uri.toString())}.${index}.js`)

const parseInlineWxsUri = (uri: vscode.Uri): [vscode.Uri, number] | null => {
  if (uri.authority === 'wxs' && uri.path.endsWith('.js')) {
    const p = uri.path.slice(1, -3)
    const dotPos = p.lastIndexOf('.')
    const originalUri = p.slice(0, dotPos)
    const indexStr = p.slice(dotPos + 1)
    const decodedUri = decodeURIComponent(originalUri || '')
    const index = Number(indexStr) || 0
    return [vscode.Uri.parse(decodedUri), index]
  }
  return null
}

export const updateInlineWxsScripts = (info: { uri: string; list: InlineWxsScript[] }) => {
  const segs = info.list.map((item, index) => {
    const start = new vscode.Position(item.startLine, item.startColumn)
    const end = new vscode.Position(item.endLine, item.endColumn)
    const maskedContent = '\n'.repeat(start.line) + ' '.repeat(start.character) + item.content
    // eslint-disable-next-line @typescript-eslint/no-floating-promises
    vscode.workspace.openTextDocument(generateInlineWxsUri(vscode.Uri.parse(info.uri), index))
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
        const ret = await vscode.commands.executeCommand<vscode.Hover[]>(
          'vscode.executeHoverProvider',
          generateInlineWxsUri(document.uri, script.index),
          position,
        )
        const item = ret?.[0]
        return item
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

  async provideDefinition(document, position, token, next) {
    if (document.languageId === 'wxml') {
      const script = searchInlineWxsScript(document.uri, position)
      if (script) {
        const ret = await vscode.commands.executeCommand<vscode.Declaration>(
          'vscode.executeDefinitionProvider',
          generateInlineWxsUri(document.uri, script.index),
          position,
        )
        const locations = Array.isArray(ret) ? ret : [ret]
        locations.forEach((loc) => {
          if ('uri' in loc && loc.uri.scheme === MANAGED_URI_SCHEME) {
            loc.uri = parseInlineWxsUri(loc.uri)?.[0] ?? loc.uri
          }
          if ('targetUri' in loc && loc.targetUri.scheme === MANAGED_URI_SCHEME) {
            loc.targetUri = parseInlineWxsUri(loc.targetUri)?.[0] ?? loc.targetUri
          }
        })
        return locations
      }
    }
    const ret = await next(document, position, token)
    return ret
  },

  async provideDeclaration(document, position, token, next) {
    if (document.languageId === 'wxml') {
      const script = searchInlineWxsScript(document.uri, position)
      if (script) {
        const ret = await vscode.commands.executeCommand<vscode.Declaration>(
          'vscode.executeDeclarationProvider',
          generateInlineWxsUri(document.uri, script.index),
          position,
        )
        const locations = Array.isArray(ret) ? ret : [ret]
        locations.forEach((loc) => {
          if ('uri' in loc && loc.uri.scheme === MANAGED_URI_SCHEME) {
            loc.uri = parseInlineWxsUri(loc.uri)?.[0] ?? loc.uri
          }
          if ('targetUri' in loc && loc.targetUri.scheme === MANAGED_URI_SCHEME) {
            loc.targetUri = parseInlineWxsUri(loc.targetUri)?.[0] ?? loc.targetUri
          }
        })
        return locations
      }
    }
    const ret = await next(document, position, token)
    return ret
  },

  async provideReferences(document, position, options, token, next) {
    if (document.languageId === 'wxml') {
      const script = searchInlineWxsScript(document.uri, position)
      if (script) {
        const locations = await vscode.commands.executeCommand<vscode.Location[]>(
          'vscode.executeReferenceProvider',
          generateInlineWxsUri(document.uri, script.index),
          position,
        )
        locations.forEach((loc) => {
          if (loc.uri.scheme === MANAGED_URI_SCHEME) {
            loc.uri = parseInlineWxsUri(loc.uri)?.[0] ?? loc.uri
          }
        })
        return locations
      }
    }
    const ret = await next(document, position, options, token)
    return ret
  },
}

export default middleware
