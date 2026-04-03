# RustyHand installer for Windows
# Usage: iwr -useb https://rustyhand.sh/install.ps1 | iex
#   or:  powershell -c "irm https://rustyhand.sh/install.ps1 | iex"
#
# Flags (via environment variables):
#   $env:RUSTY_HAND_INSTALL_DIR = custom install directory
#   $env:RUSTY_HAND_VERSION     = specific version tag (e.g. "v0.1.0")

$ErrorActionPreference = 'Stop'

$Repo = "ginkida/rustyhand"
$DefaultInstallDir = Join-Path $env:USERPROFILE ".rustyhand\bin"
$InstallDir = if ($env:RUSTY_HAND_INSTALL_DIR) { $env:RUSTY_HAND_INSTALL_DIR } else { $DefaultInstallDir }

function Write-Banner {
    Write-Host ""
    Write-Host "  RustyHand Installer" -ForegroundColor Cyan
    Write-Host "  ===================" -ForegroundColor Cyan
    Write-Host ""
}

function Get-Architecture {
    try {
        $arch = [System.Runtime.InteropServices.RuntimeInformation]::OSArchitecture
    } catch {
        # PowerShell 5.1 fallback
        $arch = $env:PROCESSOR_ARCHITECTURE
    }
    switch ($arch) {
        { $_ -in "X64", "AMD64" }  { return "x86_64" }
        { $_ -in "Arm64", "ARM64" } { return "aarch64" }
        default {
            Write-Host "  Unsupported architecture: $arch" -ForegroundColor Red
            exit 1
        }
    }
}

function Get-LatestVersion {
    if ($env:RUSTY_HAND_VERSION) {
        return $env:RUSTY_HAND_VERSION
    }

    Write-Host "  Fetching latest release..."
    try {
        $release = Invoke-RestMethod -Uri "https://api.github.com/repos/$Repo/releases/latest"
        return $release.tag_name
    }
    catch {
        Write-Host "  Could not determine latest version." -ForegroundColor Red
        Write-Host "  Install from source instead:" -ForegroundColor Yellow
        Write-Host "    cargo install --git https://github.com/$Repo rusty-hand-cli"
        exit 1
    }
}

function Install-RustyHand {
    Write-Banner

    $arch = Get-Architecture
    $version = Get-LatestVersion
    $target = "${arch}-pc-windows-msvc"
    $archive = "rustyhand-${target}.zip"
    $url = "https://github.com/$Repo/releases/download/$version/$archive"
    $checksumUrl = "$url.sha256"

    Write-Host "  Installing RustyHand $version for $target..."

    # Create install directory
    if (-not (Test-Path $InstallDir)) {
        New-Item -ItemType Directory -Path $InstallDir -Force | Out-Null
    }

    # Download to temp
    $tempDir = Join-Path ([System.IO.Path]::GetTempPath()) "rustyhand-install"
    if (Test-Path $tempDir) { Remove-Item -Recurse -Force $tempDir }
    New-Item -ItemType Directory -Path $tempDir -Force | Out-Null

    $archivePath = Join-Path $tempDir $archive
    $checksumPath = Join-Path $tempDir "$archive.sha256"

    try {
        Invoke-WebRequest -Uri $url -OutFile $archivePath -UseBasicParsing
    }
    catch {
        Write-Host "  Download failed. The release may not exist for your platform." -ForegroundColor Red
        Write-Host "  Install from source instead:" -ForegroundColor Yellow
        Write-Host "    cargo install --git https://github.com/$Repo rusty-hand-cli"
        Remove-Item -Recurse -Force $tempDir -ErrorAction SilentlyContinue
        exit 1
    }

    # Verify checksum if available
    $checksumDownloaded = $false
    try {
        Invoke-WebRequest -Uri $checksumUrl -OutFile $checksumPath -UseBasicParsing
        $checksumDownloaded = $true
    }
    catch {
        Write-Host "  Checksum file not available, skipping verification." -ForegroundColor Yellow
    }
    if ($checksumDownloaded) {
        $expectedHash = (Get-Content $checksumPath -Raw).Split(" ")[0].Trim().ToLower()
        $actualHash = (Get-FileHash $archivePath -Algorithm SHA256).Hash.ToLower()
        if ($expectedHash -ne $actualHash) {
            Write-Host "  Checksum verification FAILED!" -ForegroundColor Red
            Write-Host "    Expected: $expectedHash" -ForegroundColor Red
            Write-Host "    Got:      $actualHash" -ForegroundColor Red
            Remove-Item -Recurse -Force $tempDir -ErrorAction SilentlyContinue
            exit 1
        }
        Write-Host "  Checksum verified." -ForegroundColor Green
    }

    # Extract
    Expand-Archive -Path $archivePath -DestinationPath $tempDir -Force
    $exePath = Join-Path $tempDir "rustyhand.exe"
    if (-not (Test-Path $exePath)) {
        # May be nested in a directory
        $found = Get-ChildItem -Path $tempDir -Filter "rustyhand.exe" -Recurse | Select-Object -First 1
        if ($found) {
            $exePath = $found.FullName
        }
        else {
            Write-Host "  Could not find rustyhand.exe in archive." -ForegroundColor Red
            Remove-Item -Recurse -Force $tempDir -ErrorAction SilentlyContinue
            exit 1
        }
    }

    # Install
    Copy-Item -Path $exePath -Destination (Join-Path $InstallDir "rustyhand.exe") -Force

    # Clean up temp
    Remove-Item -Recurse -Force $tempDir -ErrorAction SilentlyContinue

    # Add to user PATH if not already present
    $currentPath = [Environment]::GetEnvironmentVariable("Path", "User")
    if ($currentPath -notlike "*$InstallDir*") {
        [Environment]::SetEnvironmentVariable("Path", "$InstallDir;$currentPath", "User")
        Write-Host "  Added $InstallDir to user PATH." -ForegroundColor Green
        Write-Host "  Restart your terminal for PATH changes to take effect." -ForegroundColor Yellow
    }

    # Verify
    $installedExe = Join-Path $InstallDir "rustyhand.exe"
    if (Test-Path $installedExe) {
        try {
            $versionOutput = & $installedExe --version 2>&1
            Write-Host ""
            Write-Host "  RustyHand installed successfully! ($versionOutput)" -ForegroundColor Green
        }
        catch {
            Write-Host ""
            Write-Host "  RustyHand binary installed to $installedExe" -ForegroundColor Green
        }
    }

    Write-Host ""
    Write-Host "  Get started:" -ForegroundColor Cyan
    Write-Host "    rustyhand init"
    Write-Host ""
    Write-Host "  The setup wizard will guide you through provider selection"
    Write-Host "  and configuration."
    Write-Host ""
}

Install-RustyHand
