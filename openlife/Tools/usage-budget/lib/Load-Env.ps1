# Shared helpers: load .env into process env (never logs secret values).

function Import-DotEnv {
    param(
        [Parameter(Mandatory = $true)]
        [string]$Path
    )
    if (-not (Test-Path -LiteralPath $Path)) {
        return $false
    }
    $lines = Get-Content -LiteralPath $Path -Encoding UTF8
    foreach ($raw in $lines) {
        $line = $raw.Trim()
        if (-not $line -or $line.StartsWith("#")) { continue }
        if ($line -match '^\s*export\s+') {
            $line = $line -replace '^\s*export\s+', ''
        }
        $eq = $line.IndexOf("=")
        if ($eq -lt 1) { continue }
        $name = $line.Substring(0, $eq).Trim()
        $value = $line.Substring($eq + 1).Trim()
        if (
            ($value.StartsWith('"') -and $value.EndsWith('"')) -or
            ($value.StartsWith("'") -and $value.EndsWith("'"))
        ) {
            $value = $value.Substring(1, $value.Length - 2)
        }
        if ($name -notmatch '^[A-Za-z_][A-Za-z0-9_]*$') { continue }
        # Process-scope only - does not write to user/machine env permanently.
        Set-Item -Path "Env:$name" -Value $value
    }
    return $true
}

function Get-UsageAuthFromEnv {
    <#
      Prefer process env (after Import-DotEnv):
        GROK_API_KEY  (or GROK_BEARER_TOKEN)
        GROK_USER_ID
      Returns hashtable @{ key; user_id } or $null.
    #>
    $key = $env:GROK_API_KEY
    if (-not $key) { $key = $env:GROK_BEARER_TOKEN }
    $uid = $env:GROK_USER_ID
    if (-not $key -or -not $uid) {
        return $null
    }
    if ([string]::IsNullOrWhiteSpace($key) -or [string]::IsNullOrWhiteSpace($uid)) {
        return $null
    }
    return @{
        key     = $key.Trim()
        user_id = $uid.Trim()
    }
}

function Mask-Secret {
    param([string]$Value, [int]$Keep = 4)
    if (-not $Value) { return "(empty)" }
    if ($Value.Length -le ($Keep * 2)) { return "***" }
    return $Value.Substring(0, $Keep) + "..." + $Value.Substring($Value.Length - $Keep)
}
