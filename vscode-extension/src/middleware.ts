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

const middleware: Middleware = {
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
}

export default middleware
