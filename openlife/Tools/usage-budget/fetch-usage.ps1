<#
.SYNOPSIS
  Fetch SuperGrok / Grok Build usage (same path as /usage).

.DESCRIPTION
  Auth is loaded ONLY from local secrets, never hard-coded:

    1) openlife/Tools/usage-budget/.env  (preferred - gitignored)
    2) Process environment variables GROK_API_KEY + GROK_USER_ID
    3) Optional: %USERPROFILE%\.grok\auth.json if GROK_ALLOW_AUTH_JSON=1

  Snapshots written to data/usage-snapshot.json contain only usage percentages -
  never API keys.

.EXAMPLE
  copy .env.example .env
  # fill GROK_API_KEY and GROK_USER_ID, or:
  .\export-auth-to-env.ps1
  .\fetch-usage.ps1
#>
[CmdletBinding()]
param(
    [switch]$Json,
    [switch]$NoSave,
    [string]$ProxyBase = $(
        if ($env:GROK_CLI_CHAT_PROXY_BASE_URL) { $env:GROK_CLI_CHAT_PROXY_BASE_URL }
        else { "https://cli-chat-proxy.grok.com/v1" }
    )
)

$ErrorActionPreference = "Stop"

. (Join-Path $PSScriptRoot "lib\Load-Env.ps1")

$BudgetDir = Join-Path $PSScriptRoot "data"
$SnapshotPath = Join-Path $BudgetDir "usage-snapshot.json"
$ConfigPath = Join-Path $BudgetDir "config.json"
$EnvPath = Join-Path $PSScriptRoot ".env"

function Get-Auth {
    # 1-2: .env + process env
    $null = Import-DotEnv -Path $EnvPath
    $fromEnv = Get-UsageAuthFromEnv
    if ($fromEnv) {
        return [pscustomobject]@{
            source = if (Test-Path $EnvPath) { ".env" } else { "process-env" }
            auth   = $fromEnv
        }
    }

    # 3: optional local auth.json (not in this repo) - opt-in only
    $allowJson = $env:GROK_ALLOW_AUTH_JSON -eq "1" -or $env:GROK_ALLOW_AUTH_JSON -eq "true"
    if (-not $allowJson) {
        throw @"
No credentials found.

Create a local .env (never commit it):
  copy .env.example .env
  # then either fill GROK_API_KEY / GROK_USER_ID, or run:
  .\export-auth-to-env.ps1

Or set process env GROK_API_KEY and GROK_USER_ID.

Optional: set GROK_ALLOW_AUTH_JSON=1 to fall back to %USERPROFILE%\.grok\auth.json
"@
    }

    $authPath = Join-Path $env:USERPROFILE ".grok\auth.json"
    if (-not (Test-Path $authPath)) {
        throw "GROK_ALLOW_AUTH_JSON set but auth.json missing at $authPath - run: grok login"
    }
    $store = Get-Content -Raw -Path $authPath -Encoding UTF8 | ConvertFrom-Json
    $now = [DateTimeOffset]::UtcNow
    foreach ($p in $store.PSObject.Properties) {
        $a = $p.Value
        if (-not $a.key -or -not $a.user_id) { continue }
        $expired = $false
        if ($a.expires_at) {
            try {
                $exp = [DateTimeOffset]::Parse([string]$a.expires_at)
                if ($exp -lt $now.AddMinutes(-2)) { $expired = $true }
            } catch { }
        }
        if ($expired) { continue }
        return [pscustomobject]@{
            source = "auth.json:$($p.Name)"
            auth   = @{ key = [string]$a.key; user_id = [string]$a.user_id }
        }
    }
    throw "No non-expired key+user_id in $authPath"
}

function Invoke-BillingCredits($Auth, [string]$Base) {
    $base = $Base.TrimEnd('/')
    $url = "$base/billing?format=credits"
    $headers = @{
        "Authorization"         = "Bearer $($Auth.key)"
        "X-XAI-Token-Auth"      = "xai-grok-cli"
        "x-userid"              = [string]$Auth.user_id
        "x-grok-client-version" = "openlife-tools-usage/0.4"
        "Accept"                = "application/json"
    }
    if ($env:GROK_CLIENT_MODE) {
        $headers["x-grok-client-mode"] = $env:GROK_CLIENT_MODE
    }
    return Invoke-RestMethod -Method Get -Uri $url -Headers $headers -TimeoutSec 20
}

function Convert-BillingToReport($Billing, [string]$Source) {
    $cfg = $Billing.config
    if (-not $cfg) { throw "Billing response had no config field" }

    $used = $null
    if ($null -ne $cfg.creditUsagePercent) {
        $used = [double]$cfg.creditUsagePercent
    }
    elseif ($cfg.monthlyLimit -and $cfg.used -and $cfg.monthlyLimit.val -gt 0) {
        $used = [Math]::Min(100.0, [double]$cfg.used.val / [double]$cfg.monthlyLimit.val * 100.0)
    }

    $periodType = $null
    $periodStart = $null
    $periodEnd = $null
    if ($cfg.currentPeriod) {
        $periodType = $cfg.currentPeriod.type
        $periodStart = $cfg.currentPeriod.start
        $periodEnd = $cfg.currentPeriod.end
    }
    if (-not $periodStart) { $periodStart = $cfg.billingPeriodStart }
    if (-not $periodEnd) { $periodEnd = $cfg.billingPeriodEnd }

    $products = @{}
    if ($cfg.productUsage) {
        foreach ($p in $cfg.productUsage) {
            $name = [string]$p.product
            if (-not $name) { continue }
            $products[$name] = $p.usagePercent
        }
    }

    $prepaid = $null
    if ($cfg.prepaidBalance -and $null -ne $cfg.prepaidBalance.val) {
        $prepaid = [Math]::Abs([int64]$cfg.prepaidBalance.val) / 100.0
    }

    return [ordered]@{
        ok                  = $true
        source              = "cli-chat-proxy-billing"
        auth_source         = $Source
        captured_at_utc     = [DateTimeOffset]::UtcNow.ToString("o")
        used_percent        = $used
        remaining_percent   = if ($null -ne $used) { [Math]::Round(100.0 - $used, 2) } else { $null }
        period_type         = $periodType
        period_start_utc    = $periodStart
        reset_at_utc        = $periodEnd
        is_unified_billing  = $cfg.isUnifiedBillingUser
        prepaid_credits_usd = $prepaid
        product_usage       = $products
        subscription_tier   = $Billing.subscription_tier
    }
}

try {
    $pick = Get-Auth
    $billing = Invoke-BillingCredits -Auth $pick.auth -Base $ProxyBase
    $report = Convert-BillingToReport -Billing $billing -Source $pick.source

    if (-not $NoSave) {
        if (-not (Test-Path $BudgetDir)) {
            New-Item -ItemType Directory -Path $BudgetDir | Out-Null
        }
        # Snapshot: usage numbers only - never write API keys.
        $snap = [ordered]@{
            captured_at_utc   = $report.captured_at_utc
            source            = "cli-chat-proxy-billing"
            used_percent      = $report.used_percent
            remaining_percent = $report.remaining_percent
            reset_at_utc      = $report.reset_at_utc
            period_start_utc  = $report.period_start_utc
            period_type       = $report.period_type
            product_breakdown = $report.product_usage
            extra_credits_usd = $report.prepaid_credits_usd
            raw_note          = "usage % only; secrets stay in .env (gitignored)"
        }
        ($snap | ConvertTo-Json -Depth 8) | Set-Content -Path $SnapshotPath -Encoding utf8

        if (Test-Path $ConfigPath) {
            $cfgFile = Get-Content -Raw $ConfigPath -Encoding UTF8 | ConvertFrom-Json
            if ($report.reset_at_utc) {
                $cfgFile | Add-Member -NotePropertyName reset_at_utc -NotePropertyValue $report.reset_at_utc -Force
            }
            if ($report.period_start_utc) {
                $cfgFile | Add-Member -NotePropertyName period_start_utc -NotePropertyValue $report.period_start_utc -Force
            }
            if ($report.period_start_utc -and $report.reset_at_utc) {
                try {
                    $s = [DateTimeOffset]::Parse([string]$report.period_start_utc)
                    $e = [DateTimeOffset]::Parse([string]$report.reset_at_utc)
                    $days = [Math]::Max(1, [Math]::Round(($e - $s).TotalDays))
                    $cfgFile | Add-Member -NotePropertyName period_days -NotePropertyValue $days -Force
                } catch { }
            }
            ($cfgFile | ConvertTo-Json -Depth 6) | Set-Content -Path $ConfigPath -Encoding utf8
        }
        $report["snapshot_path"] = $SnapshotPath
    }

    if ($Json) {
        # Never dump key into JSON output.
        $safe = [ordered]@{}
        foreach ($k in $report.Keys) {
            if ($k -eq "auth_source") { $safe[$k] = $report[$k]; continue }
            $safe[$k] = $report[$k]
        }
        $safe | ConvertTo-Json -Depth 8
    }
    else {
        Write-Host "=== Grok usage (live) ===" -ForegroundColor Cyan
        Write-Host "Auth source:   $($report.auth_source) (key not printed)"
        Write-Host "Used:          $($report.used_percent)%"
        Write-Host "Remaining:     $($report.remaining_percent)%"
        Write-Host "Period type:   $($report.period_type)"
        Write-Host "Period start:  $($report.period_start_utc)"
        Write-Host "Next reset:    $($report.reset_at_utc)"
        Write-Host "Unified pool:  $($report.is_unified_billing)"
        if ($report.product_usage -and $report.product_usage.Count -gt 0) {
            Write-Host "By product:"
            foreach ($k in $report.product_usage.Keys) {
                Write-Host ("  {0,-16} {1}%" -f $k, $report.product_usage[$k])
            }
        }
        if (-not $NoSave) {
            Write-Host "Saved:         $SnapshotPath" -ForegroundColor Green
        }
    }
    exit 0
}
catch {
    $err = $_.Exception.Message
    if ($Json) {
        @{ ok = $false; error = $err } | ConvertTo-Json
    }
    else {
        Write-Host "Failed to fetch usage: $err" -ForegroundColor Red
        Write-Host "See README.md - use .env (gitignored), never commit secrets."
    }
    exit 1
}
