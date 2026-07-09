#Requires -Version 5.1
<#
.SYNOPSIS
    Build xMahjong for Windows: portable ZIP, MSI installer, and/or MSIX package.

.DESCRIPTION
    package.ps1 — Windows packaging script for xMahjong.

    Actions (-Action):
      portable  — Release build + SDL2 DLLs bundled into a ZIP archive.
      msi       — Portable ZIP + MSI installer via WiX Toolset v4.
      msix      — MSIX package (sideload or Microsoft Store submission).
      all       — All three (default).

    SDL2 setup (automatic)
    ──────────────────────
    On MSVC builds the linker needs SDL2.lib / SDL2_image.lib / etc.
    This script downloads the official SDL2 "VC" developer packages from
    GitHub Releases automatically and places them in:
        .\sdl2-dev\   (git-ignored, kept between runs)

    If you already have the SDL2 dev packages somewhere, set:
        $env:SDL2_DEV_DIR = "C:\path\to\SDL2-devel-x.x.x-VC"
    and the script will use that instead of downloading.

    WiX Toolset (for MSI, optional)
    ────────────────────────────────
        dotnet tool install --global wix
        wix extension add WixToolset.UI.wixext

    MSIX prerequisites
    ──────────────────
    makeappx.exe and signtool.exe ship with the Windows SDK (installed with
    Visual Studio or the standalone Windows SDK).  The script searches the
    standard SDK paths automatically.

    For Store submission the package must be signed with a certificate whose
    Subject matches MSIX_PUBLISHER exactly.  For sideloading, a self-signed
    certificate is generated automatically (requires New-SelfSignedCertificate,
    available in PowerShell 5+ on Windows 10/11).

.PARAMETER Action
    One of: portable | msi | msix | all   (default: all)

.EXAMPLE
    .\package.ps1
    .\package.ps1 -Action portable
    .\package.ps1 -Action msi
    .\package.ps1 -Action msix
#>

param(
    [ValidateSet('portable', 'msi', 'msix', 'all')]
    [string]$Action = 'all'
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

# ─── Configuration ────────────────────────────────────────────────────────────

$APP_NAME        = 'xmahjong'
$APP_DISPLAY     = 'xMahjong'
$APP_VERSION     = (Get-Content "$PSScriptRoot\release" -Raw).Trim()
$APP_DESCRIPTION = 'A Tux-themed Mahjong solitaire game'
$APP_MANUFACTURER= 'Ricardo Wagemaker'
# Stable GUIDs — do NOT change between releases.
$UPGRADE_GUID    = 'B7C83D2E-5F6A-4B9D-0E3C-2F7A8B4C1D5E'

$SCRIPT_DIR   = $PSScriptRoot
$BUILD_DIR    = Join-Path $SCRIPT_DIR 'target\package'
$RELEASE_BIN  = Join-Path $SCRIPT_DIR "target\release\$APP_NAME.exe"
$SDL2_DEV_DIR_LOCAL = Join-Path $SCRIPT_DIR 'sdl2-dev'

# SDL2 release versions to download (VC = MSVC developer packages).
# Update these when bumping SDL2 versions.
$SDL2_VER        = '2.30.9'
$SDL2_IMAGE_VER  = '2.8.2'
$SDL2_MIXER_VER  = '2.8.0'
$SDL2_TTF_VER    = '2.22.0'

# Runtime DLL names that must ship alongside the .exe
$SDL2_RUNTIME_DLLS = @(
    'SDL2.dll',
    'SDL2_image.dll',
    'SDL2_mixer.dll',
    'SDL2_ttf.dll'
)

# ─── MSIX / AppX identity ─────────────────────────────────────────────────────
# Edit these values to match your Microsoft Partner Center app registration.
# Publisher must be the exact Subject of your signing certificate.
$MSIX_PUBLISHER         = 'CN=47955afa-afc7-46ee-abc1-02ab2632b4ad'
$MSIX_PUBLISHER_DISPLAY = 'Ricardo Wagemaker'
$MSIX_APP_ID            = 'xMahjong'          # No spaces, no special chars
$MSIX_DESCRIPTION       = $APP_DESCRIPTION
# Version must be 4-part (Major.Minor.Build.Revision) for MSIX
$MSIX_VERSION           = $APP_VERSION + '.0'

# ─── Helpers ──────────────────────────────────────────────────────────────────

function Write-Step([string]$msg) { Write-Host "`n==> $msg" -ForegroundColor Cyan }
function Write-Info([string]$msg) { Write-Host "    $msg" }
function Write-Warn([string]$msg) { Write-Host "    WARNING: $msg" -ForegroundColor Yellow }

function Require-Command([string]$cmd) {
    if (-not (Get-Command $cmd -ErrorAction SilentlyContinue)) {
        throw "Required command '$cmd' not found on PATH."
    }
}

# Download a URL to a file, showing progress.
function Download-File([string]$url, [string]$dest) {
    Write-Info "Downloading: $url"
    $wc = New-Object System.Net.WebClient
    $wc.DownloadFile($url, $dest)
    Write-Info "Saved to: $dest"
}

# Expand a zip archive to a directory.
function Expand-Zip([string]$zipPath, [string]$destDir) {
    if (Test-Path $destDir) { Remove-Item $destDir -Recurse -Force }
    Add-Type -AssemblyName System.IO.Compression.FileSystem
    [System.IO.Compression.ZipFile]::ExtractToDirectory($zipPath, $destDir)
}

# ─── SDL2 Dev Package Setup ───────────────────────────────────────────────────
#
# Downloads the four SDL2 VC developer zips from GitHub Releases and extracts
# them into $SDL2_DEV_DIR_LOCAL so every subsequent run can reuse them.
#
# Sets $script:SDL2_LIB_DIR  — path to the x64 .lib files  (for the linker)
# Sets $script:SDL2_DLL_DIR  — path to the x64 .dll files  (bundled at runtime)

function Setup-SDL2Dev {
    Write-Step "Setting up SDL2 developer libraries..."

    # Allow full override via environment variable
    if ($env:SDL2_DEV_DIR -and (Test-Path $env:SDL2_DEV_DIR)) {
        Write-Info "Using SDL2_DEV_DIR from environment: $env:SDL2_DEV_DIR"
        $script:SDL2_LIB_DIR = Find-SDL2SubDir $env:SDL2_DEV_DIR 'lib\x64'
        $script:SDL2_DLL_DIR = $script:SDL2_LIB_DIR   # VC zips have .dll alongside .lib
        return
    }

    New-Item $SDL2_DEV_DIR_LOCAL -ItemType Directory -Force | Out-Null
    $tmp = Join-Path $env:TEMP 'sdl2-pkg'
    New-Item $tmp -ItemType Directory -Force | Out-Null

    # Each package extracts into its own isolated subdirectory under sdl2-dev\
    # so they cannot overwrite each other.
    $pkgs = @(
        @{
            Name    = "SDL2 $SDL2_VER"
            ZipName = "SDL2-devel-$SDL2_VER-VC.zip"
            Url     = "https://github.com/libsdl-org/SDL/releases/download/release-$SDL2_VER/SDL2-devel-$SDL2_VER-VC.zip"
            # Extraction target — each package gets its own folder
            ExtractTo = Join-Path $SDL2_DEV_DIR_LOCAL 'pkg-sdl2'
        },
        @{
            Name    = "SDL2_image $SDL2_IMAGE_VER"
            ZipName = "SDL2_image-devel-$SDL2_IMAGE_VER-VC.zip"
            Url     = "https://github.com/libsdl-org/SDL_image/releases/download/release-$SDL2_IMAGE_VER/SDL2_image-devel-$SDL2_IMAGE_VER-VC.zip"
            ExtractTo = Join-Path $SDL2_DEV_DIR_LOCAL 'pkg-sdl2_image'
        },
        @{
            Name    = "SDL2_mixer $SDL2_MIXER_VER"
            ZipName = "SDL2_mixer-devel-$SDL2_MIXER_VER-VC.zip"
            Url     = "https://github.com/libsdl-org/SDL_mixer/releases/download/release-$SDL2_MIXER_VER/SDL2_mixer-devel-$SDL2_MIXER_VER-VC.zip"
            ExtractTo = Join-Path $SDL2_DEV_DIR_LOCAL 'pkg-sdl2_mixer'
        },
        @{
            Name    = "SDL2_ttf $SDL2_TTF_VER"
            ZipName = "SDL2_ttf-devel-$SDL2_TTF_VER-VC.zip"
            Url     = "https://github.com/libsdl-org/SDL_ttf/releases/download/release-$SDL2_TTF_VER/SDL2_ttf-devel-$SDL2_TTF_VER-VC.zip"
            ExtractTo = Join-Path $SDL2_DEV_DIR_LOCAL 'pkg-sdl2_ttf'
        }
    )

    # Download and extract each package into its own folder (skip if done)
    foreach ($pkg in $pkgs) {
        $marker = Join-Path $pkg.ExtractTo '.extracted'
        if (Test-Path $marker) {
            Write-Info "$($pkg.Name): already extracted, skipping."
            continue
        }

        $zipPath = Join-Path $tmp $pkg.ZipName
        if (-not (Test-Path $zipPath)) {
            Download-File $pkg.Url $zipPath
        }

        Write-Info "Extracting $($pkg.ZipName) -> $($pkg.ExtractTo) ..."
        # Expand-Zip creates the destination; each package gets its own clean dir
        Expand-Zip $zipPath $pkg.ExtractTo
        New-Item $marker -ItemType File -Force | Out-Null
    }

    Remove-Item $tmp -Recurse -Force -ErrorAction SilentlyContinue

    # Merge all x64 .lib and .dll files into a single directory.
    # sdl2-sys on MSVC requires all four .lib files to be in the same SDL2_LIB_DIR.
    $mergedLib = Join-Path $SDL2_DEV_DIR_LOCAL 'merged\lib\x64'
    $mergedDll = Join-Path $SDL2_DEV_DIR_LOCAL 'merged\dll\x64'
    New-Item $mergedLib -ItemType Directory -Force | Out-Null
    New-Item $mergedDll -ItemType Directory -Force | Out-Null

    foreach ($pkg in $pkgs) {
        # Recurse into the extracted folder and find all x64 .lib / .dll files.
        # The VC zips always have a lib\x64\ subfolder somewhere inside.
        Get-ChildItem $pkg.ExtractTo -Recurse -Filter '*.lib' |
            Where-Object { $_.DirectoryName -like '*\x64' } |
            ForEach-Object { Copy-Item $_.FullName $mergedLib -Force }

        Get-ChildItem $pkg.ExtractTo -Recurse -Filter '*.dll' |
            Where-Object { $_.DirectoryName -like '*\x64' } |
            ForEach-Object { Copy-Item $_.FullName $mergedDll -Force }
    }

    # Verify the four critical .lib files are present before proceeding
    foreach ($lib in @('SDL2.lib', 'SDL2_image.lib', 'SDL2_mixer.lib', 'SDL2_ttf.lib')) {
        $p = Join-Path $mergedLib $lib
        if (-not (Test-Path $p)) {
            throw "Expected '$lib' not found in merged lib dir: $mergedLib`nExtraction may have failed. Delete sdl2-dev\ and try again."
        }
    }

    $script:SDL2_LIB_DIR = $mergedLib
    $script:SDL2_DLL_DIR = $mergedDll

    Write-Info ".lib dir: $mergedLib"
    Write-Info ".dll dir: $mergedDll"
    Get-ChildItem $mergedLib -Filter '*.lib' | ForEach-Object { Write-Info "  lib: $($_.Name)" }
    Get-ChildItem $mergedDll -Filter '*.dll' | ForEach-Object { Write-Info "  dll: $($_.Name)" }
}

# Helper: find a subdirectory pattern inside a dev root
function Find-SDL2SubDir([string]$root, [string]$subpath) {
    # Try direct
    $direct = Join-Path $root $subpath
    if (Test-Path $direct) { return $direct }
    # Try one level deep (e.g. SDL2-2.x.x\lib\x64)
    foreach ($child in Get-ChildItem $root -Directory -ErrorAction SilentlyContinue) {
        $candidate = Join-Path $child.FullName $subpath
        if (Test-Path $candidate) { return $candidate }
    }
    throw "Could not find '$subpath' under '$root'"
}

# ─── Step 1: Release build ────────────────────────────────────────────────────

function Build-Release {
    Write-Step "Building release binary..."
    Require-Command 'cargo'

    # sdl2-sys 0.37 does not read SDL2_LIB_DIR. On MSVC the linker finds .lib
    # files via the LIB environment variable or via rustflags -L.
    # We write a temporary .cargo/config.toml that adds our merged lib dir as a
    # native search path. This is the cleanest cross-session solution and does
    # not pollute the global toolchain config.
    $cargoConfigDir = Join-Path $SCRIPT_DIR '.cargo'
    $cargoConfigFile = Join-Path $cargoConfigDir 'config.toml'
    New-Item $cargoConfigDir -ItemType Directory -Force | Out-Null

    # Escape backslashes for TOML (use forward slashes — Rust accepts them on Windows)
    $libDirToml = $script:SDL2_LIB_DIR -replace '\\', '/'

    $configContent = @"
# Auto-generated by package.ps1 for Windows SDL2 linking.
# Safe to commit — the path is relative-ish via the sdl2-dev\ folder.
[target.x86_64-pc-windows-msvc]
rustflags = ["-L", "$libDirToml"]
"@
    Set-Content -Path $cargoConfigFile -Value $configContent -Encoding UTF8
    Write-Info "Wrote .cargo\config.toml with SDL2 lib path: $libDirToml"

    # Also set LIB for belt-and-suspenders (some linker invocations bypass rustflags)
    $existingLib = $env:LIB
    if ($existingLib) {
        $env:LIB = "$($script:SDL2_LIB_DIR);$existingLib"
    } else {
        $env:LIB = $script:SDL2_LIB_DIR
    }
    Write-Info "LIB = $env:LIB"

    Push-Location $SCRIPT_DIR
    try {
        cargo build --release
        if ($LASTEXITCODE -ne 0) { throw "cargo build failed (exit $LASTEXITCODE)" }
    } finally {
        Pop-Location
        # Restore LIB
        $env:LIB = $existingLib
    }

    if (-not (Test-Path $RELEASE_BIN)) {
        throw "Expected binary not found after build: $RELEASE_BIN"
    }
    Write-Info "Binary: $RELEASE_BIN"
}

# ─── Step 2: Staging area ─────────────────────────────────────────────────────

function New-StagingDir([string]$stagingDir) {
    Write-Step "Creating staging directory..."

    if (Test-Path $stagingDir) { Remove-Item $stagingDir -Recurse -Force }
    New-Item $stagingDir -ItemType Directory | Out-Null

    # Executable
    Copy-Item $RELEASE_BIN (Join-Path $stagingDir "$APP_NAME.exe")
    Write-Info "Copied $APP_NAME.exe"

    # Assets
    $srcAssets = Join-Path $SCRIPT_DIR 'assets'
    if (-not (Test-Path $srcAssets)) { throw "assets\ not found at $srcAssets" }
    Copy-Item $srcAssets (Join-Path $stagingDir 'assets') -Recurse
    Write-Info "Copied assets\"

    # Runtime SDL2 DLLs
    Write-Info "Copying SDL2 runtime DLLs..."
    $missing = @()
    foreach ($dll in $SDL2_RUNTIME_DLLS) {
        $src = Join-Path $script:SDL2_DLL_DIR $dll
        if (Test-Path $src) {
            Copy-Item $src (Join-Path $stagingDir $dll)
            Write-Info "  Bundled: $dll"
        } else {
            $missing += $dll
            Write-Warn "$dll not found in $($script:SDL2_DLL_DIR)"
        }
    }
    if ($missing.Count -gt 0) {
        Write-Warn "Some DLLs missing: $($missing -join ', ') — the package may not run on clean machines."
    }

    # README / LICENSE
    foreach ($extra in @('README.md', 'LICENSE')) {
        $p = Join-Path $SCRIPT_DIR $extra
        if (Test-Path $p) {
            Copy-Item $p (Join-Path $stagingDir $extra)
            Write-Info "Copied $extra"
        }
    }
}

# ─── Step 3: Portable ZIP ─────────────────────────────────────────────────────

function Build-Portable {
    Write-Step "Building portable ZIP package..."

    $stagingDir = Join-Path $BUILD_DIR 'portable-staging'
    New-StagingDir $stagingDir

    # Wrap everything in a top-level folder inside the zip
    $wrapName = "${APP_NAME}-${APP_VERSION}"
    $wrapDir  = Join-Path $BUILD_DIR $wrapName
    if (Test-Path $wrapDir) { Remove-Item $wrapDir -Recurse -Force }
    Copy-Item $stagingDir $wrapDir -Recurse

    $zipOutput = Join-Path $BUILD_DIR "${APP_NAME}-${APP_VERSION}-windows-x64.zip"
    if (Test-Path $zipOutput) { Remove-Item $zipOutput -Force }
    Compress-Archive -Path $wrapDir -DestinationPath $zipOutput -Force

    Remove-Item $wrapDir -Recurse -Force
    Write-Info "Created: $zipOutput"
    return $zipOutput
}

# ─── Step 4: MSI installer via WiX v4 ────────────────────────────────────────

# Converts a plain-text file to a minimal valid RTF file that WiX can embed
# as the license agreement screen.  RTF special chars ( \ { } ) are escaped.
function ConvertTo-LicenseRtf([string]$txtPath, [string]$rtfPath) {
    $lines = Get-Content $txtPath -Encoding UTF8
    $sb = [System.Text.StringBuilder]::new()
    [void]$sb.AppendLine('{\rtf1\ansi\deff0')
    [void]$sb.AppendLine('{\fonttbl{\f0\fmodern\fcharset0 Courier New;}}')
    [void]$sb.AppendLine('\f0\fs18')
    foreach ($line in $lines) {
        # Escape RTF special characters
        $escaped = $line `
            -replace '\\', '\\\\' `
            -replace '\{', '\{' `
            -replace '\}', '\}'
        [void]$sb.AppendLine($escaped + '\line')
    }
    [void]$sb.AppendLine('}')
    [System.IO.File]::WriteAllText($rtfPath, $sb.ToString(), [System.Text.Encoding]::ASCII)
}

function Build-Msi {
    Write-Step "Building MSI installer (WiX v4)..."

    if (-not (Get-Command 'wix' -ErrorAction SilentlyContinue)) {
        Write-Warn "wix not found on PATH — skipping MSI."
        Write-Warn "Install: dotnet tool install --global wix"
        Write-Warn "Then:    wix extension add WixToolset.UI.wixext"
        return $null
    }

    $stagingDir = Join-Path $BUILD_DIR 'portable-staging'
    if (-not (Test-Path $stagingDir)) { New-StagingDir $stagingDir }

    $wxsPath   = Join-Path $BUILD_DIR "${APP_NAME}.wxs"
    $msiOutput = Join-Path $BUILD_DIR "${APP_NAME}-${APP_VERSION}-windows-x64.msi"

    # Convert LICENSE plain text -> RTF for the WiX license screen
    $licenseRtf = Join-Path $BUILD_DIR 'license.rtf'
    $licenseTxt = Join-Path $SCRIPT_DIR 'LICENSE'
    if (Test-Path $licenseTxt) {
        ConvertTo-LicenseRtf $licenseTxt $licenseRtf
        Write-Info "Converted LICENSE -> license.rtf"
    } else {
        $licenseRtf = $null
        Write-Warn "LICENSE file not found — installer will show default placeholder text."
    }

    # Locate the .ico for Add/Remove Programs and shortcut icons.
    # Priority: assets\icon.ico (pre-generated) > generate from assets\icon.png via magick
    $icoPath  = Join-Path $BUILD_DIR "${APP_NAME}.ico"
    $icoReady = $false
    $srcIco   = Join-Path $SCRIPT_DIR 'assets\icon.ico'
    $srcPng   = Join-Path $SCRIPT_DIR 'assets\icon.png'

    if (Test-Path $srcIco) {
        Copy-Item $srcIco $icoPath -Force
        $icoReady = $true
        Write-Info "Using icon: assets\icon.ico"
    } elseif (Test-Path $srcPng) {
        $magickCmd = Get-Command 'magick' -ErrorAction SilentlyContinue
        if ($magickCmd) {
            & $magickCmd.Source $srcPng -define icon:auto-resize=256,128,64,48,32,16 $icoPath
            if ($LASTEXITCODE -eq 0 -and (Test-Path $icoPath)) {
                $icoReady = $true
                Write-Info "Converted icon.png -> icon.ico via magick"
            }
        }
        if (-not $icoReady) {
            Write-Warn "Could not create icon.ico — shortcuts will have no custom icon."
        }
    }

    # Build component list by enumerating every file in the staging dir.
    # We write XML to a plain text file rather than building it in PowerShell
    # string expressions, to avoid the parser choking on < > in XML.
    $stageFiles = Get-ChildItem $stagingDir -Recurse -File
    $dirSet     = [System.Collections.Generic.HashSet[string]]::new()

    # Collect data rows first, then write XML lines to lists
    $compRefLines  = [System.Collections.Generic.List[string]]::new()
    $compLines     = [System.Collections.Generic.List[string]]::new()

    foreach ($file in $stageFiles) {
        $rel    = $file.FullName.Substring($stagingDir.Length + 1)
        $compId = 'Comp_' + ($rel -replace '[\\\/\.\-\s]', '_')
        $fileId = 'File_' + ($compId -replace '^Comp_', '')
        $guid   = [System.Guid]::NewGuid().ToString().ToUpper()
        $subdir = Split-Path $rel -Parent
        $dirRef = if ($subdir) { 'dir_' + ($subdir -replace '[\\\/\.\-\s]', '_') } else { 'INSTALLFOLDER' }

        if ($subdir) { [void]$dirSet.Add($subdir) }

        $compRefLines.Add('      <ComponentRef Id="' + $compId + '" />')
        $compLines.Add('    <Component Id="' + $compId + '" Guid="' + $guid + '" Directory="' + $dirRef + '">')
        $compLines.Add('      <File Id="' + $fileId + '" Source="' + $file.FullName + '" KeyPath="yes" />')
        $compLines.Add('    </Component>')
    }

    # Build nested Directory XML for WiX.
    # WiX requires child directories to be nested inside parent Directory elements,
    # not listed as flat siblings.  We build a tree then serialize it.

    # Collect all unique directory paths, sorted shallowest-first
    $allDirs = $dirSet | Sort-Object { ($_ -split '[\\\/]').Count }, { $_ }

    # Recursive function to emit nested <Directory> elements
    function Get-DirXml([string]$parentPath, [string[]]$allDirs, [int]$depth) {
        $indent = '        ' + ('  ' * $depth)
        $lines  = [System.Collections.Generic.List[string]]::new()
        # Find direct children of parentPath
        $children = $allDirs | Where-Object {
            $parts = $_ -split '[\\\/]'
            if ($parentPath -eq '') {
                $parts.Count -eq 1
            } else {
                $_ -like "$parentPath\*" -and ($_ -split '[\\\/]').Count -eq ($parentPath -split '[\\\/]').Count + 1
            }
        }
        foreach ($child in $children) {
            $parts = $child -split '[\\\/]'
            $dirId = 'dir_' + ($child -replace '[\\\/\.\-\s]', '_')
            $name  = $parts[-1]
            # Recursively find grandchildren
            $grandChildren = $allDirs | Where-Object {
                $_ -like "$child\*" -and ($_ -split '[\\\/]').Count -eq $parts.Count + 1
            }
            if ($grandChildren) {
                $lines.Add($indent + '<Directory Id="' + $dirId + '" Name="' + $name + '">')
                foreach ($sub in (Get-DirXml $child $allDirs ($depth + 1))) { $lines.Add($sub) }
                $lines.Add($indent + '</Directory>')
            } else {
                $lines.Add($indent + '<Directory Id="' + $dirId + '" Name="' + $name + '" />')
            }
        }
        return $lines
    }

    $dirLines = Get-DirXml '' $allDirs 0

    # Build the full .wxs content as a list of lines, then join
    $lines = [System.Collections.Generic.List[string]]::new()
    $lines.Add('<?xml version="1.0" encoding="utf-8"?>')
    $lines.Add('<!-- Auto-generated by package.ps1 -->')
    $lines.Add('<Wix xmlns="http://wixtoolset.org/schemas/v4/wxs"')
    $lines.Add('     xmlns:ui="http://wixtoolset.org/schemas/v4/wxs/ui">')
    $lines.Add('  <Package')
    $lines.Add('    Name="'           + $APP_DISPLAY + '"')
    $lines.Add('    Manufacturer="'   + $APP_MANUFACTURER + '"')
    $lines.Add('    Version="'        + $APP_VERSION + '"')
    $lines.Add('    UpgradeCode="'    + $UPGRADE_GUID + '"')
    $lines.Add('    Scope="perMachine"')
    $lines.Add('    Compressed="yes"')
    $lines.Add('    InstallerVersion="500">')
    $lines.Add('')
    $lines.Add('    <MajorUpgrade DowngradeErrorMessage="A newer version of ' + $APP_DISPLAY + ' is already installed." />')
    $lines.Add('    <MediaTemplate EmbedCab="yes" />')
    if ($licenseRtf) {
        $lines.Add('    <WixVariable Id="WixUILicenseRtf" Value="' + $licenseRtf + '" />')
    }
    if ($icoReady) {
        $lines.Add('    <Icon Id="AppIcon" SourceFile="' + $icoPath + '" />')
        $lines.Add('    <Property Id="ARPPRODUCTICON" Value="AppIcon" />')
    }
    $lines.Add('    <Property Id="ARPHELPLINK"     Value="https://github.com/gcclinux/xmahjong" />')
    $lines.Add('    <Property Id="ARPURLINFOABOUT" Value="https://github.com/gcclinux/xmahjong" />')
    $lines.Add('    <Property Id="ARPCOMMENTS"     Value="' + $APP_DESCRIPTION + '" />')
    $lines.Add('')
    $lines.Add('    <StandardDirectory Id="ProgramFiles64Folder">')
    $lines.Add('      <Directory Id="INSTALLFOLDER" Name="' + $APP_DISPLAY + '">')
    foreach ($dl in $dirLines) { $lines.Add($dl) }
    $lines.Add('      </Directory>')
    $lines.Add('    </StandardDirectory>')
    $lines.Add('')
    $lines.Add('    <StandardDirectory Id="ProgramMenuFolder">')
    $lines.Add('      <Directory Id="AppMenuFolder" Name="' + $APP_DISPLAY + '" />')
    $lines.Add('    </StandardDirectory>')
    $lines.Add('    <StandardDirectory Id="DesktopFolder" />')
    $lines.Add('')
    $lines.Add('    <ComponentGroup Id="ProductComponents" Directory="INSTALLFOLDER">')
    foreach ($cl in $compLines) { $lines.Add($cl) }
    $lines.Add('    </ComponentGroup>')
    $lines.Add('')
    $lines.Add('    <Component Id="StartMenuShortcut" Directory="AppMenuFolder" Guid="E1A2B3C4-D5E6-F7A8-B9C0-D1E2F3A4B5C6">')
    $lines.Add('      <Shortcut Id="AppStartMenu" Name="' + $APP_DISPLAY + '" Description="' + $APP_DESCRIPTION + '"')
    $lines.Add('                Target="[INSTALLFOLDER]' + $APP_NAME + '.exe" WorkingDirectory="INSTALLFOLDER"')
    if ($icoReady) {
        $lines.Add('                Icon="AppIcon" />')
    } else {
        $lines.Add('                />')
    }
    $lines.Add('      <RemoveFolder Id="CleanUpStartMenu" On="uninstall" />')
    $lines.Add('      <RegistryValue Root="HKCU" Key="Software\' + $APP_MANUFACTURER + '\' + $APP_DISPLAY + '"')
    $lines.Add('                     Name="StartMenuInstalled" Type="integer" Value="1" KeyPath="yes" />')
    $lines.Add('    </Component>')
    $lines.Add('')
    $lines.Add('    <Component Id="DesktopShortcut" Directory="DesktopFolder" Guid="F2B3C4D5-E6F7-A8B9-C0D1-E2F3A4B5C6D7">')
    $lines.Add('      <Shortcut Id="AppDesktop" Name="' + $APP_DISPLAY + '" Description="' + $APP_DESCRIPTION + '"')
    $lines.Add('                Target="[INSTALLFOLDER]' + $APP_NAME + '.exe" WorkingDirectory="INSTALLFOLDER"')
    if ($icoReady) {
        $lines.Add('                Icon="AppIcon" />')
    } else {
        $lines.Add('                />')
    }
    $lines.Add('      <RegistryValue Root="HKCU" Key="Software\' + $APP_MANUFACTURER + '\' + $APP_DISPLAY + '"')
    $lines.Add('                     Name="DesktopInstalled" Type="integer" Value="1" KeyPath="yes" />')
    $lines.Add('    </Component>')
    $lines.Add('')
    $lines.Add('    <Feature Id="ProductFeature" Title="' + $APP_DISPLAY + '" Level="1">')
    $lines.Add('      <ComponentGroupRef Id="ProductComponents" />')
    $lines.Add('      <ComponentRef Id="StartMenuShortcut" />')
    $lines.Add('      <ComponentRef Id="DesktopShortcut" />')
    $lines.Add('    </Feature>')
    $lines.Add('')
    $lines.Add('    <ui:WixUI Id="WixUI_Minimal" />')
    $lines.Add('')
    $lines.Add('  </Package>')
    $lines.Add('</Wix>')

    [System.IO.File]::WriteAllLines($wxsPath, $lines, [System.Text.Encoding]::UTF8)
    Write-Info "Generated: $wxsPath"

    if (Test-Path $msiOutput) {
        Remove-Item $msiOutput -Force -ErrorAction SilentlyContinue
        Start-Sleep -Milliseconds 500
    }
    wix build $wxsPath -o $msiOutput -ext WixToolset.UI.wixext
    if ($LASTEXITCODE -ne 0) { throw "wix build failed (exit $LASTEXITCODE)" }

    Write-Info "Created: $msiOutput"
    return $msiOutput
}

# ─── Step 5: MSIX package ────────────────────────────────────────────────────
#
# Produces xmahjong-VERSION-windows-x64.msix suitable for:
#   - Sideloading on Windows 10/11 (signed with a self-signed cert)
#   - Microsoft Store submission (signed with your Partner Center certificate)
#
# Requires makeappx.exe and signtool.exe from the Windows SDK.

function Find-WindowsSdkTool([string]$toolName) {
    # Search installed Windows SDK versions, newest first
    $sdkRoot = 'C:\Program Files (x86)\Windows Kits\10\bin'
    if (Test-Path $sdkRoot) {
        $found = Get-ChildItem $sdkRoot -Directory |
            Sort-Object Name -Descending |
            ForEach-Object { Join-Path $_.FullName "x64\$toolName" } |
            Where-Object { Test-Path $_ } |
            Select-Object -First 1
        if ($found) { return $found }
    }
    # Fallback: try PATH
    $onPath = Get-Command $toolName -ErrorAction SilentlyContinue
    if ($onPath) { return $onPath.Source }
    return $null
}

function Build-Msix {
    Write-Step "Building MSIX package..."

    # Locate required SDK tools
    $makeappx = Find-WindowsSdkTool 'makeappx.exe'
    $signtool = Find-WindowsSdkTool 'signtool.exe'

    if (-not $makeappx) {
        Write-Warn "makeappx.exe not found — skipping MSIX."
        Write-Warn "Install the Windows SDK (included with Visual Studio or standalone):"
        Write-Warn "https://developer.microsoft.com/windows/downloads/windows-sdk/"
        return $null
    }
    Write-Info "makeappx : $makeappx"
    if ($signtool) {
        Write-Info "signtool : $signtool"
    } else {
        Write-Warn "signtool.exe not found — package will be built but not signed."
        Write-Warn "Unsigned MSIX can only be installed if developer mode is enabled."
    }

    # Re-use the portable staging dir (same files the ZIP uses)
    $stagingDir = Join-Path $BUILD_DIR 'portable-staging'
    if (-not (Test-Path $stagingDir)) { New-StagingDir $stagingDir }

    # MSIX layout goes into its own directory
    $msixStage = Join-Path $BUILD_DIR 'msix-staging'
    if (Test-Path $msixStage) { Remove-Item $msixStage -Recurse -Force }
    New-Item $msixStage -ItemType Directory | Out-Null

    # Copy all app files into the MSIX layout root
    Copy-Item (Join-Path $stagingDir '*') $msixStage -Recurse -Force
    Write-Info "Copied app files to MSIX staging dir"

    # ── Prepare icons ──────────────────────────────────────────────────────
    # MSIX requires PNG assets at specific sizes in an Assets\ subfolder.
    # We scale from assets\icon.png using System.Drawing (no extra tools needed).
    $assetsDir = Join-Path $msixStage 'Assets'
    New-Item $assetsDir -ItemType Directory -Force | Out-Null

    $srcIcon = Join-Path $SCRIPT_DIR 'assets\icon.png'
    $iconSizes = @(
        @{ Name = 'Square44x44Logo.png';     Size = 44  },
        @{ Name = 'Square150x150Logo.png';   Size = 150 },
        @{ Name = 'Wide310x150Logo.png';     W = 310; H = 150 },
        @{ Name = 'Square310x310Logo.png';   Size = 310 },
        @{ Name = 'StoreLogo.png';           Size = 50  }
    )

    Add-Type -AssemblyName System.Drawing

    foreach ($icon in $iconSizes) {
        $destPath = Join-Path $assetsDir $icon.Name
        $w = if ($icon.ContainsKey('W')) { $icon.W } else { $icon.Size }
        $h = if ($icon.ContainsKey('H')) { $icon.H } else { $icon.Size }

        if (Test-Path $srcIcon) {
            try {
                $src = [System.Drawing.Image]::FromFile($srcIcon)
                $bmp = New-Object System.Drawing.Bitmap($w, $h)
                $g   = [System.Drawing.Graphics]::FromImage($bmp)
                $g.InterpolationMode = [System.Drawing.Drawing2D.InterpolationMode]::HighQualityBicubic
                $g.DrawImage($src, 0, 0, $w, $h)
                $g.Dispose()
                $src.Dispose()
                $bmp.Save($destPath, [System.Drawing.Imaging.ImageFormat]::Png)
                $bmp.Dispose()
                Write-Info "  Icon: $($icon.Name) (${w}x${h})"
            } catch {
                # If System.Drawing fails (e.g. missing GDI+), copy the source as-is
                Copy-Item $srcIcon $destPath -Force
                Write-Warn "  Could not resize $($icon.Name), using original icon."
            }
        } else {
            # Create a plain coloured placeholder PNG (dark green, matches the game)
            $bmp = New-Object System.Drawing.Bitmap($w, $h)
            $g   = [System.Drawing.Graphics]::FromImage($bmp)
            $g.Clear([System.Drawing.Color]::FromArgb(34, 85, 34))
            $g.Dispose()
            $bmp.Save($destPath, [System.Drawing.Imaging.ImageFormat]::Png)
            $bmp.Dispose()
            Write-Warn "  No source icon found — placeholder used for $($icon.Name)"
        }
    }

    # ── AppxManifest.xml ───────────────────────────────────────────────────
    $manifestPath = Join-Path $msixStage 'AppxManifest.xml'
    $manifestLines = [System.Collections.Generic.List[string]]::new()
    $manifestLines.Add('<?xml version="1.0" encoding="utf-8"?>')
    $manifestLines.Add('<Package')
    $manifestLines.Add('  xmlns="http://schemas.microsoft.com/appx/manifest/foundation/windows10"')
    $manifestLines.Add('  xmlns:uap="http://schemas.microsoft.com/appx/manifest/uap/windows10"')
    $manifestLines.Add('  xmlns:rescap="http://schemas.microsoft.com/appx/manifest/foundation/windows10/restrictedcapabilities"')
    $manifestLines.Add('  IgnorableNamespaces="uap rescap">')
    $manifestLines.Add('')
    $manifestLines.Add('  <Identity')
    $manifestLines.Add('    Name="' + $MSIX_APP_ID + '"')
    $manifestLines.Add('    Publisher="' + $MSIX_PUBLISHER + '"')
    $manifestLines.Add('    Version="' + $MSIX_VERSION + '"')
    $manifestLines.Add('    ProcessorArchitecture="x64" />')
    $manifestLines.Add('')
    $manifestLines.Add('  <Properties>')
    $manifestLines.Add('    <DisplayName>' + $APP_DISPLAY + '</DisplayName>')
    $manifestLines.Add('    <PublisherDisplayName>' + $MSIX_PUBLISHER_DISPLAY + '</PublisherDisplayName>')
    $manifestLines.Add('    <Description>' + $MSIX_DESCRIPTION + '</Description>')
    $manifestLines.Add('    <Logo>Assets\StoreLogo.png</Logo>')
    $manifestLines.Add('  </Properties>')
    $manifestLines.Add('')
    $manifestLines.Add('  <Dependencies>')
    $manifestLines.Add('    <TargetDeviceFamily Name="Windows.Desktop" MinVersion="10.0.17763.0" MaxVersionTested="10.0.22621.0" />')
    $manifestLines.Add('  </Dependencies>')
    $manifestLines.Add('')
    $manifestLines.Add('  <Resources>')
    $manifestLines.Add('    <Resource Language="en-US" />')
    $manifestLines.Add('  </Resources>')
    $manifestLines.Add('')
    $manifestLines.Add('  <Applications>')
    $manifestLines.Add('    <Application Id="App" Executable="' + $APP_NAME + '.exe" EntryPoint="Windows.FullTrustApplication">')
    $manifestLines.Add('      <uap:VisualElements')
    $manifestLines.Add('        DisplayName="' + $APP_DISPLAY + '"')
    $manifestLines.Add('        Description="' + $MSIX_DESCRIPTION + '"')
    $manifestLines.Add('        Square150x150Logo="Assets\Square150x150Logo.png"')
    $manifestLines.Add('        Square44x44Logo="Assets\Square44x44Logo.png"')
    $manifestLines.Add('        BackgroundColor="#225522">')
    $manifestLines.Add('        <uap:DefaultTile Wide310x150Logo="Assets\Wide310x150Logo.png"')
    $manifestLines.Add('                         Square310x310Logo="Assets\Square310x310Logo.png" />')
    $manifestLines.Add('        <uap:SplashScreen Image="Assets\Square150x150Logo.png" />')
    $manifestLines.Add('      </uap:VisualElements>')
    $manifestLines.Add('    </Application>')
    $manifestLines.Add('  </Applications>')
    $manifestLines.Add('')
    $manifestLines.Add('  <Capabilities>')
    $manifestLines.Add('    <rescap:Capability Name="runFullTrust" />')
    $manifestLines.Add('  </Capabilities>')
    $manifestLines.Add('')
    $manifestLines.Add('</Package>')

    [System.IO.File]::WriteAllLines($manifestPath, $manifestLines, [System.Text.Encoding]::UTF8)
    Write-Info "Generated: AppxManifest.xml"
    Write-Info "  Identity.Name      : $MSIX_APP_ID"
    Write-Info "  Identity.Publisher : $MSIX_PUBLISHER"
    Write-Info "  PublisherDisplay   : $MSIX_PUBLISHER_DISPLAY"
    Write-Info "  Version            : $MSIX_VERSION"

    # ── Pack with makeappx ────────────────────────────────────────────────
    $msixOutput = Join-Path $BUILD_DIR "${APP_NAME}-${APP_VERSION}-windows-x64.msix"
    if (Test-Path $msixOutput) { Remove-Item $msixOutput -Force }

    & $makeappx pack /d $msixStage /p $msixOutput /nv /o 2>&1 | ForEach-Object { Write-Info $_ }
    if ($LASTEXITCODE -ne 0) { throw "makeappx failed (exit $LASTEXITCODE)" }
    Write-Info "Packed: $msixOutput"

    # ── Sign the package ──────────────────────────────────────────────────
    if ($signtool) {
        # Check if a PFX cert is provided via environment variable.
        # $env:MSIX_CERT_PFX = path to .pfx file
        # $env:MSIX_CERT_PASS = password (optional, leave unset if no password)
        if ($env:MSIX_CERT_PFX -and (Test-Path $env:MSIX_CERT_PFX)) {
            Write-Info "Signing with certificate: $env:MSIX_CERT_PFX"
            $signArgs = @('sign', '/fd', 'SHA256', '/f', $env:MSIX_CERT_PFX)
            if ($env:MSIX_CERT_PASS) { $signArgs += @('/p', $env:MSIX_CERT_PASS) }
            $signArgs += $msixOutput
            & $signtool @signArgs 2>&1 | ForEach-Object { Write-Info $_ }
            if ($LASTEXITCODE -ne 0) {
                Write-Warn "signtool failed — package is unsigned. Check certificate compatibility."
            } else {
                Write-Info "Package signed successfully."
            }
        } else {
            # Generate a temporary self-signed certificate for sideloading.
            # The Subject must match MSIX_PUBLISHER exactly.
            Write-Info "No MSIX_CERT_PFX set — generating self-signed certificate for sideloading..."
            $certSubject = $MSIX_PUBLISHER
            $pfxPath     = Join-Path $BUILD_DIR 'xmahjong-dev.pfx'
            # Generate a random password — only needed to satisfy Export-PfxCertificate's
            # API requirement. The PFX is used immediately and the password is never stored.
            $pfxPassword = ConvertTo-SecureString -String ([System.Guid]::NewGuid().ToString()) -Force -AsPlainText

            try {
                $cert = New-SelfSignedCertificate `
                    -Subject $certSubject `
                    -Type CodeSigningCert `
                    -CertStoreLocation 'Cert:\CurrentUser\My' `
                    -HashAlgorithm SHA256 `
                    -NotAfter (Get-Date).AddYears(2)

                Export-PfxCertificate -Cert $cert -FilePath $pfxPath -Password $pfxPassword | Out-Null
                Write-Info "Self-signed cert exported: $pfxPath"
                Write-Warn "This cert is for DEV/SIDELOAD only."
                Write-Warn "To install: first trust the cert via 'Install-Package' or Settings > Developer Mode."

                # Extract the plain-text password from the SecureString to pass to signtool
                $pfxPlain = [System.Runtime.InteropServices.Marshal]::PtrToStringAuto(
                    [System.Runtime.InteropServices.Marshal]::SecureStringToBSTR($pfxPassword))
                & $signtool sign /fd SHA256 /f $pfxPath /p $pfxPlain $msixOutput 2>&1 |
                    ForEach-Object { Write-Info $_ }
                $pfxPlain = $null  # clear from memory immediately
                if ($LASTEXITCODE -ne 0) {
                    Write-Warn "Signing failed — distributing unsigned MSIX."
                } else {
                    Write-Info "Package signed with self-signed cert."
                }
            } catch {
                Write-Warn "Could not create self-signed certificate: $_"
                Write-Warn "MSIX is unsigned — enable Developer Mode to sideload it."
            }
        }
    }

    Write-Info "Created: $msixOutput"
    return $msixOutput
}

# ─── Main ─────────────────────────────────────────────────────────────────────

function Main {
    Write-Host ""
    Write-Host "  xMahjong Windows Packager  v$APP_VERSION" -ForegroundColor Green
    Write-Host "  Action: $Action" -ForegroundColor Green
    Write-Host ""

    New-Item $BUILD_DIR -ItemType Directory -Force | Out-Null

    # Always set up SDL2 dev libs first (needed for the build step)
    Setup-SDL2Dev
    Build-Release

    $results = @()
    switch ($Action) {
        'portable' { $results += Build-Portable }
        'msi'      { Build-Portable | Out-Null; $results += Build-Msi }
        'msix'     { Build-Portable | Out-Null; $results += Build-Msix }
        'all'      { $results += Build-Portable; $results += Build-Msi; $results += Build-Msix }
    }

    Write-Host ""
    Write-Host "==> Done! Output files:" -ForegroundColor Green
    foreach ($r in $results) { if ($r) { Write-Host "    $r" } }
    Write-Host ""
    Write-Host "    All packages in: $BUILD_DIR" -ForegroundColor Green
    Get-ChildItem $BUILD_DIR -File |
        Where-Object { $_.Extension -in '.zip', '.msi', '.msix' } |
        Format-Table Name, @{L='Size'; E={ '{0:N0} KB' -f ($_.Length / 1KB) }} -AutoSize
}

Main
