import fs from 'node:fs'
import path from 'node:path'
import * as vscode from 'vscode'
import {
  type Executable,
  LanguageClient,
  type LanguageClientOptions,
} from 'vscode-languageclient/node'
import middleware from './middleware'

export type ClientOptions = {
  serverPath: string
  backendConfigPath: string
  ignorePaths: string[]
  analyzeOtherStylesheets: boolean
}

export class Client {
  options: ClientOptions
  client: LanguageClient | null = null

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

  private getBackendConfigPath(): string {
    return this.options.backendConfigPath
  }

  // eslint-disable-next-line class-methods-use-this
  private getHomeUri(): vscode.Uri {
    return vscode.workspace.workspaceFile && vscode.workspace.workspaceFile.scheme !== 'untitled'
      ? vscode.workspace.workspaceFile
      : vscode.workspace.workspaceFolders?.[0]?.uri ?? vscode.Uri.file(process.cwd())
  }

  // eslint-disable-next-line class-methods-use-this
  private resolveRelativePath(homeUri: vscode.Uri, p: string): vscode.Uri {
    const uri = path.isAbsolute(p) ? vscode.Uri.file(p) : homeUri && vscode.Uri.joinPath(homeUri, p)
    return uri
  }

  async start() {
    let backendConfig = ''
    const homeUri = this.getHomeUri()
    const backendConfigPath = this.getBackendConfigPath()
    const backendConfigUrl = backendConfigPath
      ? this.resolveRelativePath(homeUri, backendConfigPath)
      : vscode.Uri.file(`${__dirname}/web.toml`)
    if (backendConfigUrl) {
      try {
        backendConfig = new TextDecoder().decode(
          await vscode.workspace.fs.readFile(backendConfigUrl),
        )
      } catch (err) {
        // eslint-disable-next-line @typescript-eslint/no-floating-promises
        vscode.window.showErrorMessage(
          `Failed to read glass-easel backend configuration from ${backendConfigUrl.toString()}`,
        )
      }
    } else {
      // eslint-disable-next-line @typescript-eslint/no-floating-promises
      vscode.window.showErrorMessage(`Invalid glass-easel backend config path ${backendConfigPath}`)
    }
    const workspaceFolders = vscode.workspace.workspaceFolders?.map((x) => x.uri.toString()) ?? []
    const ignorePaths = this.options.ignorePaths.map((x) =>
      this.resolveRelativePath(homeUri, x).toString(),
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
    await this.client.start()
  }

  async stop() {
    await this.client?.stop()
  }
}
