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

  constructor(root: string) {
    this.root = root
    this.services = new server.Server({
      projectPath: root,
      onDiagnosticsNeedUpdate: (_fullPath: string) => {
        // TODO
      },
    })
  }

  static find(path: string): TsService | undefined {
    return serviceList.findLast((service) => service.containsPath(path))
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
