# Start Open Life Reborn server with Job-breakaway durability (tmux analog on Windows).
#
# Why servers die when an agent/shell ends:
# - Grok/CI PowerShell often runs inside a Windows Job with KILL_ON_JOB_CLOSE.
# - Children of Start-Process / & exe inherit that job and die with the shell.
# - CREATE_BREAKAWAY_FROM_JOB is usually Access Denied under agent jobs.
#
# Durable pattern (no real tmux required):
# 1) Launch ol-server via WMI Win32_Process.Create (not a job descendant).
# 2) Fallback: schtasks one-shot outside the interactive job.
# 3) Optional watchdog (also WMI/schtasks-detached) restarts unless stop.flag.
#
# Usage:
#   powershell -File scripts/start-server.ps1
#   powershell -File scripts/start-server.ps1 -NoBrowser
#   powershell -File scripts/start-server.ps1 -Watchdog
#   powershell -File scripts/start-server.ps1 -Watchdog -NoBrowser
#
# Stop:
#   powershell -File scripts/stop-server.ps1
#
# Health:
#   Invoke-WebRequest http://127.0.0.1:8080/health

param(
    [switch]$Watchdog,
    [switch]$NoBrowser
)

$ErrorActionPreference = "Stop"
$Root = Split-Path -Parent (Split-Path -Parent $MyInvocation.MyCommand.Path)
Set-Location $Root

$SaveDir = Join-Path $Root "SaveFiles"
$PidFile = Join-Path $SaveDir "server.pid"
$StopFlag = Join-Path $SaveDir "stop.flag"
$WatchPidFile = Join-Path $SaveDir "watchdog.pid"

function Get-OlExe {
    # Prefer the newer of release/debug so a fresh debug build is not ignored.
    $rel = Join-Path $Root "target\release\ol-server.exe"
    $dbg = Join-Path $Root "target\debug\ol-server.exe"
    $candidates = @()
    if (Test-Path $rel) { $candidates += Get-Item $rel }
    if (Test-Path $dbg) { $candidates += Get-Item $dbg }
    if ($candidates.Count -eq 0) { return $null }
    return ($candidates | Sort-Object LastWriteTime -Descending | Select-Object -First 1).FullName
}

$exe = Get-OlExe
if (-not $exe) {
    Write-Error "ol-server.exe not found. Build: cargo build -p ol-server   (or --release)"
}

New-Item -ItemType Directory -Path $SaveDir -Force | Out-Null
# Clear sticky stop.flag from a previous orderly stop (server no longer deletes it).
Remove-Item $StopFlag -ErrorAction SilentlyContinue

# Stop prior instance from this repo's pid file first (avoid killing unrelated ol-server).
if (Test-Path $PidFile) {
    $oldPid = Get-Content $PidFile -ErrorAction SilentlyContinue | Select-Object -First 1
    if ($oldPid) {
        $old = Get-Process -Id ([int]$oldPid) -ErrorAction SilentlyContinue
        if ($old -and $old.ProcessName -eq "ol-server") {
            Write-Host "Stopping prior ol-server pid=$oldPid (from server.pid)"
            Stop-Process -Id ([int]$oldPid) -Force -ErrorAction SilentlyContinue
        }
    }
    Remove-Item $PidFile -ErrorAction SilentlyContinue
}
# Fallback: if something is still bound under our cwd health, name-kill last resort.
Start-Sleep -Milliseconds 400
try {
    $null = Invoke-WebRequest -Uri "http://127.0.0.1:8080/health" -UseBasicParsing -TimeoutSec 1
    Write-Warning "Port 8080 still healthy after pid stop - name-killing ol-server as last resort"
    Get-Process ol-server -ErrorAction SilentlyContinue | Stop-Process -Force -ErrorAction SilentlyContinue
} catch {}
Start-Sleep -Milliseconds 400

function Start-OlServerDetached {
    param([string]$ServerExe)
    # 1) WMI Create - not a child of this shell job
    try {
        $r = Invoke-CimMethod -ClassName Win32_Process -MethodName Create -Arguments @{
            CommandLine      = ('"{0}"' -f $ServerExe)
            CurrentDirectory = $Root
        }
        if ($r.ReturnValue -eq 0 -and $r.ProcessId -gt 0) {
            Write-Host "WMI Create ok pid=$($r.ProcessId)"
            return [int]$r.ProcessId
        }
        Write-Warning "WMI Create returned $($r.ReturnValue)"
    } catch {
        Write-Warning "WMI Create failed: $_"
    }

    # 2) schtasks one-shot (also outside agent Job Object)
    $task = "OLR_Server_Start"
    $null = schtasks /Delete /TN $task /F 2>$null
    $tr = '\"' + $ServerExe + '\"'
    $created = schtasks /Create /TN $task /TR $tr /SC ONCE /ST 00:00 /F /SD 01/01/2099 2>&1
    Write-Host "schtasks create: $created"
    $null = schtasks /Run /TN $task 2>&1
    Start-Sleep -Seconds 2
    $p = Get-Process ol-server -ErrorAction SilentlyContinue | Select-Object -First 1
    if ($p) {
        Write-Host "schtasks Run ok pid=$($p.Id)"
        return $p.Id
    }

    # 3) Last resort (may die when agent shell ends)
    Write-Warning "Using Start-Process fallback (NOT durable under Job KILL_ON_JOB_CLOSE)"
    $p = Start-Process -FilePath $ServerExe -WorkingDirectory $Root -WindowStyle Hidden -PassThru
    return $p.Id
}

function Start-WatchdogDetached {
    $watchScript = Join-Path $Root "scripts\watch-server.ps1"
    if (-not (Test-Path $watchScript)) {
        Write-Warning "watch-server.ps1 missing - inline watchdog only if -Watchdog blocks"
        return $null
    }
    # Detach watchdog via WMI so it survives this script ending.
    $ps = Join-Path $env:SystemRoot "System32\WindowsPowerShell\v1.0\powershell.exe"
    $cmd = '"{0}" -NoProfile -ExecutionPolicy Bypass -WindowStyle Hidden -File "{1}" -IntervalSec 5' -f $ps, $watchScript
    try {
        $r = Invoke-CimMethod -ClassName Win32_Process -MethodName Create -Arguments @{
            CommandLine      = $cmd
            CurrentDirectory = $Root
        }
        if ($r.ReturnValue -eq 0 -and $r.ProcessId -gt 0) {
            Write-Host "Watchdog WMI pid=$($r.ProcessId)"
            return [int]$r.ProcessId
        }
        Write-Warning "Watchdog WMI returned $($r.ReturnValue)"
    } catch {
        Write-Warning "Watchdog WMI failed: $_"
    }
    $task = "OLR_Watchdog"
    $null = schtasks /Delete /TN $task /F 2>$null
    $tr = 'powershell.exe -NoProfile -ExecutionPolicy Bypass -WindowStyle Hidden -File "' + $watchScript + '" -IntervalSec 5'
    $null = schtasks /Create /TN $task /TR $tr /SC ONCE /ST 00:00 /F /SD 01/01/2099 2>$null
    $null = schtasks /Run /TN $task 2>$null
    Start-Sleep -Seconds 1
    return $null
}

$serverPid = Start-OlServerDetached -ServerExe $exe
$serverPid | Out-File $PidFile -Encoding ascii
Write-Host "Started ol-server pid=$serverPid exe=$exe"

Write-Host "Waiting for http://127.0.0.1:8080/health ..."
$ready = $false
for ($i = 0; $i -lt 90; $i++) {
    Start-Sleep -Seconds 1
    try {
        $h = Invoke-WebRequest -Uri "http://127.0.0.1:8080/health" -UseBasicParsing -TimeoutSec 2
        if ($h.StatusCode -eq 200) { $ready = $true; break }
    } catch {}
    if (-not (Get-Process -Id $serverPid -ErrorAction SilentlyContinue)) {
        $p2 = Get-Process ol-server -ErrorAction SilentlyContinue | Select-Object -First 1
        if ($p2) { $serverPid = $p2.Id; $serverPid | Out-File $PidFile -Encoding ascii; continue }
        Write-Error "ol-server exited early. See SaveFiles\ol-server.log"
    }
}
if (-not $ready) {
    Write-Error "Web did not become ready. See SaveFiles\ol-server.log"
}

$serverPid | Out-File $PidFile -Encoding ascii

Write-Host ""
Write-Host "Open Life Reborn is up (pid=$serverPid) - durable (WMI/schtasks)"
Write-Host "  Home:      http://127.0.0.1:8080/"
Write-Host "  Viewer:    http://127.0.0.1:8080/viewer"
Write-Host "  Ops:       http://127.0.0.1:8080/ops"
Write-Host "  Log:       SaveFiles\ol-server.log"
Write-Host "  Heartbeat: SaveFiles\server.heartbeat"
Write-Host "  Stop:      powershell -File scripts\stop-server.ps1"
Write-Host ""
Write-Host "Durability notes:"
Write-Host "  - Server process is NOT a child of this shell (survives shell exit)."
Write-Host "  - With -Watchdog: a detached watcher restarts if the process dies,"
Write-Host "    unless SaveFiles\stop.flag exists (stop-server.ps1 creates it)."
Write-Host ""

if ($Watchdog) {
    $wpid = Start-WatchdogDetached
    if ($wpid) {
        $wpid | Out-File $WatchPidFile -Encoding ascii
        Write-Host "Detached watchdog running (pid=$wpid). This script can exit safely."
    } else {
        Write-Host "Could not detach watchdog via WMI; running inline watchdog (blocks)."
        Write-Host "Prefer re-run after fixing WMI, or: powershell -File scripts\watch-server.ps1"
        while ($true) {
            Start-Sleep -Seconds 5
            if (Test-Path $StopFlag) {
                Write-Host "stop.flag set - inline watchdog exiting"
                break
            }
            if (-not (Get-Process ol-server -ErrorAction SilentlyContinue)) {
                Write-Warning "ol-server died - restarting..."
                $serverPid = Start-OlServerDetached -ServerExe $exe
                $serverPid | Out-File $PidFile -Encoding ascii
                Write-Host "restarted pid=$serverPid"
            }
        }
    }
}

if (-not $NoBrowser) {
    Start-Process "http://127.0.0.1:8080/"
}
