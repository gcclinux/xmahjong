# Usage: .\bump_version.ps1 0.2.0
# Updates the version in both `release` and `Cargo.toml`.

param(
    [Parameter(Mandatory=$false, Position=0)]
    [string]$NewVersion
)

if (-not $NewVersion) {
    Write-Host "Usage: .\bump_version.ps1 <new_version>"
    Write-Host "Example: .\bump_version.ps1 0.2.0"
    exit 1
}

$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path

# Update the release file
Set-Content -Path (Join-Path $ScriptDir "release") -Value $NewVersion -NoNewline

# Update Cargo.toml version field
$CargoPath = Join-Path $ScriptDir "Cargo.toml"
$Content = Get-Content $CargoPath -Raw
$Content = $Content -replace '(?m)^version = ".*"', "version = `"$NewVersion`""
Set-Content -Path $CargoPath -Value $Content -NoNewline

Write-Host "Version updated to $NewVersion in both release and Cargo.toml"
