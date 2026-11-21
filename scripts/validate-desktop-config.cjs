const fs = require('fs')

const packageJson = require('../package.json')
const tauriConfig = require('../src-tauri/tauri.conf.json')
const capability = require('../src-tauri/capabilities/default.json')
const cargoToml = fs.readFileSync('src-tauri/Cargo.toml', 'utf8')
const guiSource = fs.readFileSync('src-tauri/src/bin/gui.rs', 'utf8')
const acl = fs.readFileSync('src-tauri/permissions/desktop-ipc.toml', 'utf8')

if (packageJson.version !== tauriConfig.version) {
  throw new Error(`Version mismatch: package.json=${packageJson.version}, tauri.conf.json=${tauriConfig.version}`)
}

const cargoVersion = cargoToml.match(/^version = "([^"]+)"/m)?.[1]
if (cargoVersion !== packageJson.version) {
  throw new Error(`Version mismatch: package.json=${packageJson.version}, Cargo.toml=${cargoVersion}`)
}
if (tauriConfig.mainBinaryName !== 'workflow-engine-gui') {
  throw new Error('Desktop bundle must use workflow-engine-gui')
}
if (!cargoToml.includes('default-run = "workflow-engine-gui"')) {
  throw new Error('Cargo default-run must select workflow-engine-gui')
}
if (tauriConfig.bundle.resources['../dist'] !== 'dist/') {
  throw new Error('Desktop bundle must preserve the dist directory structure')
}
if (
  !capability.windows.includes('main') ||
  !capability.permissions.includes('core:event:allow-listen') ||
  !capability.permissions.includes('desktop-ipc') ||
  !capability.remote?.urls?.includes('http://127.0.0.1:19529')
) {
  throw new Error('Main window and its loopback origin must allow desktop IPC')
}

const commands = [...guiSource.matchAll(/workflow_engine::commands::\w+::(\w+),/g)].map(match => match[1])
const allowed = [...acl.matchAll(/"(\w+)"/g)].map(match => match[1])
const missing = commands.filter(command => !allowed.includes(command))
const stale = allowed.filter(command => !commands.includes(command))
if (missing.length || stale.length) {
  throw new Error(`IPC permission mismatch: missing=${missing.join(',')}, stale=${stale.join(',')}`)
}

console.log('Desktop bundle:', packageJson.version, tauriConfig.mainBinaryName, 'commands:', commands.length)
