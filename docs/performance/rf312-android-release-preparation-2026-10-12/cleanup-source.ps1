$ErrorActionPreference = 'Stop'
$taskBase = 'D:\SoloSoul\build\android-native-feasibility-20261010'
$taskStage = Join-Path $taskBase 'rf312-android-release-build-20261012'
$plan = Get-Content -Raw -LiteralPath (Join-Path $taskBase 'rf312-android-release-copy-cleanup-plan.json') | ConvertFrom-Json
$publicRoot = 'D:\SoloSoul\docs\performance\rf312-android-release-preparation-2026-10-12'
$summary = Get-Content -Raw -LiteralPath ($publicRoot + '.json') | ConvertFrom-Json
if (!$summary.accepted -or $summary.nativeRuntimeAccepted -or $plan.files.Count -ne 6) { throw 'Unexpected preparation/cleanup scope' }
if ((Get-FileHash -LiteralPath (Join-Path $publicRoot 'raw.tar.gz') -Algorithm SHA256).Hash.ToLowerInvariant() -ne $summary.archive.sha256) { throw 'Archive changed' }
$package = Get-Content -Raw -LiteralPath (Join-Path $taskStage 'package-verification.json') | ConvertFrom-Json
if ((Get-FileHash -LiteralPath $package.apk.path -Algorithm SHA256).Hash.ToLowerInvariant() -ne $package.apk.sha256) { throw 'Retained signed APK differs' }
if ((Get-FileHash -LiteralPath $summary.nativeReleaseLibrary.copy -Algorithm SHA256).Hash.ToLowerInvariant() -ne $summary.nativeReleaseLibrary.sha256) { throw 'Retained Release library differs' }
$expected = @((Join-Path $taskStage 'generated-jni-before.so'))
foreach ($number in 1..5) { $expected += Join-Path $taskStage ('gradle-packaging-attempt' + $number + '\jni\x86_64\libsolo_soul.so') }
foreach ($row in $plan.files) {
    $path = [IO.Path]::GetFullPath($row.path)
    if ($path -notin $expected -or !$path.StartsWith($taskStage + '\', [StringComparison]::OrdinalIgnoreCase)) { throw 'Outside exact owned targets' }
    $item = Get-Item -LiteralPath $path -Force
    if ($item.PSIsContainer -or $item.Length -ne $row.bytes) { throw 'Target size/type mismatch' }
    $ancestor = $item
    while ($ancestor) {
        if ($ancestor.Attributes -band [IO.FileAttributes]::ReparsePoint) { throw 'Reparse ancestor refused' }
        $ancestor = if ($ancestor -is [IO.FileInfo]) { $ancestor.Directory } else { $ancestor.Parent }
    }
    if ((Get-FileHash -LiteralPath $path -Algorithm SHA256).Hash.ToLowerInvariant() -ne $row.sha256) { throw 'Copy identity mismatch' }
}
$oldJni = 'D:\SoloSoul\tauri\src-tauri\gen\android\app\src\main\jniLibs\x86_64\libsolo_soul.so'
$initial = Get-Content -Raw -LiteralPath (Join-Path $taskStage 'input-proof.json') | ConvertFrom-Json
if ((Get-FileHash -LiteralPath $oldJni -Algorithm SHA256).Hash.ToLowerInvariant() -ne $initial.previousGeneratedJniSha256) { throw 'Original Debug JNI not restored' }
foreach ($row in $plan.files) { Remove-Item -LiteralPath $row.path -Force }
foreach ($row in $plan.files) { if (Test-Path -LiteralPath $row.path) { throw 'Copy still exists' } }
$receipt = [ordered]@{task='RF-312';exactTemporaryBinaryCopiesRemoved=6;recursiveDeleteUsed=$false;allAncestorsNonReparse=$true;archiveVerified=$true;originalGeneratedDebugJniPreserved=$true;releaseLibraryAndApksPreserved=$true;files=$plan.files;atUtc=[DateTime]::UtcNow.ToString('o')}
$receipt | ConvertTo-Json -Depth 6 | Set-Content -LiteralPath (Join-Path $publicRoot 'cleanup.json') -Encoding utf8
Copy-Item -LiteralPath $PSCommandPath -Destination (Join-Path $publicRoot 'cleanup-source.ps1')
Write-Output 'Six exact redundant binary copies removed; original Debug JNI, Release ELF and APKs retained'
