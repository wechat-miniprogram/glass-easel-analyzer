/* eslint-disable no-console */

const fs = require('fs')
const childProcess = require('child_process')

const writeFileAndGitAdd = (p, content) => {
  fs.writeFileSync(p, content)
  if (childProcess.spawnSync('git', ['add', p]).status !== 0) {
    throw new Error(`failed to execute git add on ${p}`)
  }
}

// check arguments
const version = process.argv[2]
if (!version) {
  throw new Error('version not given in argv')
}
if (!/[0-9]+\.[0-9]+\.[0-9]+/.test(version)) {
  throw new Error('version illegal')
}

// avoid rust warnings
console.info('Run cargo check')
if (
  childProcess.spawnSync('cargo', ['check'], {
    env: { RUSTFLAGS: '-D warnings', ...process.env },
    stdio: 'inherit',
  }).status !== 0
) {
  throw new Error('failed to check rust modules (are there rust warnings or errors?)')
}

// force rust formatting
console.info('Run cargo fmt --check')
if (
  childProcess.spawnSync('cargo', ['fmt', '--check'], {
    stdio: 'inherit',
  }).status !== 0
) {
  throw new Error('failed to check formatting of rust modules')
}

// avoid eslint warnings
;['vscode-extension'].forEach((p) => {
  console.info(`Run eslint on ${p}`)
  if (
    childProcess.spawnSync('npx', ['eslint', '-c', 'eslint.config.js', 'src'], {
      cwd: p,
      stdio: 'inherit',
    }).status !== 0
  ) {
    throw new Error('failed to lint modules (are there eslint warnings or errors?)')
  }
})

// check git status
const gitStatusRes = childProcess.spawnSync('git', ['diff', '--name-only'], { encoding: 'utf8' })
if (gitStatusRes.status !== 0 || gitStatusRes.stdout.length > 0) {
  throw new Error('failed to check git status (are there uncommitted changes?)')
}

// change npm version
;['vscode-extension/package.json'].forEach((p) => {
  let content = fs.readFileSync(p, { encoding: 'utf8' })
  let oldVersion
  const refVersions = []
  content = content.replace(/"version": "(.+)"/, (_, v) => {
    oldVersion = v
    return `"version": "${version}"`
  })
  if (!oldVersion) {
    throw new Error(`version segment not found in ${p}`)
  }
  console.info(`Update ${p} version from "${oldVersion}" to "${version}"`)
  refVersions.forEach(({ mod, v }) => {
    console.info(`  + dependency ${mod} version from "${v}" to "${version}"`)
  })
  writeFileAndGitAdd(p, content)
})

// change cargo version
;['Cargo.toml'].forEach(
  (p) => {
    let content = fs.readFileSync(p, { encoding: 'utf8' })
    let oldVersion
    content = content.replace(/\nversion = "(.+)"/, (_, v) => {
      oldVersion = v
      return `\nversion = "${version}"`
    })
    if (!oldVersion) {
      throw new Error(`version segment not found in ${p}`)
    }
    console.info(`Update ${p} version from "${oldVersion}" to "${version}"`)
    writeFileAndGitAdd(p, content)
  },
)

// npm test
;['vscode-extension'].forEach((p) => {
  console.info(`Run npm test in ${p}`)
  if (childProcess.spawnSync('npm', ['test'], { cwd: p, stdio: 'inherit' }).status !== 0) {
    throw new Error('failed to run npm test')
  }
})

// add lock files
;['Cargo.lock', 'vscode-extension/package.json'].forEach((p) => {
  if (childProcess.spawnSync('git', ['add', p]).status !== 0) {
    throw new Error(`failed to execute git add on ${p}`)
  }
})

// git commit
if (
  childProcess.spawnSync('git', ['commit', '--message', `version: ${version}`], {
    stdio: 'inherit',
  }).status !== 0
) {
  throw new Error('failed to execute git commit')
}

// add a git tag and push
console.info('Push to git origin')
if (childProcess.spawnSync('git', ['tag', `v${version}`]).status !== 0) {
  throw new Error('failed to execute git tag')
}
if (childProcess.spawnSync('git', ['push'], { stdio: 'inherit' }).status !== 0) {
  throw new Error('failed to execute git push')
}
if (childProcess.spawnSync('git', ['push', '--tags'], { stdio: 'inherit' }).status !== 0) {
  throw new Error('failed to execute git push --tags')
}

console.info('Version updated! Wait the remote actions to build and publish.')
