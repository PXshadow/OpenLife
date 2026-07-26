<#
.SYNOPSIS
  Read SuperGrok usage (live fetch preferred) and compute soft-ceiling budget status.
#>
[CmdletBinding()]
param(
    [switch]$Offline,
    [switch]$Json,
    [switch]$Quiet
)

$ErrorActionPreference = "Stop"

$BudgetDir = Join-Path $PSScriptRoot "data"
$SnapshotPath = Join-Path $BudgetDir "usage-snapshot.json"
$ConfigPath = Join-Path $BudgetDir "config.json"
$FetchScript = Join-Path $PSScriptRoot "fetch-usage.ps1"

function Read-JsonFile([string]$Path) {
    if (-not (Test-Path $Path)) { return $null }
    return Get-Content -Raw -Path $Path -Encoding UTF8 | ConvertFrom-Json
}

function Get-UtcNow { [DateTimeOffset]::UtcNow }

function Get-DayInfo([datetimeoffset]$Now, $Config, $Snapshot) {
    $periodDays = 7.0
    if ($Config -and $Config.period_days) { $periodDays = [double]$Config.period_days }
    if ($periodDays -lt 1) { $periodDays = 7 }

    $start = $null
    $end = $null

    if ($Config -and $Config.period_start_utc) {
        try { $start = [DateTimeOffset]::Parse([string]$Config.period_start_utc) } catch { }
    }
    if ($Snapshot -and $Snapshot.period_start_utc) {
        try { $start = [DateTimeOffset]::Parse([string]$Snapshot.period_start_utc) } catch { }
    }
    if ($Config -and $Config.reset_at_utc) {
        try { $end = [DateTimeOffset]::Parse([string]$Config.reset_at_utc) } catch { }
    }
    if ($Snapshot -and $Snapshot.reset_at_utc) {
        try { $end = [DateTimeOffset]::Parse([string]$Snapshot.reset_at_utc) } catch { }
    }

    if ($start -and $end) {
        $periodDays = [Math]::Max(1.0, ($end - $start).TotalDays)
    }
    elseif ($end -and -not $start) {
        $start = $end.AddDays(-$periodDays)
    }
    elseif (-not $start) {
        $d = $Now.UtcDateTime.Date
        $dow = [int]$d.DayOfWeek
        $daysFromMonday = ($dow + 6) % 7
        $start = [DateTimeOffset]::new($d.AddDays(-$daysFromMonday), [TimeSpan]::Zero)
        $periodDays = 7
    }

    if (-not $end) {
        $end = $start.AddDays($periodDays)
    }

    $elapsed = [Math]::Max(0.0, ($Now - $start).TotalDays)
    $dayIndex = [Math]::Min([Math]::Floor($elapsed) + 1, [Math]::Ceiling($periodDays))
    $frac = [Math]::Min(1.0, $elapsed / $periodDays)

    return [pscustomobject]@{
        period_days = $periodDays
        start       = $start
        end         = $end
        day_index   = [int]$dayIndex
        fraction    = $frac
    }
}

try {
    if (-not $Offline) {
        & $FetchScript -NoSave:$false 2>$null | Out-Null
    }

    $snap = Read-JsonFile $SnapshotPath
    $cfg = Read-JsonFile $ConfigPath
    if (-not $cfg) {
        $cfg = [pscustomobject]@{
            cap_percent = 80
            soft_pace   = $true
            period_days = 7
        }
    }

    if (-not $snap -or $null -eq $snap.used_percent) {
        if ($Quiet) { exit 3 }
        if ($Json) {
            @{ ok = $false; error = "no usage snapshot" } | ConvertTo-Json
            exit 3
        }
        Write-Host "No usage data. Run: .\fetch-usage.ps1 (needs .env)" -ForegroundColor Yellow
        exit 3
    }

    $now = Get-UtcNow
    $day = Get-DayInfo $now $cfg $snap
    $cap = 80.0
    if ($cfg.cap_percent) { $cap = [double]$cfg.cap_percent }
    $used = [double]$snap.used_percent
    $soft = $cap
    if ($cfg.soft_pace -ne $false) {
        $soft = [Math]::Round($cap * $day.fraction, 2)
        if ($soft -lt 1) { $soft = 1 }
    }

    $status = "ok"
    $code = 0
    if ($used -ge $cap) {
        $status = "over_hard_cap"
        $code = 2
    }
    elseif ($used -ge $soft) {
        $status = "over_soft_ceiling"
        $code = 2
    }

    $report = [ordered]@{
        ok                = ($code -eq 0)
        status            = $status
        used_percent      = $used
        remaining_percent = [Math]::Round(100.0 - $used, 2)
        hard_cap_percent  = $cap
        soft_ceiling      = $soft
        day_index         = $day.day_index
        period_days       = $day.period_days
        reset_at_utc      = $snap.reset_at_utc
        snapshot_path     = $SnapshotPath
    }

    if ($Quiet) { exit $code }
    if ($Json) {
        $report | ConvertTo-Json -Depth 6
        exit $code
    }

    Write-Host "=== Budget status ===" -ForegroundColor Cyan
    Write-Host "Used:          $used%"
    Write-Host "Hard cap:      $cap%"
    Write-Host "Soft ceiling:  $soft%  (day $($day.day_index)/$([Math]::Ceiling($day.period_days)))"
    Write-Host "Status:        $status"
    Write-Host "Reset:         $($snap.reset_at_utc)"
    exit $code
}
catch {
    if ($Quiet) { exit 1 }
    if ($Json) {
        @{ ok = $false; error = $_.Exception.Message } | ConvertTo-Json
    }
    else {
        Write-Host "Error: $($_.Exception.Message)" -ForegroundColor Red
    }
    exit 1
}
