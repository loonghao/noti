<#
.SYNOPSIS
    Install noti CLI on Windows.

.DESCRIPTION
    Downloads the latest (or a pinned) release binary of noti from GitHub
    and installs it to a local directory.

.PARAMETER Version
    Version to install (default: latest). Accepts "v0.2.0" or "0.2.0".

.PARAMETER InstallDir
    Where to place the binary. Default: $env:USERPROFILE\.noti\bin

.PARAMETER Repository
    GitHub owner/repo. Default: loonghao/wecom-bot-cli

.EXAMPLE
    irm https://raw.githubusercontent.com/loonghao/wecom-bot-cli/main/scripts/install.ps1 | iex
    .\install.ps1 -Version v0.2.0
#>

param(
    [string]$Version    = ($env:NOTI_INSTALL_VERSION    ?? "latest"),
    [string]$InstallDir = ($env:NOTI_INSTALL_DIR        ?? "$env:USERPROFILE\.noti\bin"),
    [string]$Repository = ($env:NOTI_INSTALL_REPOSITORY ?? "loonghao/wecom-bot-cli")
)

$ErrorActionPreference = "Stop"

# ---------- detect architecture -----------------------------------------------

$arch = [System.Runtime.InteropServices.RuntimeInformation]::OSArchitecture
switch ($arch) {
    "X64"   { $target = "x86_64-pc-windows-msvc" }
    "Arm64" { $target = "aarch64-pc-windows-msvc" }
    default { throw "Unsupported architecture: $arch" }
}

# ---------- resolve download URL ----------------------------------------------

if ($Version -eq "latest") {
    $url = "https://github.com/$Repository/releases/latest/download/noti-$target.zip"
} else {
    if (-not $Version.StartsWith("v")) { $Version = "v$Version" }
    $url = "https://github.com/$Repository/releases/download/$Version/noti-$Version-$target.zip"
}

Write-Host "-> Downloading noti ($Version) for $target..."
Write-Host "   $url"

# ---------- download & extract ------------------------------------------------

$tmp = Join-Path ([System.IO.Path]::GetTempPath()) ([System.IO.Path]::GetRandomFileName())
New-Item -ItemType Directory -Force -Path $tmp | Out-Null

try {
    $zip = Join-Path $tmp "noti.zip"
    Invoke-WebRequest -Uri $url -OutFile $zip -UseBasicParsing

    Expand-Archive -Path $zip -DestinationPath $tmp -Force

    New-Item -ItemType Directory -Force -Path $InstallDir | Out-Null
    Copy-Item -Path (Join-Path $tmp "noti.exe") -Destination (Join-Path $InstallDir "noti.exe") -Force

    Write-Host "✓ Installed noti to $InstallDir\noti.exe"
} finally {
    Remove-Item -Recurse -Force -Path $tmp -ErrorAction SilentlyContinue
}

# ---------- PATH hint ---------------------------------------------------------

$currentPath = [Environment]::GetEnvironmentVariable("PATH", "User")
if ($currentPath -notlike "*$InstallDir*") {
    Write-Host ""
    Write-Host "⚠  $InstallDir is not in your PATH."
    Write-Host "   Add it with:"
    Write-Host ""
    Write-Host "   `$env:PATH = `"$InstallDir;`$env:PATH`""
    Write-Host ""
    Write-Host "   Or permanently (requires restart):"
    Write-Host "   [Environment]::SetEnvironmentVariable('PATH', `"$InstallDir;`$currentPath`", 'User')"
    Write-Host ""
}
