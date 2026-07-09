#Requires -Version 5.1
<#
.SYNOPSIS
    Build (debug or release) and run xMahjong with SDL2 DLLs on PATH.

.PARAMETER Release
    Build and run in release mode. Default is debug.

.PARAMETER Dev
    Run in development mode (disables saving, skips update check).

.PARAMETER Level
    Start at a specific level (1-50). Requires -Dev flag.

.EXAMPLE
    .\run.ps1
    .\run.ps1 -Release
    .\run.ps1 -Dev -Level 29
#>

param(
    [switch]$Release,
    [switch]$Dev,
    [int]$Level = 0
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
$exe = Join-Path $ScriptDir "target\$profile\xmahjong.exe"

$runArgs = @()
if ($Dev) {
    $runArgs += '--dev'
    if ($Level -gt 0) {
        $runArgs += '--level'
        $runArgs += $Level.ToString()
    }
    Write-Host "Running (DEV mode, level $Level): $exe $runArgs" -ForegroundColor Yellow
} else {
    Write-Host "Running: $exe" -ForegroundColor Green
}

& $exe @runArgs
