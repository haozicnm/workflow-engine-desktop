# build.ps1 — Build with git commit hash in version
# Ensure we're in the script's directory (not WSL UNC path)
Set-Location $PSScriptRoot

$hash = (git rev-parse --short HEAD)
Write-Host "Build: 6.9.0-$hash"
# Patch versions (use JSON manipulation to avoid regex escaping issues)
$pkg = Get-Content package.json -Raw | ConvertFrom-Json
$pkg.version = "6.9.0-$hash"
$pkg | ConvertTo-Json -Depth 10 | Out-File -Encoding utf8NoBOM package.json

$cargo = Get-Content src-tauri/Cargo.toml -Raw
$cargo = $cargo -replace '^version = "[^"]*"', ('version = "6.9.0-' + $hash + '"')
$cargo | Out-File -Encoding utf8NoBOM src-tauri/Cargo.toml

$tauri = Get-Content src-tauri/tauri.conf.json -Raw | ConvertFrom-Json
$tauri.version = "6.9.0-$hash"
$tauri | ConvertTo-Json -Depth 10 | Out-File -Encoding utf8NoBOM src-tauri/tauri.conf.json

# Build
npx tauri build
# Revert
git checkout -- package.json src-tauri/Cargo.toml src-tauri/tauri.conf.json
Write-Host "Done: Workflow Engine_6.9.0_${hash}_x64-setup.exe"
