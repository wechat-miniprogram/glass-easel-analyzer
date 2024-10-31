import { ExtensionContext } from 'vscode'
import {
  Executable,
  LanguageClient,
  LanguageClientOptions,
} from 'vscode-languageclient/node'

export type ClientOptions = {
  serverPath: string,
}

export class Client {
  ctx: ExtensionContext
  options: ClientOptions
  client: LanguageClient | null = null

  constructor(ctx: ExtensionContext, options: ClientOptions) {
    this.ctx = ctx
    this.options = options
  }

  async start() {
    const command = this.options.serverPath
    const args: string[] = []
    const run: Executable = {
      command,
      args,
      options: {
        env: {
          'RUST_BACKTRACE': '1',
        },
      },
    }
    const debug: Executable = {
      command,
      args,
      options: {
        env: {
          'RUST_BACKTRACE': '1',
        },
      },
    }
    const languageClientOptions: LanguageClientOptions = {
      documentSelector: [
        { language: 'wxml', scheme: 'file' },
        { language: 'wxss', scheme: 'file' },
      ],
      outputChannelName: 'glass-easel-analyzer'
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
