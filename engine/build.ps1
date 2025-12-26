# Build script for the WASM engine (Windows PowerShell)

$ErrorActionPreference = "Stop"

Write-Host "Building editor-engine WASM module..." -ForegroundColor Cyan

# Check if wasm-pack is installed
$wasmPackPath = Get-Command wasm-pack -ErrorAction SilentlyContinue
if (-not $wasmPackPath) {
    Write-Host "wasm-pack not found. Installing..." -ForegroundColor Yellow
    cargo install wasm-pack
}

# Build the WASM module
Push-Location $PSScriptRoot
try {
    wasm-pack build --target web --out-dir ../src/lib/engine-wasm
    Write-Host "Build complete! Output in src/lib/engine-wasm/" -ForegroundColor Green
}
finally {
    Pop-Location
}
