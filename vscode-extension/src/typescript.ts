import path from 'node:path'
import { server } from 'glass-easel-miniprogram-typescript'

const serviceList: TsService[] = []

export const initTsService = (path: string) => {
  const service = new TsService(path)
  serviceList.push(service)
}

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
        // TODO
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

  getDiagnostics(fullPath: string) {
    return this.services.analyzeWxmlFile(fullPath)
  }
}
