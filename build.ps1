# build.ps1 — Build with git commit hash in version
$hash = (git rev-parse --short HEAD)
Write-Host "Build: 6.9.0-$hash"
# Patch versions
(Get-Content package.json) -replace '"version": "6\.9\.0"', ('"version": "6.9.0-' + $hash + '"') | Set-Content package.json
(Get-Content src-tauri/Cargo.toml) -replace '^version = "6\.9\.0"', ('version = "6.9.0-' + $hash + '"') | Set-Content src-tauri/Cargo.toml
# Build
npx tauri build
# Revert
git checkout -- package.json src-tauri/Cargo.toml
Write-Host "Done: Workflow Engine_6.9.0_${hash}_x64-setup.exe"
