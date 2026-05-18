# build.ps1 — Build with git commit hash in version
# Run from Windows (not WSL). $PSScriptRoot resolves to the repo root.
Set-Location $PSScriptRoot

$hash = (git rev-parse --short HEAD)
Write-Host "Build: 6.9.0-$hash"

# Write UTF-8 without BOM (compatible with PowerShell 5.1)
function Write-Utf8NoBOM($path, $content) {
    $utf8 = New-Object System.Text.UTF8Encoding($false)
    [System.IO.File]::WriteAllText($path, $content, $utf8)
}

# Patch versions
$pkg = Get-Content "$PSScriptRoot\package.json" -Raw | ConvertFrom-Json
$pkg.version = "6.9.0-$hash"
Write-Utf8NoBOM "$PSScriptRoot\package.json" ($pkg | ConvertTo-Json -Depth 10)

$cargo = Get-Content "$PSScriptRoot\src-tauri\Cargo.toml" -Raw
$cargo = $cargo -replace '^version = "[^"]*"', ('version = "6.9.0-' + $hash + '"')
Write-Utf8NoBOM "$PSScriptRoot\src-tauri\Cargo.toml" $cargo

$tauri = Get-Content "$PSScriptRoot\src-tauri\tauri.conf.json" -Raw | ConvertFrom-Json
$tauri.version = "6.9.0-$hash"
Write-Utf8NoBOM "$PSScriptRoot\src-tauri\tauri.conf.json" ($tauri | ConvertTo-Json -Depth 10)

# Build
npx tauri build
# Revert
git checkout -- "$PSScriptRoot\package.json" "$PSScriptRoot\src-tauri\Cargo.toml" "$PSScriptRoot\src-tauri\tauri.conf.json"
Write-Host "Done: Workflow Engine_6.9.0_${hash}_x64-setup.exe"
