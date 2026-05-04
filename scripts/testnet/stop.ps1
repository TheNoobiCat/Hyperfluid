# Hyperfluid Local Testnet — Stop Script
# Sends Ctrl+C to the running node process.

Write-Host "=== Stopping Hyperfluid Testnet ===" -ForegroundColor Cyan

$process = Get-Process -Name "hyperfluid-node" -ErrorAction SilentlyContinue

if ($process) {
    Write-Host "Sending graceful shutdown signal to hyperfluid-node (PID: $($process.Id))..." -ForegroundColor Yellow
    $process.CloseMainWindow() | Out-Null
    Start-Sleep -Seconds 2
    if (!$process.HasExited) {
        Write-Host "Force stopping..." -ForegroundColor Red
        $process.Kill()
    }
    Write-Host "Node stopped." -ForegroundColor Green
} else {
    Write-Host "No hyperfluid-node process found. Testnet is not running." -ForegroundColor Yellow
}
