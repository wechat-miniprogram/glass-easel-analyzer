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

const middleware: Middleware = {
  handleDiagnostics(uri, diagnostics, next) {
    if (path.extname(uri.path) === '.wxss') {
      const mode = getWxssDiagnostics()
      if (mode === 'CSS' || mode === 'LESS' || mode === 'SCSS') {
        let ls: LanguageService = cssLangService
        if (mode === 'LESS') ls = lessLangService
        else if (mode === 'SCSS') ls = scssLangService
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
}

export default middleware
