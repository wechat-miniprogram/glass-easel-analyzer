import path from "node:path"
import fs from 'node:fs'
import * as vscode from 'vscode'
import chalk from 'chalk'
import { format as prettyFormat } from 'pretty-format'

const wxmlCases = [
  'core-attribute',
  'comment',
  'meta-tag',
  'event-binding',
  'slot-value',
  'wx-for',
  'wx-if',
  'import',
  'template',
  'wxs',
]

const EXTENSION_DIR = path.resolve(__dirname, '..', '..')
const TEST_FIXTURE_DIR = path.resolve(EXTENSION_DIR, 'test-fixture')
const SNAPSHOT_DIR = path.resolve(EXTENSION_DIR, 'test-snapshot')

const sleep = (ms: number): Promise<void> => {
	return new Promise((resolve) => {
		setTimeout(resolve, ms)
	})
}

export const forEachWxmlCase = (namespace: string, suiteName: string, f: (uri: vscode.Uri, expect: Expect) => Promise<void>) => {
  suite(suiteName, async () => {
    const suiteId = suiteName.replace(/[^a-zA-Z0-9]+/g, '-').toLowerCase()
    for (const name of wxmlCases) {
      const absPath = path.resolve(TEST_FIXTURE_DIR, 'wxml', `${name}.wxml`)
      const uri = vscode.Uri.file(absPath)
      test(`[${name}]`)
      const expect = new Expect(`${namespace}/${suiteId}/wxml/${name}`)
      await f(uri, expect)
    }
  })
}

class Expect {
  id: string
  index = 0
  snapshots: string[] = []

  constructor(id: string) {
    this.id = id
    try {
      const s = fs.readFileSync(this.snapshotPath(), { encoding: 'utf8' })
      let cur = s.indexOf('// ====== SNAPSHOT ', 0)
      while (true) {
        if (cur < 0) break
        cur = s.indexOf('\n', cur)
        if (cur < 0) break
        const start = cur + 1
        cur = s.indexOf('// ====== SNAPSHOT ', start)
        const end = cur < 0 ? s.length : cur
        const expected = s.slice(start, end)
        this.snapshots.push(expected)
      }
    } catch (_err) {
      // empty
    }
  }

  private snapshotPath() {
    return path.resolve(SNAPSHOT_DIR, this.id)
  }

  snapshot(actual: unknown) {
    const actualStr = prettyFormat(actual)
    if (this.index >= this.snapshots.length) {
      console.warn(chalk.yellow(`+ new snapshot ${this.index}`))
      console.warn(actualStr)
    } else if (this.snapshots[this.index] !== actual) {
      console.warn(chalk.green(`- expected snapshot ${this.index}`))
      console.warn(this.snapshots[this.index])
      console.warn(chalk.red(`+ actual snapshot ${this.index}`))
      console.warn(actualStr)
    }
    this.index += 1
  }
}
