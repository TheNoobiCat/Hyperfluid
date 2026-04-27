# if you are wondering why i made this stupid script you can thank github and microslop for ratelimiting me for absolutely no reason
# this is for when i need to switch over to cheap chinese models after the ratelimits
# i regret even bothering to try the gh copilot cli i should have just stuck with opencode
# maybe i will switch back to that later

# ALWAYS resolve repo root from script location
$repoRoot = Split-Path $PSScriptRoot -Parent
Set-Location $repoRoot

# .env lives in .github
$envPath = Join-Path $PSScriptRoot ".env"

if (!(Test-Path $envPath)) {
    Write-Host ".env file not found in .github"
    exit 1
}

# Load .env
Get-Content $envPath | ForEach-Object {
    if ($_ -match "^\s*#") { return } # comments
    if ($_ -match "^\s*$") { return } # empty lines

    $key, $value = $_ -split "=", 2
    [System.Environment]::SetEnvironmentVariable($key, $value, "Process")
}

# Validate required vars
if (-not $env:COPILOT_PROVIDER_BASE_URL -or -not $env:COPILOT_MODEL) {
    Write-Host "Missing required variables in .env"
    exit 1
}

Write-Host "BYOK Enabled"
Write-Host "Provider: $env:COPILOT_PROVIDER_TYPE"
Write-Host "Model: $env:COPILOT_MODEL"

copilot