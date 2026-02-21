# Jarvis Automated Build Script (PowerShell)

$ErrorActionPreference = "Stop"

Write-Host "--- Jarvis Build System ---" -ForegroundColor Cyan

# 1. Check for Rust
if (!(Get-Command "cargo" -ErrorAction SilentlyContinue)) {
    Write-Error "Cargo not found! Please install Rust from https://rustup.rs"
}
Write-Host "[OK] Rust found." -ForegroundColor Green

# 2. Check for Windows SDK (kernel32.lib)
$SDKPath = "C:\Program Files (x86)\Windows Kits\10\Lib"
if (!(Test-Path $SDKPath)) {
    Write-Warning "Windows SDK path not found at $SDKPath."
    Write-Host "Please install Windows SDK: https://go.microsoft.com/fwlink/?linkid=2349110" -ForegroundColor Yellow
}
else {
    $kernel32 = Get-ChildItem -Path $SDKPath -Filter "kernel32.lib" -Recurse -File -ErrorAction SilentlyContinue | Where-Object { $_.FullName -like "*\um\x64\*" } | Select-Object -First 1
    if ($kernel32) {
        Write-Host "[OK] Windows SDK found at $($kernel32.DirectoryName)" -ForegroundColor Green
        # Set LIB for current session if not already there
        if (!($env:LIB -like "*$($kernel32.DirectoryName)*")) {
            $env:LIB += ";$($kernel32.DirectoryName)"
        }
    }
    else {
        Write-Error "kernel32.lib not found in Windows Kits! Please reinstall Windows SDK with 'Desktop C++' components."
    }
}

# 3. Check for Interception Driver
$InterceptionDll = "C:\Windows\System32\drivers\interception.sys" # Check driver file
if (!(Test-Path $InterceptionDll)) {
    Write-Warning "Interception Driver (.sys) not found in System32. Input simulation may not work."
    Write-Host "Download and install driver from: https://github.com/oblitum/Interception/releases" -ForegroundColor Yellow
}
else {
    Write-Host "[OK] Interception Driver detected." -ForegroundColor Green
}

# 4. Final Build
Write-Host "Starting Cargo Build (win_mcp)..." -ForegroundColor Cyan
Set-Location "win_mcp"
cargo build --release

if ($LASTEXITCODE -eq 0) {
    Write-Host "`n[SUCCESS] Jarvis build complete!" -ForegroundColor Green
    Write-Host "Binary: d:\Rust\Win-Ai\win_mcp\target\release\win_mcp.exe"
}
else {
    Write-Error "Build failed. Check error logs above."
}
