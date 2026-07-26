# Orderly stop of ol-server + watchdog (tmux-session teardown analog).
#
# Order matters to avoid the stop.flag race:
# 1) Kill watchdog FIRST so it cannot restart after the server exits.
# 2) Write SaveFiles/stop.flag (sticky — server no longer deletes it).
# 3) Wait for server exit; force-kill by pid-file if needed.
# 4) start-server.ps1 clears stop.flag on next start.

$ErrorActionPreference = "Continue"
$Root = Split-Path -Parent (Split-Path -Parent $MyInvocation.MyCommand.Path)
$SaveDir = Join-Path $Root "SaveFiles"
$flag = Join-Path $SaveDir "stop.flag"
$PidFile = Join-Path $SaveDir "server.pid"
$WatchPidFile = Join-Path $SaveDir "watchdog.pid"

New-Item -ItemType Directory -Path $SaveDir -Force | Out-Null

# 1) Stop watchdog first so it cannot WMI-restart while we wait for the server.
if (Test-Path $WatchPidFile) {
    $wpid = Get-Content $WatchPidFile -ErrorAction SilentlyContinue | Select-Object -First 1
    if ($wpid) {
        $wp = Get-Process -Id ([int]$wpid) -ErrorAction SilentlyContinue
        if ($wp -and $wp.ProcessName -match "powershell|pwsh") {
            Write-Host "Stopping watchdog pid=$wpid"
            Stop-Process -Id ([int]$wpid) -Force -ErrorAction SilentlyContinue
        }
    }
    Remove-Item $WatchPidFile -ErrorAction SilentlyContinue
}
Get-CimInstance Win32_Process -Filter "Name = 'powershell.exe' OR Name = 'pwsh.exe'" -ErrorAction SilentlyContinue |
    Where-Object { $_.CommandLine -and $_.CommandLine -like "*watch-server.ps1*" -and $_.CommandLine -like "*$Root*" } |
    ForEach-Object {
        Write-Host "Stopping watchdog process Id=$($_.ProcessId)"
        Stop-Process -Id $_.ProcessId -Force -ErrorAction SilentlyContinue
    }

# 2) Sticky stop flag (server leaves it in place; start-server clears on next run).
New-Item -ItemType File -Path $flag -Force | Out-Null
Write-Host "Wrote $flag - waiting for ol-server to exit..."

# 3) Wait, then pid-file targeted kill (not every ol-server on the machine).
function Stop-OlServerByPidFile {
    if (-not (Test-Path $PidFile)) { return $false }
    $spid = Get-Content $PidFile -ErrorAction SilentlyContinue | Select-Object -First 1
    if (-not $spid) { return $false }
    $p = Get-Process -Id ([int]$spid) -ErrorAction SilentlyContinue
    if ($p -and $p.ProcessName -eq "ol-server") {
        Write-Warning "Force killing ol-server pid=$spid (from server.pid)"
        Stop-Process -Id ([int]$spid) -Force -ErrorAction SilentlyContinue
        return $true
    }
    return $false
}

for ($i = 0; $i -lt 30; $i++) {
    $alive = $false
    if (Test-Path $PidFile) {
        $spid = Get-Content $PidFile -ErrorAction SilentlyContinue | Select-Object -First 1
        if ($spid -and (Get-Process -Id ([int]$spid) -ErrorAction SilentlyContinue)) {
            $alive = $true
        }
    }
    if (-not $alive -and -not (Get-Process ol-server -ErrorAction SilentlyContinue)) {
        Write-Host "Server stopped."
        break
    }
    if (-not $alive) {
        # pid file stale but name still present — wait a bit more then pid-less exit check
        if (-not (Get-Process ol-server -ErrorAction SilentlyContinue)) {
            Write-Host "Server stopped."
            break
        }
    }
    Start-Sleep -Seconds 1
}

if (Get-Process ol-server -ErrorAction SilentlyContinue) {
    if (-not (Stop-OlServerByPidFile)) {
        Write-Warning "server.pid missing/stale - falling back to name kill for remaining ol-server"
        Get-Process ol-server -ErrorAction SilentlyContinue | Stop-Process -Force
    }
}

Remove-Item $PidFile -ErrorAction SilentlyContinue
Write-Host "Done. stop.flag left sticky until next start-server.ps1"
Write-Host "Restart with: powershell -File scripts\start-server.ps1 -Watchdog -NoBrowser"
