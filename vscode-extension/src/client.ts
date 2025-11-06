import fs from 'node:fs'
import * as vscode from 'vscode'
import {
  type Executable,
  LanguageClient,
  type LanguageClientOptions,
} from 'vscode-languageclient/node'
import { middleware, updateInlineWxsScripts } from './middleware'
import { TsServiceHost } from './typescript'
import { resolveRelativePath } from './utils'

export type ClientOptions = {
  serverPath: string
  backendConfigPath: string
  ignorePaths: string[]
  analyzeOtherStylesheets: boolean
  preferredTypescriptVersion: string
  localTypescriptNodeModulePath: string
}

export class Client {
  private options: ClientOptions
  private client: LanguageClient | null = null
  private tsServerHost: TsServiceHost | null = null
  templateBackendConfig = ''

  constructor(options: ClientOptions) {
    this.options = options
  }

  private getServerPath(): string {
    if (process.env.GLASS_EASEL_ANALYZER_SERVER) {
      return process.env.GLASS_EASEL_ANALYZER_SERVER
    }
    if (!this.options.serverPath) {
      const detects = [`${__dirname}/glass-easel-analyzer.exe`, `${__dirname}/glass-easel-analyzer`]
      return detects.find((p) => fs.existsSync(p)) ?? 'glass-easel-analyzer'
    }
    return this.options.serverPath
  }

  getBackendConfigPath(): string {
    return this.options.backendConfigPath
  }

  getBackendConfigUrl(): vscode.Uri {
    const homeUri = this.getHomeUri()
    const backendConfigPath = this.getBackendConfigPath()
    return backendConfigPath
      ? resolveRelativePath(homeUri, backendConfigPath)
      : vscode.Uri.file(`${__dirname}/web.toml`)
  }

  private getHomeUri(): vscode.Uri {
    return vscode.workspace.workspaceFile && vscode.workspace.workspaceFile.scheme !== 'untitled'
      ? vscode.workspace.workspaceFile
      : (vscode.workspace.workspaceFolders?.[0]?.uri ?? vscode.Uri.file(process.cwd()))
  }

  // eslint-disable-next-line @typescript-eslint/no-unnecessary-type-parameters
  async customRequest<Req, Resp>(method: string, params: Req): Promise<Resp | undefined> {
    return this.client?.sendRequest(`glassEaselAnalyzer/${method}`, params)
  }

  async start() {
    let backendConfig = ''
    const homeUri = this.getHomeUri()
    const backendConfigUrl = this.getBackendConfigUrl()
    try {
      backendConfig = new TextDecoder().decode(await vscode.workspace.fs.readFile(backendConfigUrl))
      // eslint-disable-next-line @typescript-eslint/no-unused-vars
    } catch (err) {
      vscode.window.showErrorMessage(
        `Failed to read glass-easel backend configuration from ${backendConfigUrl.toString()}`,
      )
    }
    const workspaceFolders = vscode.workspace.workspaceFolders?.map((x) => x.uri.toString()) ?? []
    const ignorePaths = this.options.ignorePaths.map((x) =>
      resolveRelativePath(homeUri, x).toString(),
    )
    const command = this.getServerPath()
    const args: string[] = []
    const run: Executable = {
      command,
      args,
      options: {
        env: {
          RUST_BACKTRACE: '1',
        },
      },
    }
    const debug: Executable = {
      command,
      args,
      options: {
        env: {
          RUST_BACKTRACE: '1',
        },
      },
    }
    const stylesheetSelectors = this.options.analyzeOtherStylesheets
      ? [
          { language: 'css', scheme: 'file' },
          { language: 'css', scheme: 'untitled' },
          { language: 'less', scheme: 'file' },
          { language: 'less', scheme: 'untitled' },
          { language: 'scss', scheme: 'file' },
          { language: 'scss', scheme: 'untitled' },
        ]
      : []
    const languageClientOptions: LanguageClientOptions = {
      initializationOptions: {
        backendConfig,
        workspaceFolders,
        ignorePaths,
        enableOtherSs: this.options.analyzeOtherStylesheets,
      },
      documentSelector: [
        { language: 'wxml', scheme: 'file' },
        { language: 'wxml', scheme: 'untitled' },
        { language: 'wxss', scheme: 'file' },
        { language: 'wxss', scheme: 'untitled' },
        ...stylesheetSelectors,
      ],
      outputChannelName: 'glass-easel-analyzer',
      progressOnInitialization: true,
      middleware,
    }
    this.client = new LanguageClient(
      'glass_easel_analyzer',
      'glass-easel language server',
      { run, debug },
      languageClientOptions,
    )
    this.client.onNotification('glassEaselAnalyzer/inlineWxsScripts', (msg) => {
      updateInlineWxsScripts(msg)
    })
    this.tsServerHost = new TsServiceHost(homeUri, this, this.options)
    this.client.onNotification(
      'glassEaselAnalyzer/templateBackendConfig',
      (msg: { content: string }) => {
        this.templateBackendConfig = msg.content
      },
    )
    this.client.onNotification(
      'glassEaselAnalyzer/discoveredProject',
      (msg: { path: string; templateBackendConfig: string }) => {
        this.tsServerHost?.initTsService(msg.path)
      },
    )
    await this.client.start()
  }

  async stop() {
    this.tsServerHost?.destroy()
    this.tsServerHost = null
    await this.client?.stop()
  }
}
