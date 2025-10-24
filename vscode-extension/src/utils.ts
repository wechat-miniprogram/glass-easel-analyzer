import path from 'node:path'
import * as vscode from 'vscode'

export const resolveRelativePath = (homeUri: vscode.Uri, p: string): vscode.Uri => {
  const uri = path.isAbsolute(p) ? vscode.Uri.file(p) : vscode.Uri.joinPath(homeUri, p)
  return uri
}
