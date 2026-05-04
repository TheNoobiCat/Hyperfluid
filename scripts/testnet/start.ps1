# Hyperfluid Local Testnet — Start Script
# Starts a single-validator node in the foreground.
# Output: genesis.json (if --gen-genesis is passed), then block production.

param(
    [switch]$GenGenesis,
    [string]$GenesisFile = "config\testnet-single.toml"
)

$ErrorActionPreference = "Stop"
Set-Location $PSScriptRoot\..

Write-Host "=== Hyperfluid Local Testnet ===" -ForegroundColor Cyan
Write-Host "Chain: hyperfluid-testnet-1"
Write-Host "Validators: 1 (local)"
Write-Host ""

$args = @()

if ($GenGenesis) {
    Write-Host "Generating genesis config..." -ForegroundColor Yellow
    $args += "--gen-genesis"
    $args += "--genesis"
    $args += $GenesisFile
}

Write-Host "Building workspace..." -ForegroundColor Yellow
cargo build --workspace --release
if ($LASTEXITCODE -ne 0) {
    Write-Host "Build failed!" -ForegroundColor Red
    exit 1
}

Write-Host "Starting node..." -ForegroundColor Green
cargo run --release --bin hyperfluid-node -- @args
