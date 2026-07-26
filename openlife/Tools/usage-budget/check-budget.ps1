<#
.SYNOPSIS
  Fail if Grok usage is above today's soft ceiling (or hard cap).
  Exit: 0 OK, 2 over budget, 3 no data, 1 error
#>
[CmdletBinding()]
param(
    [switch]$Offline,
    [switch]$AllowMissing
)

$read = Join-Path $PSScriptRoot "read-usage.ps1"
& $read -Offline:$Offline
$code = $LASTEXITCODE

if ($code -eq 3 -and $AllowMissing) {
    Write-Host "WARN: no usage data; -AllowMissing set, continuing." -ForegroundColor Yellow
    exit 0
}

if ($code -eq 0) {
    Write-Host "BUDGET OK - under soft ceiling." -ForegroundColor Green
}
elseif ($code -eq 2) {
    Write-Host "BUDGET STOP - over soft ceiling or hard cap." -ForegroundColor Red
}
elseif ($code -eq 3) {
    Write-Host "BUDGET UNKNOWN - run .\fetch-usage.ps1 after setting up .env" -ForegroundColor Yellow
}

exit $code
