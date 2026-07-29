# Create Desktop shortcuts for Open Life Rust server (no secrets).
#
# Usage:
#   powershell -NoProfile -ExecutionPolicy Bypass -File scripts\install-desktop-shortcuts.ps1
#
# Creates:
#   Desktop\Start Open Life Rust Server.lnk  (visible console, stays open)
#   Desktop\Stop Open Life Rust Server.lnk

$ErrorActionPreference = "Stop"
$Root = Split-Path -Parent (Split-Path -Parent $MyInvocation.MyCommand.Path)
$StartScript = Join-Path $Root "scripts\start-server.ps1"
$StopScript = Join-Path $Root "scripts\stop-server.ps1"

if (-not (Test-Path $StartScript)) {
    Write-Error "Missing $StartScript"
}
if (-not (Test-Path $StopScript)) {
    Write-Error "Missing $StopScript"
}

$Desktop = [Environment]::GetFolderPath("Desktop")
if (-not $Desktop -or -not (Test-Path $Desktop)) {
    $Desktop = Join-Path $env:USERPROFILE "Desktop"
}
if (-not (Test-Path $Desktop)) {
    Write-Error "Desktop folder not found."
}

$ps = Join-Path $env:SystemRoot "System32\WindowsPowerShell\v1.0\powershell.exe"
$wsh = New-Object -ComObject WScript.Shell

function New-OlShortcut {
    param(
        [string]$Name,
        [string]$Arguments,
        [string]$Description
    )
    $lnkPath = Join-Path $Desktop $Name
    $sc = $wsh.CreateShortcut($lnkPath)
    $sc.TargetPath = $ps
    $sc.Arguments = $Arguments
    $sc.WorkingDirectory = $Root
    $sc.WindowStyle = 1  # Normal window
    $sc.Description = $Description
    $icon = Join-Path $Root "target\debug\ol-server.exe"
    if (-not (Test-Path $icon)) {
        $icon = Join-Path $Root "target\release\ol-server.exe"
    }
    if (Test-Path $icon) {
        $sc.IconLocation = "$icon,0"
    }
    $sc.Save()
    Write-Host "Created: $lnkPath"
}

# -NoExit keeps the window open after the script ends or errors.
# -Console runs the server in that same window so logs/errors are visible.
$startArgs = @(
    '-NoExit'
    '-NoProfile'
    '-ExecutionPolicy'
    'Bypass'
    '-File'
    ('"{0}"' -f $StartScript)
    '-Console'
) -join ' '

$stopArgs = @(
    '-NoExit'
    '-NoProfile'
    '-ExecutionPolicy'
    'Bypass'
    '-File'
    ('"{0}"' -f $StopScript)
) -join ' '

New-OlShortcut `
    -Name "Start Open Life Rust Server.lnk" `
    -Arguments $startArgs `
    -Description "Start Open Life Rust server in a visible console (game :8005, web :8080)."

New-OlShortcut `
    -Name "Stop Open Life Rust Server.lnk" `
    -Arguments $stopArgs `
    -Description "Stop Open Life Rust server and watchdog."

Write-Host ""
Write-Host "Desktop shortcuts ready."
Write-Host "  Start: double-click 'Start Open Life Rust Server'"
Write-Host "         -> PowerShell stays open; server logs stream in the window."
Write-Host "  Web:   http://127.0.0.1:8080/"
Write-Host "  Game:  port 8005 (server.toml)"
Write-Host "  Stop:  double-click 'Stop Open Life Rust Server' (or Ctrl+C in start window)"
Write-Host ""
Write-Host "Other start flags (append to shortcut Arguments if needed):"
Write-Host "  -Detached   background (old behavior, less visible)"
Write-Host "  -Watchdog   auto-restart if process dies (with -Detached)"
Write-Host "  -NoBrowser  do not open the web UI"
Write-Host "  -Build      force cargo build before start"
Write-Host "  -Config server-playtest.toml"
