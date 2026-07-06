#Requires -Version 5.1
<#
.SYNOPSIS
    Build (debug or release) and run LMahjong with SDL2 DLLs on PATH.

.PARAMETER Release
    Build and run in release mode. Default is debug.

.EXAMPLE
    .\run.ps1
    .\run.ps1 -Release
#>

param(
    [switch]$Release
)

$ErrorActionPreference = 'Stop'
$ScriptDir = $PSScriptRoot

# SDL2 DLL directory (created by package.ps1)
$DllDir = Join-Path $ScriptDir 'sdl2-dev\merged\dll\x64'

if (-not (Test-Path $DllDir)) {
    Write-Host "SDL2 DLLs not found at: $DllDir" -ForegroundColor Red
    Write-Host "Run .\package.ps1 -Action portable first to download SDL2 libraries." -ForegroundColor Yellow
    exit 1
}

# Add DLL directory to PATH for this process
$env:PATH = "$DllDir;$env:PATH"

# Build
$buildArgs = @('build')
if ($Release) { $buildArgs += '--release' }

Write-Host "Building ($( if ($Release) {'release'} else {'debug'} ))..." -ForegroundColor Cyan
cargo @buildArgs
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

# Run
$profile = if ($Release) { 'release' } else { 'debug' }
$exe = Join-Path $ScriptDir "target\$profile\lmahjong.exe"

Write-Host "Running: $exe" -ForegroundColor Green
& $exe
