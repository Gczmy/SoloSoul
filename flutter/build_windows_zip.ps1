# ============================================================
# SoloSoul Windows ZIP Builder (PowerShell)
# ============================================================
# Usage: .\build_windows_zip.ps1 1.1.0
# Run in PowerShell (not Git Bash), from the flutter/ directory.
# ============================================================

param(
    [string]$Version = "1.0.0"
)

$AppName = "SoloSoul"
$ZipName = "${AppName}-v${Version}-windows-x64"
$ReleaseDir = "build\windows\x64\runner\Release"
$ZipOutput = "build\windows\${ZipName}.zip"
$StagingDir = "build\windows\zip_staging"

Write-Host "========================================" -ForegroundColor Green
Write-Host "  SoloSoul Windows ZIP Builder" -ForegroundColor Green
Write-Host "========================================" -ForegroundColor Green

# --- Inject version into pubspec.yaml ---
Write-Host "Injecting version ${Version} into pubspec.yaml..." -ForegroundColor Yellow
$PubspecPath = "pubspec.yaml"
$OriginalVersionLine = (Get-Content $PubspecPath | Select-String "^version:").Line
(Get-Content $PubspecPath -Raw) -replace "^version: .*", "version: ${Version}+1" | Set-Content $PubspecPath -NoNewline

# Ensure cleanup on exit
try {
    # --- Clean previous artifacts ---
    Write-Host "Cleaning previous build artifacts..." -ForegroundColor Yellow
    if (Test-Path $ZipOutput) { Remove-Item $ZipOutput -Force }
    if (Test-Path $StagingDir) { Remove-Item $StagingDir -Recurse -Force }

    # --- Build Flutter app ---
    Write-Host "Building Flutter app for Windows..." -ForegroundColor Yellow
    flutter build windows --release --obfuscate --split-debug-info=.\debug_info\windows
    if ($LASTEXITCODE -ne 0) {
        throw "Flutter build failed with exit code $LASTEXITCODE"
    }

    # --- Verify build output ---
    $ExePath = Join-Path $ReleaseDir "solosoul_flutter.exe"
    if (-not (Test-Path $ExePath)) {
        throw "Build output not found at $ExePath"
    }

    # --- Stage files for ZIP ---
    Write-Host "Staging files for ZIP..." -ForegroundColor Yellow
    $DestDir = Join-Path $StagingDir $AppName
    New-Item -ItemType Directory -Path $DestDir -Force | Out-Null
    Copy-Item -Path (Join-Path $ReleaseDir "*") -Destination $DestDir -Recurse -Force

    # --- Create ZIP ---
    Write-Host "Creating ZIP archive..." -ForegroundColor Yellow
    Compress-Archive -Path "$DestDir\*" -DestinationPath $ZipOutput -Force

    Write-Host "Build Complete!" -ForegroundColor Green
    Write-Host "Output: $ZipOutput" -ForegroundColor Green
}
finally {
    # --- Restore pubspec.yaml ---
    Write-Host "Restoring pubspec.yaml version..." -ForegroundColor Yellow
    (Get-Content $PubspecPath -Raw) -replace "^version: .*", $OriginalVersionLine | Set-Content $PubspecPath -NoNewline

    # --- Clean staging ---
    if (Test-Path $StagingDir) {
        Remove-Item $StagingDir -Recurse -Force
    }
}
