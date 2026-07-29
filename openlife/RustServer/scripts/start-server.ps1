# Start Open Life Reborn Rust server.
#
# Desktop / debugging (visible terminal — recommended):
#   powershell -NoExit -File scripts\start-server.ps1 -Console
#
# Detached background (survives shell exit; less visible):
#   powershell -File scripts\start-server.ps1 -Detached
#   powershell -File scripts\start-server.ps1 -Detached -Watchdog -NoBrowser
#
# Stop:
#   powershell -File scripts\stop-server.ps1
#
# Health:
#   Invoke-WebRequest http://127.0.0.1:8080/health

param(
    # Run ol-server in THIS window so you see live logs and errors (default for Desktop).
    [switch]$Console,
    # Background launch via WMI (hidden). Use when you do not need a live console.
    [switch]$Detached,
    [switch]$Watchdog,
    [switch]$NoBrowser,
    # Build if no binary, or always rebuild with -Build.
    [switch]$Build,
    # Config file relative to repo root (public server.toml only — no secrets).
    [string]$Config = "server.toml",
    # Wait for a key before the window can close (on success and failure).
    [switch]$Pause,
    # Never wait for a key (agents / CI).
    [switch]$NoPause
)

$ErrorActionPreference = "Stop"

function Wait-ForKeyIfNeeded {
    param([string]$Message = "Press Enter to close this window...")
    if ($NoPause) { return }
    # Default: pause when -Pause, or when -Console, or when not Detached (interactive default).
    $should = $Pause -or $Console -or (-not $Detached)
    if (-not $should) { return }
    try {
        Write-Host ""
        Write-Host $Message -ForegroundColor Yellow
        [void](Read-Host)
    } catch {
        # Non-interactive host
    }
}

try {
    $Root = Split-Path -Parent (Split-Path -Parent $MyInvocation.MyCommand.Path)
    Set-Location $Root

    # Default mode: Console (visible) unless user asked for Detached.
    if (-not $Detached -and -not $Console) {
        $Console = $true
    }
    if ($Detached -and $Console) {
        Write-Warning "Both -Detached and -Console set; using -Console."
        $Detached = $false
    }

    $SaveDir = Join-Path $Root "SaveFiles"
    $PidFile = Join-Path $SaveDir "server.pid"
    $StopFlag = Join-Path $SaveDir "stop.flag"
    $WatchPidFile = Join-Path $SaveDir "watchdog.pid"
    $LogFile = Join-Path $SaveDir "ol-server.log"
    $ConfigPath = Join-Path $Root $Config
    if (-not (Test-Path $ConfigPath)) {
        throw "Config not found: $ConfigPath"
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

    function Ensure-OlExe {
        param([switch]$ForceBuild)
        $exe = Get-OlExe
        if ($exe -and -not $ForceBuild) { return $exe }

        $cargo = Get-Command cargo -ErrorAction SilentlyContinue
        if (-not $cargo) {
            $cargoBin = Join-Path $env:USERPROFILE ".cargo\bin\cargo.exe"
            if (Test-Path $cargoBin) {
                $env:PATH = "$(Split-Path $cargoBin -Parent);$env:PATH"
                $cargo = Get-Command cargo -ErrorAction SilentlyContinue
            }
        }
        if (-not $cargo) {
            throw "cargo not found. Install Rust from https://rustup.rs then rebuild."
        }

        Write-Host "Building ol-server (cargo build -p ol-server)..."
        & cargo build -p ol-server
        if ($LASTEXITCODE -ne 0) {
            throw "cargo build failed (exit $LASTEXITCODE)"
        }
        $exe = Get-OlExe
        if (-not $exe) {
            throw "ol-server.exe still missing after build."
        }
        return $exe
    }

    Write-Host "=== Open Life Rust Server ===" -ForegroundColor Cyan
    Write-Host "Root:   $Root"
    Write-Host "Config: $ConfigPath"
    Write-Host "Mode:   $(if ($Console) { 'Console (visible logs)' } else { 'Detached (background)' })"
    Write-Host ""

    $exe = Ensure-OlExe -ForceBuild:$Build
    Write-Host "Binary: $exe"
    Write-Host ""

    New-Item -ItemType Directory -Path $SaveDir -Force | Out-Null
    Remove-Item $StopFlag -ErrorAction SilentlyContinue

    # Stop prior instance from this repo's pid file.
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
    Start-Sleep -Milliseconds 400
    try {
        $null = Invoke-WebRequest -Uri "http://127.0.0.1:8080/health" -UseBasicParsing -TimeoutSec 1
        Write-Warning "Port 8080 still healthy after pid stop - name-killing ol-server as last resort"
        Get-Process ol-server -ErrorAction SilentlyContinue | Stop-Process -Force -ErrorAction SilentlyContinue
    } catch {}
    Start-Sleep -Milliseconds 400

    if ($Console) {
        # Run in THIS window so stderr/stdout stay visible.
        Write-Host "Starting ol-server in this window (Ctrl+C to stop)..." -ForegroundColor Green
        Write-Host "  Game port: see $Config (default 8005)"
        Write-Host "  Web UI:    http://127.0.0.1:8080/"
        Write-Host "  Log file:  $LogFile"
        Write-Host ""

        if (-not $NoBrowser) {
            # Open browser shortly after start so it does not block boot output.
            Start-Job -ScriptBlock {
                Start-Sleep -Seconds 3
                Start-Process "http://127.0.0.1:8080/"
            } | Out-Null
        }

        $p = Start-Process -FilePath $exe -ArgumentList @($ConfigPath) `
            -WorkingDirectory $Root -NoNewWindow -PassThru -Wait
        $code = $p.ExitCode
        Write-Host ""
        if ($code -eq 0 -or $null -eq $code) {
            Write-Host "ol-server exited (code=$code)." -ForegroundColor Yellow
        } else {
            Write-Host "ol-server FAILED (exit code=$code)." -ForegroundColor Red
            if (Test-Path $LogFile) {
                Write-Host ""
                Write-Host "--- last 40 lines of SaveFiles\ol-server.log ---" -ForegroundColor Yellow
                Get-Content $LogFile -Tail 40 -ErrorAction SilentlyContinue
            }
        }
        Wait-ForKeyIfNeeded
        exit $(if ($code) { $code } else { 0 })
    }

    # --- Detached path (background) ---
    function Start-OlServerDetached {
        param(
            [string]$ServerExe,
            [string]$ConfigFile
        )
        $cmdLine = '"{0}" "{1}"' -f $ServerExe, $ConfigFile
        try {
            $r = Invoke-CimMethod -ClassName Win32_Process -MethodName Create -Arguments @{
                CommandLine      = $cmdLine
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

        $task = "OLR_Server_Start"
        $null = schtasks /Delete /TN $task /F 2>$null
        $tr = '\"' + $ServerExe + '\" \"' + $ConfigFile + '\"'
        $created = schtasks /Create /TN $task /TR $tr /SC ONCE /ST 00:00 /F /SD 01/01/2099 2>&1
        Write-Host "schtasks create: $created"
        $null = schtasks /Run /TN $task 2>&1
        Start-Sleep -Seconds 2
        $p = Get-Process ol-server -ErrorAction SilentlyContinue | Select-Object -First 1
        if ($p) {
            Write-Host "schtasks Run ok pid=$($p.Id)"
            return $p.Id
        }

        Write-Warning "Using Start-Process fallback (visible console window)"
        $p = Start-Process -FilePath $ServerExe -ArgumentList @($ConfigFile) `
            -WorkingDirectory $Root -WindowStyle Normal -PassThru
        return $p.Id
    }

    function Start-WatchdogDetached {
        $watchScript = Join-Path $Root "scripts\watch-server.ps1"
        if (-not (Test-Path $watchScript)) {
            Write-Warning "watch-server.ps1 missing"
            return $null
        }
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
        } catch {
            Write-Warning "Watchdog WMI failed: $_"
        }
        return $null
    }

    $serverPid = Start-OlServerDetached -ServerExe $exe -ConfigFile $ConfigPath
    $serverPid | Out-File $PidFile -Encoding ascii
    Write-Host "Started ol-server pid=$serverPid"

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
            if ($p2) {
                $serverPid = $p2.Id
                $serverPid | Out-File $PidFile -Encoding ascii
                continue
            }
            Write-Host ""
            Write-Host "ol-server exited early." -ForegroundColor Red
            if (Test-Path $LogFile) {
                Write-Host "--- last 40 lines of SaveFiles\ol-server.log ---" -ForegroundColor Yellow
                Get-Content $LogFile -Tail 40 -ErrorAction SilentlyContinue
            }
            throw "ol-server exited early. See SaveFiles\ol-server.log"
        }
    }
    if (-not $ready) {
        Write-Host ""
        Write-Host "Web did not become ready within 90s." -ForegroundColor Red
        if (Test-Path $LogFile) {
            Write-Host "--- last 40 lines of SaveFiles\ol-server.log ---" -ForegroundColor Yellow
            Get-Content $LogFile -Tail 40 -ErrorAction SilentlyContinue
        }
        throw "Web did not become ready. See SaveFiles\ol-server.log"
    }

    $serverPid | Out-File $PidFile -Encoding ascii

    Write-Host ""
    Write-Host "Open Life Reborn is up (pid=$serverPid)" -ForegroundColor Green
    Write-Host "  Home:      http://127.0.0.1:8080/"
    Write-Host "  Viewer:    http://127.0.0.1:8080/viewer"
    Write-Host "  Ops:       http://127.0.0.1:8080/ops"
    Write-Host "  Log:       SaveFiles\ol-server.log"
    Write-Host "  Stop:      scripts\stop-server.ps1 (or Desktop shortcut)"
    Write-Host ""

    if ($Watchdog) {
        $wpid = Start-WatchdogDetached
        if ($wpid) {
            $wpid | Out-File $WatchPidFile -Encoding ascii
            Write-Host "Detached watchdog running (pid=$wpid)."
        }
    }

    if (-not $NoBrowser) {
        Start-Process "http://127.0.0.1:8080/"
    }

    Wait-ForKeyIfNeeded -Message "Server is running in the background. Press Enter to close this window..."
}
catch {
    Write-Host ""
    Write-Host "ERROR: $($_.Exception.Message)" -ForegroundColor Red
    Write-Host $_.ScriptStackTrace -ForegroundColor DarkGray
    $log = Join-Path (Split-Path -Parent (Split-Path -Parent $MyInvocation.MyCommand.Path)) "SaveFiles\ol-server.log"
    if (Test-Path $log) {
        Write-Host ""
        Write-Host "--- last 40 lines of SaveFiles\ol-server.log ---" -ForegroundColor Yellow
        Get-Content $log -Tail 40 -ErrorAction SilentlyContinue
    }
    Wait-ForKeyIfNeeded -Message "Press Enter to close this window..."
    exit 1
}
