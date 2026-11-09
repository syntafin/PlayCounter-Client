import fs from 'node:fs'
import path from 'node:path'
import { execSync } from 'node:child_process'
import { fileURLToPath } from 'node:url'

const __dirname = path.dirname(fileURLToPath(import.meta.url))
const rootDir = path.resolve(__dirname, '..')

const args = process.argv.slice(2)
const noGit = args.includes('--no-git')
const noPush = args.includes('--no-push')
const targetArg = args.find(a => !a.startsWith('--'))

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
if (!targetArg || targetArg === 'patch') {
  newVersion = bumpSemver(currentVersion, 'patch')
} else if (['minor', 'major'].includes(targetArg)) {
  newVersion = bumpSemver(currentVersion, targetArg)
}

// Clean leading 'v' if entered by mistake (e.g. v0.3.0 -> 0.3.0)
newVersion = newVersion.replace(/^v/, '')

console.log(`\n🚀 PlayCounter Automated Release Bump: v${currentVersion} -> v${newVersion}\n`)

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

if (!noGit) {
  let currentBranch = 'main'
  try {
    currentBranch = execSync('git rev-parse --abbrev-ref HEAD', { cwd: rootDir }).toString().trim()
  } catch {}

  console.log(`\n📦 Committe Versionsdateien...`)
  execSync('git add .', { stdio: 'inherit', cwd: rootDir })

  try {
    execSync(`git commit -m "chore: release v${newVersion}"`, { stdio: 'inherit', cwd: rootDir })
    console.log(`✓ Commit "chore: release v${newVersion}" erstellt.`)
  } catch {
    console.log(`ℹ Keine neuen Dateiänderungen für diesen Commit.`)
  }

  console.log(`🏷️ Erstelle Git-Tag v${newVersion}...`)
  execSync(`git tag -f v${newVersion}`, { stdio: 'inherit', cwd: rootDir })
  console.log(`✓ Git-Tag v${newVersion} gesetzt.`)

  if (!noPush) {
    console.log(`\n⬆️ Pushe Branch '${currentBranch}' und Tags zu origin...`)
    execSync(`git push origin ${currentBranch} --tags --force`, { stdio: 'inherit', cwd: rootDir })
    console.log(`\n✅ Erfolgreich gepusht! GitHub Actions baut und veröffentlicht das Release v${newVersion} jetzt automatisch.\n`)
  } else {
    console.log(`\nℹ Push übersprungen (--no-push aktiv).`)
  }
} else {
  console.log(`\nℹ Git-Aktionen übersprungen (--no-git aktiv).`)
}
