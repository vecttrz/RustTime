$ErrorActionPreference = 'Stop'

$root = Split-Path -Parent $PSScriptRoot
$version = '0.1.0'
$releaseDir = Join-Path $root 'release'
$stagingDir = Join-Path $releaseDir 'portable'
$exeSource = Join-Path $root 'target\release\rusttime.exe'
$setupFile = Join-Path $releaseDir "RustTime-v$version-setup.exe"
$zipFile = Join-Path $releaseDir "RustTime-v$version-portable.zip"
$checksumsFile = Join-Path $releaseDir "RustTime-v$version-checksums.txt"

# Make sure cargo is available.
if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
    $env:Path = "$env:USERPROFILE\.cargo\bin;$env:Path"
}

Write-Host '==> Building in release mode...'
Push-Location $root
try {
    cargo build --release
    if ($LASTEXITCODE -ne 0) { throw 'cargo build --release failed.' }
}
finally {
    Pop-Location
}

if (-not (Test-Path $exeSource)) {
    throw "Compiled executable not found: $exeSource"
}

New-Item -ItemType Directory -Force -Path $releaseDir | Out-Null
Remove-Item $stagingDir -Recurse -Force -ErrorAction SilentlyContinue
New-Item -ItemType Directory -Force -Path $stagingDir | Out-Null

Write-Host '==> Preparing portable version...'
Copy-Item $exeSource (Join-Path $stagingDir 'RustTime.exe')
Copy-Item (Join-Path $root 'README.md') $stagingDir
Copy-Item (Join-Path $root 'LICENSE') $stagingDir
Copy-Item (Join-Path $root 'CHANGELOG.md') $stagingDir
Copy-Item (Join-Path $root 'docs\PRODUCT.md') $stagingDir
New-Item -ItemType Directory -Force -Path (Join-Path $stagingDir 'docs') | Out-Null
Copy-Item (Join-Path $root 'docs\screenshot-en.png') (Join-Path $stagingDir 'docs')

Remove-Item $zipFile -Force -ErrorAction SilentlyContinue
Compress-Archive -Path (Join-Path $stagingDir '*') -DestinationPath $zipFile -CompressionLevel Optimal

Write-Host '==> Generating installer with Inno Setup...'
$isccCandidates = @(
    "$env:LOCALAPPDATA\Programs\Inno Setup 6\ISCC.exe",
    "${env:ProgramFiles(x86)}\Inno Setup 6\ISCC.exe",
    "$env:ProgramFiles\Inno Setup 6\ISCC.exe"
)
$iscc = $isccCandidates | Where-Object { Test-Path $_ } | Select-Object -First 1
if (-not $iscc) {
    throw 'ISCC.exe not found. Install Inno Setup 6 (winget install --id JRSoftware.InnoSetup).'
}

Remove-Item $setupFile -Force -ErrorAction SilentlyContinue
& $iscc (Join-Path $root 'installer\RustTime.iss')
if ($LASTEXITCODE -ne 0) { throw 'ISCC.exe failed to generate the installer.' }

Write-Host '==> Computing SHA-256 checksums...'
$lines = @()
foreach ($file in @($setupFile, $zipFile)) {
    $hash = (Get-FileHash -Path $file -Algorithm SHA256).Hash.ToLowerInvariant()
    $lines += "$hash  $([System.IO.Path]::GetFileName($file))"
}
Set-Content -Path $checksumsFile -Value $lines -Encoding UTF8

Remove-Item $stagingDir -Recurse -Force -ErrorAction SilentlyContinue

Write-Host ''
Write-Host 'Generated artifacts:'
Get-ChildItem -Path $releaseDir -File | Select-Object Name, Length | Format-Table -AutoSize
Write-Host "Checksums: $checksumsFile"
