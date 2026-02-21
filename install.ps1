$ErrorActionPreference = "Stop"

# Force TLS 1.2
[Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12

Write-Host "Installing cue for Windows..." -ForegroundColor Green

try {
    # Get latest release info from GitHub API
    Write-Host "Fetching latest release..." -ForegroundColor Cyan

    $webClient = New-Object System.Net.WebClient
    $webClient.Headers.Add("User-Agent", "PowerShell")

    $apiResponse = $webClient.DownloadString("https://api.github.com/repos/ozx1/cue/releases/latest")
    $release = $apiResponse | ConvertFrom-Json
    $version = $release.tag_name

    if (-not $version) {
        throw "Failed to fetch latest release version"
    }

    # Build download URL
    $downloadUrl = "https://github.com/ozx1/cue/releases/download/$version/cue-windows-x86_64.exe"
    $tempFile = Join-Path $env:TEMP "cue.exe"

    Write-Host "Downloading version $version from GitHub..." -ForegroundColor Cyan
    $webClient.DownloadFile($downloadUrl, $tempFile)
    Write-Host "Download complete!" -ForegroundColor Green

    # Create install directory
    $installDir = Join-Path $env:LOCALAPPDATA "Programs\cue"
    if (-not (Test-Path $installDir)) {
        New-Item -ItemType Directory -Path $installDir -Force | Out-Null
    }

    # Move file to install directory
    $finalPath = Join-Path $installDir "cue.exe"
    if (Test-Path $finalPath) {
        Remove-Item $finalPath -Force
    }
    Move-Item $tempFile $finalPath -Force

    Write-Host "Installed to: $finalPath" -ForegroundColor Green

    # Add to PATH if not already there
    $userPath = [Environment]::GetEnvironmentVariable("Path", "User")
    if ($userPath -notlike "*$installDir*") {
        $newPath = $userPath + ";" + $installDir
        [Environment]::SetEnvironmentVariable("Path", $newPath, "User")
        $env:Path = [Environment]::GetEnvironmentVariable("Path", "Machine") + ";" + $newPath
        Write-Host "Added to PATH" -ForegroundColor Green
        Write-Host "Note: Restart your terminal for PATH changes to take effect" -ForegroundColor Yellow
    } else {
        Write-Host "Already in PATH" -ForegroundColor Green
    }

    Write-Host ""
    Write-Host "✓ cue installed successfully!" -ForegroundColor Green
    Write-Host "Run 'cue -h' to get started (restart your terminal first if needed)" -ForegroundColor Cyan

} catch {
    Write-Host ""
    Write-Host "Installation failed: $_" -ForegroundColor Red
    Write-Host ""
    Write-Host "Manual installation:" -ForegroundColor Yellow
    Write-Host "1. Go to: https://github.com/ozx1/cue/releases/latest" -ForegroundColor Yellow
    Write-Host "2. Download: cue-windows-x86_64.exe" -ForegroundColor Yellow
    Write-Host "3. Rename to cue.exe and move to a folder in your PATH" -ForegroundColor Yellow
    exit 1
}
