import path from 'node:path'
import * as vscode from 'vscode'
import { server } from 'glass-easel-miniprogram-typescript'

const serviceList: TsService[] = []

export class TsService {
  private root: string
  private services: server.Server
  private waitInit: (() => void)[] | null = []

  constructor(root: string) {
    this.root = root
    this.services = new server.Server({
      projectPath: root,
      workingDirectory: root,
      verboseMessages: true,
      onDiagnosticsNeedUpdate: (_fullPath: string) => {
        // empty
      },
      onFirstScanDone: () => {
        this.waitInit?.forEach((f) => f())
        this.waitInit = null
      },
    })
  }

  static find(path: string): Promise<TsService | undefined> {
    const service = serviceList.findLast((service) => service.containsPath(path))
    if (!service) return Promise.resolve(undefined)
    if (service.waitInit) {
      const ret = new Promise<TsService>((resolve) => {
        service.waitInit?.push(() => resolve(service))
      })
      return ret
    }
    return Promise.resolve(service)
  }

  static initTsService(path: string) {
    const service = new TsService(path)
    serviceList.push(service)
    vscode.window.visibleTextEditors.forEach((editor) => {
      const uri = editor.document.uri
      if (service.containsPath(uri.fsPath)) {
        service.openFile(uri.fsPath, editor.document.getText())
      }
    })
  }

  private containsPath(p: string) {
    return !path.relative(this.root, p).startsWith('..')
  }

  openFile(fullPath: string, content: string) {
    this.services.openFile(fullPath, content)
  }

  updateFile(fullPath: string, content: string) {
    this.services.updateFile(fullPath, content)
  }

  closeFile(fullPath: string) {
    this.services.closeFile(fullPath)
  }

  getDiagnostics(fullPath: string): vscode.Diagnostic[] {
    return this.services.analyzeWxmlFile(fullPath).map((diag) => {
      const start = new vscode.Position(diag.start.line, diag.start.character)
      const end = new vscode.Position(diag.end.line, diag.end.character)
      let level = vscode.DiagnosticSeverity.Hint
      if (diag.level === server.DiagnosticLevel.Error) {
        level = vscode.DiagnosticSeverity.Error
      }
      if (diag.level === server.DiagnosticLevel.Warning) {
        level = vscode.DiagnosticSeverity.Warning
      }
      if (diag.level === server.DiagnosticLevel.Info) {
        level = vscode.DiagnosticSeverity.Information
      }
      const vscodeDiag = new vscode.Diagnostic(new vscode.Range(start, end), diag.message, level)
      return vscodeDiag
    })
  }

  getWxmlHoverContent(fullPath: string, position: vscode.Position): string | null {
    return this.services.getWxmlHoverContent(fullPath, position)
  }

  getWxmlDefinition(fullPath: string, position: vscode.Position): vscode.LocationLink[] | null {
    const ret = this.services.getWxmlDefinition(fullPath, position)
    if (!ret) return null
    return ret.map((link) => {
      const targetUri = vscode.Uri.file(link.targetUri)
      const srcStart = new vscode.Position(
        link.originSelectionRange.start.line,
        link.originSelectionRange.start.character,
      )
      const srcEnd = new vscode.Position(
        link.originSelectionRange.start.line,
        link.originSelectionRange.start.character,
      )
      const start = new vscode.Position(
        link.targetRange.start.line,
        link.targetRange.start.character,
      )
      const end = new vscode.Position(link.targetRange.end.line, link.targetRange.end.character)
      return {
        targetUri,
        targetRange: new vscode.Range(start, end),
        originSelectionRange: new vscode.Range(srcStart, srcEnd),
      }
    })
  }

  getWxmlReferences(fullPath: string, position: vscode.Position): vscode.Location[] | null {
    const ret = this.services.getWxmlReferences(fullPath, position)
    if (!ret) return null
    return ret.map((link) => {
      const targetUri = vscode.Uri.file(link.targetUri)
      const start = new vscode.Position(
        link.targetRange.start.line,
        link.targetRange.start.character,
      )
      const end = new vscode.Position(link.targetRange.end.line, link.targetRange.end.character)
      return {
        uri: targetUri,
        range: new vscode.Range(start, end),
      }
    })
  }

  getWxmlCompletion(fullPath: string, position: vscode.Position) {
    return this.services.getWxmlCompletion(fullPath, position)
  }
}
