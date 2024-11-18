import path from 'node:path'
import * as vscode from 'vscode'
import {
  type Executable,
  LanguageClient,
  type LanguageClientOptions,
} from 'vscode-languageclient/node'

export type ClientOptions = {
  serverPath: string
  backendConfigPath: string
  ignorePaths: string[] // TODO support ignore
}

export class Client {
  ctx: vscode.ExtensionContext
  options: ClientOptions
  client: LanguageClient | null = null

  constructor(ctx: vscode.ExtensionContext) {
    this.ctx = ctx
    const serverPath = vscode.workspace
      .getConfiguration('glass-easel-analyzer')
      .get('serverPath') as string
    const backendConfigPath = vscode.workspace
      .getConfiguration('glass-easel-analyzer')
      .get('backendConfigurationPath') as string
    const ignorePaths = vscode.workspace
      .getConfiguration('glass-easel-analyzer')
      .get('ignorePaths') as string[]
    this.options = {
      serverPath,
      backendConfigPath,
      ignorePaths,
    }
  }

  private getServerPath(): string {
    if (process.env.GLASS_EASEL_ANALYZER_SERVER) {
      return process.env.GLASS_EASEL_ANALYZER_SERVER
    }
    return this.options.serverPath
  }

  async start() {
    let backendConfig = ''
    const homeUri =
      vscode.workspace.workspaceFile && vscode.workspace.workspaceFile.scheme !== 'untitled'
        ? vscode.workspace.workspaceFile
        : vscode.workspace.workspaceFolders?.[0]?.uri
    const backendConfigUrl = path.isAbsolute(this.options.backendConfigPath)
      ? vscode.Uri.file(this.options.backendConfigPath)
      : homeUri && vscode.Uri.joinPath(homeUri, this.options.backendConfigPath)
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
      vscode.window.showErrorMessage(
        `Invalid glass-easel backend config path ${this.options.backendConfigPath}`,
      )
    }
    const workspaceFolders = vscode.workspace.workspaceFolders?.map((x) => x.uri.toString()) ?? []
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
    const languageClientOptions: LanguageClientOptions = {
      initializationOptions: {
        backendConfig,
        workspaceFolders,
      },
      documentSelector: [
        { language: 'wxml', scheme: 'file' },
        { language: 'wxss', scheme: 'file' },
      ],
      outputChannelName: 'glass-easel-analyzer',
      progressOnInitialization: true,
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
