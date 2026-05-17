# build.ps1 — Build with git commit hash in version
$hash = (git rev-parse --short HEAD)
Write-Host "Build: 6.9.0-$hash"
# Patch versions (use Out-File -Encoding utf8NoBOM to avoid corrupting UTF-8)
(Get-Content package.json) -replace '"version": "6\.9\.0"', ('"version": "6.9.0-' + $hash + '"') | Out-File -Encoding utf8NoBOM package.json
(Get-Content src-tauri/Cargo.toml) -replace '^version = "6\.9\.0"', ('version = "6.9.0-' + $hash + '"') | Out-File -Encoding utf8NoBOM src-tauri/Cargo.toml
(Get-Content src-tauri/tauri.conf.json) -replace '"version": "6\.9\.0"', ('"version": "6.9.0-' + $hash + '"') | Out-File -Encoding utf8NoBOM src-tauri/tauri.conf.json
# Build
npx tauri build
# Revert
git checkout -- package.json src-tauri/Cargo.toml src-tauri/tauri.conf.json
Write-Host "Done: Workflow Engine_6.9.0_${hash}_x64-setup.exe"
