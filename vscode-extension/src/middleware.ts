import * as vscode from 'vscode'
import path from 'node:path'
import { type Middleware } from 'vscode-languageclient'
import { getCSSLanguageService } from 'vscode-css-languageservice'

getCSSLanguageService()

const getWxssDiagnostics = () =>
  vscode.workspace.getConfiguration('glass-easel-analyzer').get('wxssDiagnosticsMode') as string

const cssLangService = getCSSLanguageService()

const doCssValidation = async (uri: vscode.Uri) => {
  const doc = await vscode.workspace.openTextDocument(uri)
  const sheet = cssLangService.parseStylesheet(doc as any)
  return cssLangService.doValidation(doc as any, sheet)
}

const middleware: Middleware = {
  handleDiagnostics(uri, diagnostics, next) {
    if (path.extname(uri.path) === '.wxss') {
      const mode = getWxssDiagnostics()
      if (mode === 'disabled') {
        next(uri, diagnostics)
      } else {
        // eslint-disable-next-line @typescript-eslint/no-floating-promises
        doCssValidation(uri)
          // eslint-disable-next-line promise/no-callback-in-promise
          .then((diag) => next(uri, diag as any))
          .catch(() => {
            // eslint-disable-next-line @typescript-eslint/no-floating-promises
            vscode.window.showErrorMessage('Failed to get CSS diagnostics')
          })
      }
      return
    }
    next(uri, diagnostics)
  },
}

export default middleware
