# ALWAYS resolve repo root from script location
$repoRoot = Split-Path $PSScriptRoot -Parent
Set-Location $repoRoot

# Vars to clear (process-only, safe for CLI use)
$vars = @(
    "COPILOT_PROVIDER_BASE_URL",
    "COPILOT_PROVIDER_TYPE",
    "COPILOT_PROVIDER_API_KEY",
    "COPILOT_MODEL",
    "COPILOT_OFFLINE"
)

foreach ($var in $vars) {
    [System.Environment]::SetEnvironmentVariable($var, $null, "Process")
}

Write-Host "BYOK Disabled: back to default Copilot models"

copilot