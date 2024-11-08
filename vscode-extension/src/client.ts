import { type ExtensionContext } from 'vscode'
import {
  type Executable,
  LanguageClient,
  type LanguageClientOptions,
} from 'vscode-languageclient/node'

export type ClientOptions = {
  serverPath: string
}

export class Client {
  ctx: ExtensionContext
  options: ClientOptions
  client: LanguageClient | null = null

  constructor(ctx: ExtensionContext, options: ClientOptions) {
    this.ctx = ctx
    this.options = options
  }

  private getServerPath(): string {
    if (process.env.GLASS_EASEL_ANALYZER_SERVER) {
      return process.env.GLASS_EASEL_ANALYZER_SERVER
    }
    return this.options.serverPath
  }

  async start() {
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
      documentSelector: [
        { language: 'wxml', scheme: 'file' },
        { language: 'wxss', scheme: 'file' },
      ],
      outputChannelName: 'glass-easel-analyzer',
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
