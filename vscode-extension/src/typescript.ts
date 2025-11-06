import fs from 'node:fs'
import path from 'node:path'
import * as vscode from 'vscode'
import type * as ts from 'typescript'
import { server } from 'glass-easel-miniprogram-typescript'
import { resolveRelativePath } from './utils'
import type { Client } from './client'

declare const __non_webpack_require__: NodeRequire

const serviceList: TsService[] = []

export type TsServiceOptions = {
  preferredTypescriptVersion: string
  localTypescriptNodeModulePath: string
}

export class TsServiceHost {
  private tsc?: typeof ts
  private client: Client

  constructor(homeUri: vscode.Uri, client: Client, options: TsServiceOptions) {
    this.client = client
    if (options.preferredTypescriptVersion === 'disabled') {
      this.tsc = undefined
      return
    }
    let tsPath = path.join(__dirname, 'typescript')
    if (
      options.preferredTypescriptVersion === 'local' &&
      homeUri.fsPath &&
      vscode.workspace.isTrusted
    ) {
      if (options.localTypescriptNodeModulePath) {
        tsPath = resolveRelativePath(homeUri, options.localTypescriptNodeModulePath).fsPath
      } else {
        const detect = path.join(homeUri.fsPath, 'node_modules', 'typescript')
        if (fs.existsSync(detect)) {
          tsPath = detect
        }
      }
    }
    const mainPath = path.join(tsPath, 'lib', 'typescript.js')
    try {
      this.tsc = __non_webpack_require__(mainPath) as typeof ts
      // console.info(`Using TypeScript compiler from ${tsPath}`)
    } catch (_err) {
      this.tsc = undefined
      vscode.window.showErrorMessage(
        `TypeScript-related features are disabled due to failed to load TypeScript compiler from ${tsPath}`,
      )
    }
  }

  destroy() {
    this.tsc = undefined
    serviceList.length = 0
  }

  initTsService(root: string) {
    if (!this.tsc) return
    const service = new TsService(this.tsc, this.client, root)
    serviceList.push(service)
    vscode.window.visibleTextEditors.forEach((editor) => {
      const uri = editor.document.uri
      if (path.extname(uri.fsPath) === '.wxml') {
        if (service.containsPath(uri.fsPath)) {
          service.openFile(uri.fsPath, editor.document.getText())
        }
      }
    })
  }
}

class TmplGroupProxyWithPath implements server.TmplConvertedExpr {
  private tsService: TsService
  private fullPath: string
  private _code: string

  constructor(tsService: TsService, fullPath: string, code: string) {
    this.tsService = tsService
    this.fullPath = fullPath
    this._code = code
  }

  free() {
    // eslint-disable-next-line @typescript-eslint/no-floating-promises
    this.tsService.client.customRequest('tmplConvertedExprRelease', {
      textDocumentUri: vscode.Uri.file(this.fullPath).toString(),
    })
  }

  code() {
    return this._code
  }

  async getSourceLocation(
    startLine: number,
    startCol: number,
    endLine: number,
    endCol: number,
  ): Promise<[number, number, number, number] | undefined> {
    const resp = await this.tsService.client.customRequest<any, { src: vscode.Range }>(
      'tmplConvertedExprGetSourceLocation',
      {
        textDocumentUri: vscode.Uri.file(this.fullPath).toString(),
        loc: new vscode.Range(startLine, startCol, endLine, endCol),
      },
    )
    return resp
      ? [resp.src.start.line, resp.src.start.character, resp.src.end.line, resp.src.end.character]
      : undefined
  }

  async getTokenAtSourcePosition(
    line: number,
    col: number,
  ): Promise<[number, number, number, number, number, number] | undefined> {
    const resp = await this.tsService.client.customRequest<
      any,
      { src: vscode.Range; dest: vscode.Position }
    >('tmplConvertedExprGetTokenAtSourcePosition', {
      textDocumentUri: vscode.Uri.file(this.fullPath).toString(),
      pos: new vscode.Position(line, col),
    })
    return resp
      ? [
          resp.src.start.line,
          resp.src.start.character,
          resp.src.end.line,
          resp.src.end.character,
          resp.dest.line,
          resp.dest.character,
        ]
      : undefined
  }
}

class TmplGroupProxy implements server.TmplGroup {
  private tsService: TsService

  constructor(tsService: TsService) {
    this.tsService = tsService
  }

  free() {}

  addTmpl(_path: string, _tmpl_str: string) {}

  async getTmplConvertedExpr(relPath: string, tsEnv: string): Promise<TmplGroupProxyWithPath> {
    const fullPath = path.join(this.tsService.root, ...relPath.split('/'))
    const resp = await this.tsService.client.customRequest<any, { code: string }>(
      'tmplConvertedExprCode',
      {
        textDocumentUri: vscode.Uri.file(fullPath).toString(),
        tsEnv,
      },
    )
    return new TmplGroupProxyWithPath(this.tsService, fullPath, resp?.code ?? '')
  }
}

export class TsService {
  readonly client: Client
  readonly root: string
  private services: server.Server
  private waitInit: (() => void)[] | null = []

  constructor(tsc: typeof ts, client: Client, root: string) {
    this.root = root
    this.client = client
    this.services = new server.Server({
      // eslint-disable-next-line @typescript-eslint/no-unsafe-assignment
      typescriptNodeModule: tsc as any,
      tmplGroup: new TmplGroupProxy(this),
      projectPath: '.',
      workingDirectory: root,
      templateBackendConfigPath: client.getBackendConfigUrl().fsPath + '.d.ts',
      templateBackendConfig: client.templateBackendConfig,
      verboseMessages: false,
      onDiagnosticsNeedUpdate: (fullPath: string) => {
        // eslint-disable-next-line @typescript-eslint/no-floating-promises
        this.client.customRequest('diagnosticsNeedsUpdate', {
          textDocumentUri: vscode.Uri.file(fullPath).toString(),
        })
      },
      onFirstScanDone: () => {
        this.waitInit?.forEach((f) => {
          f()
        })
        this.waitInit = null
      },
    })
  }

  static find(path: string): Promise<TsService | undefined> {
    const service = serviceList.findLast((service) => service.containsPath(path))
    if (!service) return Promise.resolve(undefined)
    if (service.waitInit) {
      const ret = new Promise<TsService>((resolve) => {
        service.waitInit?.push(() => {
          resolve(service)
        })
      })
      return ret
    }
    return Promise.resolve(service)
  }

  containsPath(p: string) {
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

  async getDiagnostics(fullPath: string): Promise<vscode.Diagnostic[]> {
    await this.services.waitPendingAsyncTasks()
    const diags = await this.services.analyzeWxmlFile(fullPath)
    return diags.map((diag) => {
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

  async getWxmlHoverContent(fullPath: string, position: vscode.Position): Promise<string | null> {
    return this.services.getWxmlHoverContent(fullPath, position)
  }

  async getWxmlDefinition(
    fullPath: string,
    position: vscode.Position,
  ): Promise<vscode.LocationLink[] | null> {
    const ret = await this.services.getWxmlDefinition(fullPath, position)
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

  async getWxmlReferences(
    fullPath: string,
    position: vscode.Position,
  ): Promise<vscode.Location[] | null> {
    const ret = await this.services.getWxmlReferences(fullPath, position)
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
