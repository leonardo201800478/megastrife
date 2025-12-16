# build.ps1 - Build script for MegaStrife
param(
    [switch]$Release = $false,
    [switch]$Clean = $false,
    [switch]$Run = $false,
    [string]$Rom = "roms/test/Sonic The Hedgehog (USA, Europe).md"
)

$ProjectName = "megastrife"
$ExeName = "$ProjectName.exe"

Write-Host "=== MegaStrife Build System ===" -ForegroundColor Cyan
Write-Host ""

# Clean if requested
if ($Clean) {
    Write-Host "Cleaning build..." -ForegroundColor Yellow
    cargo clean
    Remove-Item -Path "dist" -Recurse -ErrorAction Ignore
    Write-Host "Clean complete." -ForegroundColor Green
    exit 0
}

# Build
if ($Release) {
    Write-Host "Building RELEASE version..." -ForegroundColor Green
    cargo build --release
    
    # Copy executable
    Copy-Item "target\release\$ExeName" ".\$ExeName" -Force
    Write-Host "Executable: $ExeName" -ForegroundColor Green
} else {
    Write-Host "Building DEBUG version..." -ForegroundColor Cyan
    cargo build
    
    # Copy executable
    Copy-Item "target\debug\$ExeName" ".\$ExeName" -Force
    Write-Host "Executable: $ExeName" -ForegroundColor Cyan
}

# Run if requested
if ($Run) {
    Write-Host "`nRunning emulator with ROM: $Rom" -ForegroundColor Magenta
    
    if ($Release) {
        & ".\$ExeName" $Rom
    } else {
        cargo run -- $Rom
    }
}

Write-Host "`nBuild completed successfully!" -ForegroundColor Green