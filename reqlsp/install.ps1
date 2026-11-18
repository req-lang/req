$ErrorActionPreference = "Stop"

$Repo = "req-lang/req"
$Binary = "reqlsp"
$InstallDir = "$env:LOCALAPPDATA\Programs\reqlsp"

# Resolve latest version
if ($env:VERSION) {
    $Version = $env:VERSION
} else {
    $Release = Invoke-RestMethod "https://api.github.com/repos/$Repo/releases/latest"
    $Version = $Release.tag_name
}

if (-not $Version) {
    Write-Error "Failed to resolve latest version. Set `$env:VERSION manually and retry."
    exit 1
}

$Artifact = "$Binary-x86_64-pc-windows-msvc.exe"
$Url = "https://github.com/$Repo/releases/download/$Version/$Artifact"

Write-Host "Installing $Binary $Version to $InstallDir"

New-Item -ItemType Directory -Force -Path $InstallDir | Out-Null

Invoke-WebRequest -Uri $Url -OutFile "$InstallDir\$Binary.exe"

# Add to user PATH if not already present
$UserPath = [Environment]::GetEnvironmentVariable("PATH", "User")
if ($UserPath -notlike "*$InstallDir*") {
    [Environment]::SetEnvironmentVariable("PATH", "$UserPath;$InstallDir", "User")
    Write-Host "Added $InstallDir to your PATH. Restart your terminal for changes to take effect."
}

Write-Host "Done. Point your editor's language client at '$Binary' (stdio transport)."
