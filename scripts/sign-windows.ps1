[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [string[]] $Path,

    [string] $CertificatePath = $env:WINDOWS_CERTIFICATE_PATH,

    [AllowEmptyString()]
    [string] $CertificatePassword = $env:WINDOWS_CERTIFICATE_PASSWORD,

    [string] $TimestampUrl = "https://timestamp.digicert.com",

    [switch] $RequireSignature,

    [switch] $VerifyOnly
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

function Get-SignToolPath {
    $command = Get-Command signtool.exe -ErrorAction SilentlyContinue
    if ($null -ne $command) {
        return $command.Source
    }

    $programFilesX86 = [Environment]::GetFolderPath([Environment+SpecialFolder]::ProgramFilesX86)
    if ([string]::IsNullOrWhiteSpace($programFilesX86)) {
        throw "The 32-bit Program Files directory could not be resolved; install the Windows SDK signing tools."
    }
    $kitsRoot = Join-Path -Path $programFilesX86 -ChildPath "Windows Kits\10\bin"
    if (-not (Test-Path -LiteralPath $kitsRoot)) {
        throw "Windows SDK signtool.exe was not found. Install the Windows SDK signing tools."
    }

    $candidate = Get-ChildItem -Path $kitsRoot -Filter signtool.exe -Recurse -File |
        Where-Object { $_.FullName -match '\\x64\\signtool\.exe$' } |
        Sort-Object { [version] $_.Directory.Parent.Name } -Descending |
        Select-Object -First 1

    if ($null -eq $candidate) {
        throw "Windows SDK signtool.exe (x64) was not found under $kitsRoot."
    }

    return $candidate.FullName
}

function Resolve-TargetFiles {
    $files = @()
    foreach ($target in $Path) {
        $resolved = Resolve-Path -LiteralPath $target
        foreach ($item in $resolved) {
            $file = Get-Item -LiteralPath $item.Path
            if ($file.PSIsContainer) {
                throw "Signing target must be a file: $($file.FullName)"
            }
            $files += $file
        }
    }

    $uniqueFiles = @($files | Sort-Object FullName -Unique)
    if ($uniqueFiles.Count -eq 0) {
        throw "No signing targets were supplied."
    }
    return $uniqueFiles
}

function Assert-AuthenticodeSignature {
    param(
        [Parameter(Mandatory = $true)]
        [System.IO.FileInfo] $File,

        [Parameter(Mandatory = $true)]
        [string] $SignTool
    )

    $signature = Get-AuthenticodeSignature -LiteralPath $File.FullName
    if ($signature.Status -ne [System.Management.Automation.SignatureStatus]::Valid -or
        $null -eq $signature.SignerCertificate) {
        throw "Authenticode verification failed for $($File.FullName): $($signature.Status)"
    }

    $verifyOutput = & $SignTool verify /pa /all $File.FullName 2>&1
    $verifyExitCode = $LASTEXITCODE
    $verifyOutput | ForEach-Object { Write-Host $_ }
    if ($verifyExitCode -ne 0) {
        throw "signtool verify failed for $($File.FullName) with exit code $verifyExitCode."
    }

    Write-Host ("WINDOWS_SIGNING file={0} status=Valid subject={1} thumbprint={2}" -f
        $File.FullName,
        $signature.SignerCertificate.Subject,
        $signature.SignerCertificate.Thumbprint)
}

$targetFiles = @(Resolve-TargetFiles)

if ($VerifyOnly) {
    if ($RequireSignature) {
        $signTool = Get-SignToolPath
        foreach ($file in $targetFiles) {
            Assert-AuthenticodeSignature -File $file -SignTool $signTool
        }
    }
    else {
        foreach ($file in $targetFiles) {
            $status = (Get-AuthenticodeSignature -LiteralPath $file.FullName).Status
            Write-Host "WINDOWS_SIGNING file=$($file.FullName) status=$status mode=verify-only"
        }
    }
    return
}

if ([string]::IsNullOrWhiteSpace($CertificatePath)) {
    foreach ($file in $targetFiles) {
        $status = (Get-AuthenticodeSignature -LiteralPath $file.FullName).Status
        Write-Host "WINDOWS_SIGNING file=$($file.FullName) status=$status certificate=absent"
    }

    if ($RequireSignature) {
        throw "WINDOWS_CERTIFICATE_PATH is required for signed tag releases."
    }

    Write-Warning "No code signing certificate was supplied; artifacts remain unsigned."
    return
}

$certificate = Resolve-Path -LiteralPath $CertificatePath
$signTool = Get-SignToolPath
Write-Host "Using signtool: $signTool"

foreach ($file in $targetFiles) {
    $signed = $false
    for ($attempt = 1; $attempt -le 3 -and -not $signed; $attempt++) {
        Write-Host "Signing $($file.FullName) (attempt $attempt/3)"
        $signOutput = & $signTool sign /fd SHA256 /f $certificate.Path /p $CertificatePassword /tr $TimestampUrl /td SHA256 $file.FullName 2>&1
        $signExitCode = $LASTEXITCODE
        $signOutput | ForEach-Object { Write-Host $_ }
        if ($signExitCode -eq 0) {
            $signed = $true
        }
        elseif ($attempt -lt 3) {
            Start-Sleep -Seconds (5 * $attempt)
        }
    }

    if (-not $signed) {
        throw "signtool sign failed for $($file.FullName) after 3 attempts."
    }

    Assert-AuthenticodeSignature -File $file -SignTool $signTool
}
