import path from 'node:path'
import * as vscode from 'vscode'
import { type Middleware } from 'vscode-languageclient'

const toVirtualUri = (uri: string, ext: string): vscode.Uri => {
  const vdocUriString = `glass-easel-analyzer://virtual/${encodeURIComponent(uri)}/virtual.${ext}`
  return vscode.Uri.parse(vdocUriString)
}

const fromVirtualUri = (uri: vscode.Uri): string => {
  const encodedUri = uri.path.slice(1, uri.path.lastIndexOf('/'))
  // eslint-disable-next-line @typescript-eslint/no-floating-promises
  vscode.window.showWarningMessage(encodedUri)
  const uriStr = decodeURIComponent(encodedUri)
  return uriStr
}

const virtualDocuments = Object.create(null) as { [uri: string]: string }

vscode.workspace.registerTextDocumentContentProvider('glass-easel-analyzer', {
  provideTextDocumentContent: (uri: vscode.Uri) => {
    const u = fromVirtualUri(uri)
    return virtualDocuments[u]
  },
})

vscode.languages.onDidChangeDiagnostics((ev) => {
  ev.uris.forEach((uri) => {
    // eslint-disable-next-line @typescript-eslint/no-floating-promises
    vscode.window.showErrorMessage(uri.toString())
  })
})

const getWxssDiagnosticsServerPath = () =>
  vscode.workspace
    .getConfiguration('glass-easel-analyzer')
    .get('wxssDiagnosticsServerPath') as string

const middleware: Middleware = {
  async didOpen(doc, next): Promise<void> {
    const content = doc.getText()
    const ext = path.extname(doc.fileName)
    if (ext === '.wxss') {
      const uriStr = doc.uri.toString(true)
      virtualDocuments[uriStr] = content
      const virt = toVirtualUri(uriStr, 'css')
      if (!getWxssDiagnosticsServerPath()) {
        const textDocument = await vscode.commands.executeCommand('vscode.open', virt)
        const diag = vscode.languages.getDiagnostics(virt)
      }
    }
    return next(doc)
  },
  async didChange(ev, next): Promise<void> {
    return next(ev)
  },
  async didClose(doc, next): Promise<void> {
    return next(doc)
  },
  handleDiagnostics(uri, diagnostics, next) {
    next(uri, [])
  },
}

export default middleware
