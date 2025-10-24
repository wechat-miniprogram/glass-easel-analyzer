import path from 'node:path'
import fs from 'node:fs'
import * as vscode from 'vscode'
import * as diff from 'diff'
import chalk from 'chalk'

const wxmlCases = [
  'core-attribute',
  'attribute',
  'comment',
  'meta-tag',
  'event-binding',
  'slot-value',
  'static-class',
  'static-style',
  'let-var',
  'wx-for',
  'wx-if',
  'import',
  'template',
  'wxs',
  'wxs-inline',
]

const wxssCases = [
  'style-rule',
  'comment',
  'global',
  'media',
  'import',
  'font-face',
  'keyframes',
  'unknown-at-rule',
]

const wxmlTsCases = ['basic', 'special']

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

  async wrapExpect(id: string, f: (expect: Expect) => Promise<void>): Promise<number> {
    const expect = new Expect(id)
    await f(expect)
    if (expect.failureCount > 0) {
      if (OVERWRITE_SNAPSHOT === 'display') {
        // eslint-disable-next-line no-console
        console.error(
          chalk.red(
            `${expect.failureCount.toFixed(0)} snapshot(s) miss match at ${expect.actualOutputPath()}`,
          ),
        )
        expect.displayDiff()
      } else if (OVERWRITE_SNAPSHOT) {
        expect.overwriteExpected()
        // eslint-disable-next-line no-console
        console.warn(
          chalk.yellow(
            `${expect.failureCount.toFixed(0)} snapshot(s) updated ${expect.snapshotPath()}`,
          ),
        )
      } else {
        expect.writeActualAndDiff()
        // eslint-disable-next-line no-console
        console.error(
          chalk.red(
            `${expect.failureCount.toFixed(0)} snapshot(s) miss match at ${expect.actualOutputPath()}`,
          ),
        )
      }
    } else {
      expect.writeActualAndDiff()
    }
    return expect.failureCount
  }

  async forEachCase(
    ctx: Mocha.Context,
    sub: string,
    cases: string[],
    extName: string,
    f: (uri: vscode.Uri, expect: Expect) => Promise<void>,
  ) {
    const testName = ctx.test?.title || '(unnamed test)'
    const testId = normalizeTitle(testName)
    let snapshotFails = 0
    for (const name of cases) {
      const absPath = path.resolve(TEST_FIXTURE_DIR, sub, `${name}.${extName}`)
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

  async forEachWxmlCase(ctx: Mocha.Context, f: (uri: vscode.Uri, expect: Expect) => Promise<void>) {
    await this.forEachCase(ctx, 'wxml', wxmlCases, 'wxml', f)
  }

  async forEachWxssCase(ctx: Mocha.Context, f: (uri: vscode.Uri, expect: Expect) => Promise<void>) {
    await this.forEachCase(ctx, 'wxss', wxssCases, 'wxss', f)
  }

  async forEachWxmlTsCase(
    ctx: Mocha.Context,
    f: (uri: vscode.Uri, expect: Expect) => Promise<void>,
  ) {
    await this.forEachCase(ctx, 'ts', wxmlTsCases, 'wxml', f)
  }

  async casesWith<T>(
    ctx: Mocha.Context,
    sub: string,
    cases: { name: string; args: T; ext?: string }[],
    extName: string,
    f: (uri: vscode.Uri, args: T, expect: Expect) => Promise<void>,
  ) {
    const testName = ctx.test?.title || '(unnamed test)'
    const testId = normalizeTitle(testName)
    let snapshotFails = 0
    for (const { name, args, ext } of cases) {
      const absPath = path.resolve(TEST_FIXTURE_DIR, sub, `${name}.${ext ?? extName}`)
      const uri = vscode.Uri.file(absPath)
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

  async wxmlCasesWith<T>(
    ctx: Mocha.Context,
    cases: { name: string; args: T }[],
    f: (uri: vscode.Uri, args: T, expect: Expect) => Promise<void>,
  ) {
    await this.casesWith(ctx, 'wxml', cases, 'wxml', f)
  }

  async wxssCasesWith<T>(
    ctx: Mocha.Context,
    cases: { name: string; args: T; ext?: string }[],
    f: (uri: vscode.Uri, args: T, expect: Expect) => Promise<void>,
  ) {
    await this.casesWith(ctx, 'wxss', cases, 'wxss', f)
  }

  async wxmlTsCasesWith<T>(
    ctx: Mocha.Context,
    cases: { name: string; args: T; ext?: string }[],
    f: (uri: vscode.Uri, args: T, expect: Expect) => Promise<void>,
  ) {
    await this.casesWith(ctx, 'ts', cases, 'wxml', f)
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
      for (;;) {
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

  displayDiff() {
    if (this.failureCount > 0) {
      const patch = diff.createPatch(this.snapshotPath(), this.expectedStr, this.actualOutput)
      // eslint-disable-next-line no-console
      console.warn(patch)
    }
    try {
      fs.unlinkSync(this.actualOutputPath())
      fs.unlinkSync(this.diffOutputPath())
    } catch (_err) {
      // empty
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
    const normalized = JSON.parse(JSON.stringify(actual ?? null)) as unknown
    const actualStr = formatData(normalized)
    this.actualOutput += `// ====== SNAPSHOT ${this.index.toFixed(0)} ======\n`
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

const customObjectFormatter = (data: { [key: string]: unknown }): string | null => {
  if (typeof data.external === 'string' && data.external.startsWith('file:///')) {
    // local URL path
    const uri = data as unknown as vscode.Uri
    if (uri.path) {
      return path.relative(__dirname, uri.path)
    }
    return ''
  }
  if (typeof data.path === 'string' && data.scheme === 'file') {
    // local URL path
    const uri = data as unknown as vscode.Uri
    if (uri.path) {
      return path.relative(__dirname, uri.path)
    }
    return ''
  }
  return null
}

const formatData = (data: unknown) => {
  let out = ''
  const visited: Map<any, string> = new Map()
  const rec = (data: unknown, path: string[], key?: string) => {
    for (let i = 0; i < path.length; i += 1) {
      out += '  '
    }
    if (key !== undefined) out += `${key} = `
    if (typeof data === 'undefined') {
      out += 'undefined\n'
    } else if (data === null) {
      out += 'null\n'
    } else if (
      typeof data === 'bigint' ||
      typeof data === 'boolean' ||
      typeof data === 'number' ||
      typeof data === 'symbol'
    ) {
      out += data.toString()
      out += '\n'
    } else if (typeof data === 'string') {
      out += JSON.stringify(data)
      out += '\n'
    } else if (typeof data === 'function') {
      out += '[Function]'
      out += '\n'
    } else if (typeof data === 'object') {
      const customStr = customObjectFormatter(data as { [key: string]: unknown })
      if (customStr !== null) {
        out += `[Object] ${JSON.stringify(customStr)}\n`
      } else {
        const id = visited.get(data)
        if (typeof id === 'string') {
          out += `[Recursive ${id}]\n`
        } else {
          const id = path.join('/')
          visited.set(data, id)
          if (Array.isArray(data)) {
            out += `[Array]\n`
            for (let i = 0; i < data.length; i += 1) {
              const item = data[i]! as unknown
              const newPath = path.concat(String(i))
              rec(item, newPath)
            }
          } else {
            out += `[Object]\n`
            const keys = Object.keys(data)
            keys.sort()
            for (let i = 0; i < keys.length; i += 1) {
              const key = keys[i]!
              const child = (data as { [key: string]: unknown })[key]
              const newPath = path.concat(key)
              rec(child, newPath, key)
            }
          }
        }
      }
    } else {
      out += '[Unknown]\n'
    }
  }
  rec(data, [])
  return out
}
