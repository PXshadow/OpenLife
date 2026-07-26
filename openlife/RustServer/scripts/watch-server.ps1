# Watchdog: keep ol-server alive (Windows analog of a tmux session).
#
# Polls every few seconds; restarts via WMI/schtasks if the process is gone
# and SaveFiles/stop.flag is NOT present.
#
# Usage:
#   powershell -File scripts/watch-server.ps1
#   powershell -File scripts/watch-server.ps1 -IntervalSec 5
#   (normally launched detached by start-server.ps1 -Watchdog)

param(
    [int]$IntervalSec = 5
)

$ErrorActionPreference = "Continue"
$Root = Split-Path -Parent (Split-Path -Parent $MyInvocation.MyCommand.Path)
Set-Location $Root

$SaveDir = Join-Path $Root "SaveFiles"
$StopFlag = Join-Path $SaveDir "stop.flag"
$PidFile = Join-Path $SaveDir "server.pid"
$WatchPidFile = Join-Path $SaveDir "watchdog.pid"
$LogFile = Join-Path $SaveDir "watchdog.log"

New-Item -ItemType Directory -Path $SaveDir -Force | Out-Null
$PID | Out-File $WatchPidFile -Encoding ascii

function Write-WatchLog([string]$msg) {
    $line = "[{0}] {1}" -f (Get-Date -Format "yyyy-MM-dd HH:mm:ss"), $msg
    Add-Content -Path $LogFile -Value $line -ErrorAction SilentlyContinue
    Write-Host $line
}

function Get-OlExe {
    $rel = Join-Path $Root "target\release\ol-server.exe"
    $dbg = Join-Path $Root "target\debug\ol-server.exe"
    $candidates = @()
    if (Test-Path $rel) { $candidates += Get-Item $rel }
    if (Test-Path $dbg) { $candidates += Get-Item $dbg }
    if ($candidates.Count -eq 0) { return $null }
    return ($candidates | Sort-Object LastWriteTime -Descending | Select-Object -First 1).FullName
}

function Start-OlServerDetached {
    $exe = Get-OlExe
    if (-not $exe) {
        Write-WatchLog "ERROR: ol-server.exe not found"
        return $null
    }
    # Prefer WMI Create - process is not a child of this shell's Job Object.
    try {
        $r = Invoke-CimMethod -ClassName Win32_Process -MethodName Create -Arguments @{
            CommandLine      = ('"{0}"' -f $exe)
            CurrentDirectory = $Root
        }
        if ($r.ReturnValue -eq 0 -and $r.ProcessId -gt 0) {
            return [int]$r.ProcessId
        }
        Write-WatchLog "WMI Create returned $($r.ReturnValue)"
    } catch {
        Write-WatchLog "WMI Create failed: $_"
    }

    # schtasks one-shot (also outside interactive agent Job)
    $task = "OLR_Server_Watchdog_Start"
    $null = schtasks /Delete /TN $task /F 2>$null
    $tr = '\"' + $exe + '\"'
    $null = schtasks /Create /TN $task /TR $tr /SC ONCE /ST 00:00 /F /SD 01/01/2099 2>$null
    $null = schtasks /Run /TN $task 2>$null
    Start-Sleep -Seconds 2
    $p = Get-Process ol-server -ErrorAction SilentlyContinue | Select-Object -First 1
    if ($p) { return $p.Id }

    Write-WatchLog "WARNING: Start-Process fallback (may die with parent job)"
    $p = Start-Process -FilePath $exe -WorkingDirectory $Root -WindowStyle Hidden -PassThru
    return $p.Id
}

Write-WatchLog "watchdog start pid=$PID interval=${IntervalSec}s root=$Root"

while ($true) {
    # Always honor sticky stop.flag first — never restart while it exists.
    # (Server leaves the flag in place on orderly stop; start-server clears it.)
    if (Test-Path $StopFlag) {
        Write-WatchLog "stop.flag present - watchdog exiting (no restart)"
        break
    }

    $alive = Get-Process ol-server -ErrorAction SilentlyContinue
    if (-not $alive) {
        # Re-check flag after process observation (close the race window).
        if (Test-Path $StopFlag) {
            Write-WatchLog "stop.flag appeared while process down - exit without restart"
            break
        }
        Write-WatchLog "ol-server not running - restarting"
        $newPid = Start-OlServerDetached
        if ($newPid) {
            $newPid | Out-File $PidFile -Encoding ascii
            Write-WatchLog "restarted ol-server pid=$newPid"
        } else {
            Write-WatchLog "restart failed; will retry"
        }
    } else {
        $alive[0].Id | Out-File $PidFile -Encoding ascii
    }

    Start-Sleep -Seconds $IntervalSec
}

Remove-Item $WatchPidFile -ErrorAction SilentlyContinue
Write-WatchLog "watchdog exit"
