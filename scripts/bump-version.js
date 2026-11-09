import fs from 'node:fs'
import path from 'node:path'
import { execSync } from 'node:child_process'
import { fileURLToPath } from 'node:url'

const __dirname = path.dirname(fileURLToPath(import.meta.url))
const rootDir = path.resolve(__dirname, '..')

const args = process.argv.slice(2)
const targetArg = args.find(a => !a.startsWith('--'))
const shouldTag = args.includes('--tag')

if (!targetArg) {
  console.error('\nFehler: Keine Version angegeben!')
  console.log('Verwendung: node scripts/bump-version.js <version | patch | minor | major> [--tag]')
  console.log('Beispiel:   npm run bump 0.3.0')
  console.log('            npm run bump patch\n')
  process.exit(1)
}

// 1. Read current version from package.json
const pkgPath = path.join(rootDir, 'package.json')
const pkg = JSON.parse(fs.readFileSync(pkgPath, 'utf8'))
const currentVersion = pkg.version

function bumpSemver(version, type) {
  const parts = version.split('.').map(n => parseInt(n, 10))
  if (parts.length !== 3 || parts.some(isNaN)) {
    throw new Error(`Ungültige SemVer-Version: ${version}`)
  }
  let [major, minor, patch] = parts
  if (type === 'major') {
    major++
    minor = 0
    patch = 0
  } else if (type === 'minor') {
    minor++
    patch = 0
  } else if (type === 'patch') {
    patch++
  } else {
    throw new Error(`Unbekannter Bump-Typ: ${type}`)
  }
  return `${major}.${minor}.${patch}`
}

let newVersion = targetArg
if (['patch', 'minor', 'major'].includes(targetArg)) {
  newVersion = bumpSemver(currentVersion, targetArg)
}

// Clean leading 'v' if entered by mistake (e.g. v0.3.0 -> 0.3.0)
newVersion = newVersion.replace(/^v/, '')

console.log(`\nPlayCounter Version Bump: v${currentVersion} -> v${newVersion}\n`)

// 1. Update package.json
pkg.version = newVersion
fs.writeFileSync(pkgPath, JSON.stringify(pkg, null, 2) + '\n', 'utf8')
console.log(`✓ package.json aktualisiert (${newVersion})`)

// 2. Update src-tauri/tauri.conf.json
const tauriConfPath = path.join(rootDir, 'src-tauri', 'tauri.conf.json')
if (fs.existsSync(tauriConfPath)) {
  const tauriConf = JSON.parse(fs.readFileSync(tauriConfPath, 'utf8'))
  tauriConf.version = newVersion
  fs.writeFileSync(tauriConfPath, JSON.stringify(tauriConf, null, 2) + '\n', 'utf8')
  console.log(`✓ src-tauri/tauri.conf.json aktualisiert (${newVersion})`)
}

// 3. Update src-tauri/Cargo.toml
const cargoPath = path.join(rootDir, 'src-tauri', 'Cargo.toml')
if (fs.existsSync(cargoPath)) {
  let cargoContent = fs.readFileSync(cargoPath, 'utf8')
  cargoContent = cargoContent.replace(/^version\s*=\s*"[^"]+"/m, `version = "${newVersion}"`)
  fs.writeFileSync(cargoPath, cargoContent, 'utf8')
  console.log(`✓ src-tauri/Cargo.toml aktualisiert (${newVersion})`)
}

// 4. Update packaging/arch/PKGBUILD
const pkgbuildPath = path.join(rootDir, 'packaging', 'arch', 'PKGBUILD')
if (fs.existsSync(pkgbuildPath)) {
  let pkgbuild = fs.readFileSync(pkgbuildPath, 'utf8')
  pkgbuild = pkgbuild.replace(/^pkgver=.*/m, `pkgver=${newVersion}`)
  pkgbuild = pkgbuild.replace(/^pkgrel=.*/m, `pkgrel=1`)
  fs.writeFileSync(pkgbuildPath, pkgbuild, 'utf8')
  console.log(`✓ packaging/arch/PKGBUILD aktualisiert (${newVersion})`)
}

let currentBranch = 'master'
try {
  currentBranch = execSync('git rev-parse --abbrev-ref HEAD', { cwd: rootDir }).toString().trim()
} catch {}

if (shouldTag) {
  try {
    console.log(`\nErstelle Git Commit und Tag v${newVersion}...`)
    execSync('git add package.json src-tauri/tauri.conf.json src-tauri/Cargo.toml packaging/arch/PKGBUILD', { stdio: 'inherit', cwd: rootDir })
    execSync(`git commit -m "chore: release v${newVersion}"`, { stdio: 'inherit', cwd: rootDir })
    execSync(`git tag v${newVersion}`, { stdio: 'inherit', cwd: rootDir })
    console.log(`✓ Tag v${newVersion} erstellt!`)
    console.log(`\nJetzt ausführen zum Veröffentlichen:`)
    console.log(`git push origin ${currentBranch} --tags\n`)
  } catch (err) {
    console.error('Git Tagging fehlgeschlagen:', err.message)
  }
} else {
  console.log(`\nFertig! Um das Release jetzt zu committen und zu taggen:`)
  console.log(`git commit -am "chore: release v${newVersion}"`)
  console.log(`git tag v${newVersion}`)
  console.log(`git push origin ${currentBranch} --tags\n`)
}
