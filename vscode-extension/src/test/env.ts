import path from 'node:path'
import fs from 'node:fs'
import * as vscode from 'vscode'
import * as diff from 'diff'
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
const OVERWRITE_SNAPSHOT = process.env.TEST_OVERWRITE_SNAPSHOT

const normalizeTitle = (title: string) =>
  title
    .match(/[a-zA-Z0-9]+/g)!
    .map((x) => x.toLowerCase())
    .join('-')

export class Env {
  private namespace: string

  constructor(suite: Mocha.Suite) {
    this.namespace = normalizeTitle(suite.title)
  }

  // eslint-disable-next-line class-methods-use-this
  async wrapExpect(id: string, f: (expect: Expect) => Promise<void>): Promise<number> {
    const expect = new Expect(id)
    // eslint-disable-next-line no-await-in-loop
    await f(expect)
    if (expect.failureCount > 0) {
      if (OVERWRITE_SNAPSHOT) {
        expect.overwriteExpected()
        // eslint-disable-next-line no-console
        console.warn(
          chalk.yellow(`${expect.failureCount} snapshot(s) updated ${expect.snapshotPath()}`),
        )
      } else {
        expect.writeActualAndDiff()
        // eslint-disable-next-line no-console
        console.error(
          chalk.red(
            `${expect.failureCount} snapshot(s) miss match at ${expect.actualOutputPath()}`,
          ),
        )
      }
    } else {
      expect.writeActualAndDiff()
    }
    return expect.failureCount
  }

  async forEachWxmlCase(ctx: Mocha.Context, f: (uri: vscode.Uri, expect: Expect) => Promise<void>) {
    const testName = ctx.test?.title || '(unnamed test)'
    const testId = normalizeTitle(testName)
    let snapshotFails = 0
    for (const name of wxmlCases) {
      const absPath = path.resolve(TEST_FIXTURE_DIR, 'wxml', `${name}.wxml`)
      const uri = vscode.Uri.file(absPath)
      snapshotFails += await this.wrapExpect(
        `${this.namespace}/${testId}/${name}`,
        async (expect) => {
          await f(uri, expect)
        },
      )
    }
    if (snapshotFails > 0) {
      throw new Error(`several snapshot(s) miss match`)
    }
  }

  async wxmlCasesWith<T>(
    ctx: Mocha.Context,
    cases: { name: string; args: T }[],
    f: (uri: vscode.Uri, args: T, expect: Expect) => Promise<void>,
  ) {
    const testName = ctx.test?.title || '(unnamed test)'
    const testId = normalizeTitle(testName)
    let snapshotFails = 0
    // eslint-disable-next-line no-restricted-syntax
    for (const { name, args } of cases) {
      const absPath = path.resolve(TEST_FIXTURE_DIR, 'wxml', `${name}.wxml`)
      const uri = vscode.Uri.file(absPath)
      // eslint-disable-next-line no-await-in-loop
      snapshotFails += await this.wrapExpect(
        `${this.namespace}/${testId}/${name}`,
        async (expect) => {
          await f(uri, args, expect)
        },
      )
    }
    if (snapshotFails > 0) {
      throw new Error(`several snapshot(s) miss match`)
    }
  }
}

class Expect {
  private id: string
  private index = 0
  private snapshots: string[] = []
  private expectedStr = ''
  private actualOutput = ''
  failureCount = 0

  constructor(id: string) {
    this.id = id
    try {
      const s = fs.readFileSync(this.snapshotPath(), { encoding: 'utf8' })
      this.expectedStr = s
      let cur = s.indexOf('// ====== SNAPSHOT ', 0)
      // eslint-disable-next-line no-constant-condition
      while (true) {
        if (cur < 0) break
        cur = s.indexOf('\n', cur)
        if (cur < 0) break
        const start = cur + 1
        cur = s.indexOf('// ====== SNAPSHOT ', start)
        const end = cur < 0 ? s.length : cur
        const expected = s.slice(start, end - 1)
        this.snapshots.push(expected)
      }
    } catch (_err) {
      fs.mkdirSync(this.snapshotDir(), { recursive: true })
    }
  }

  private snapshotDir() {
    const dirName = this.id.slice(0, this.id.lastIndexOf('/'))
    return path.resolve(SNAPSHOT_DIR, dirName)
  }

  snapshotPath() {
    return path.resolve(SNAPSHOT_DIR, `${this.id}.expected`)
  }

  actualOutputPath() {
    return path.resolve(SNAPSHOT_DIR, `${this.id}.actual`)
  }

  diffOutputPath() {
    return path.resolve(SNAPSHOT_DIR, `${this.id}.diff`)
  }

  writeActualAndDiff() {
    if (this.failureCount > 0) {
      fs.writeFileSync(this.actualOutputPath(), this.actualOutput)
      const patch = diff.createPatch(this.snapshotPath(), this.expectedStr, this.actualOutput)
      fs.writeFileSync(this.diffOutputPath(), patch)
    } else {
      try {
        fs.unlinkSync(this.actualOutputPath())
        fs.unlinkSync(this.diffOutputPath())
      } catch (_err) {
        // empty
      }
    }
  }

  overwriteExpected() {
    fs.writeFileSync(this.snapshotPath(), this.actualOutput)
    try {
      fs.unlinkSync(this.actualOutputPath())
      fs.unlinkSync(this.diffOutputPath())
    } catch (_err) {
      // empty
    }
  }

  snapshot(actual: unknown) {
    const actualStr = prettyFormat(actual, { printFunctionName: false })
    this.actualOutput += `// ====== SNAPSHOT ${this.index} ======\n`
    this.actualOutput += actualStr
    this.actualOutput += '\n'
    if (this.index >= this.snapshots.length) {
      this.failureCount += 1
    } else if (this.snapshots[this.index] !== actualStr) {
      this.failureCount += 1
    }
    this.index += 1
  }
}
