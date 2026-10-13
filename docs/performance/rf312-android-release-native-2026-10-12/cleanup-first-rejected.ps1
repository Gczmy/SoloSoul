$ErrorActionPreference = 'Stop'
$taskBase = 'D:\SoloSoul\build\android-native-feasibility-20261010'
$taskNative = Join-Path $taskBase 'rf312-android-release-native-20261012'
$taskControl = Join-Path $taskBase 'rf312-probe-debug-control-20261012'
$taskEvidence = 'D:\SoloSoul\docs\performance\rf312-android-release-native-2026-10-12'
$taskIndex = Get-Content -LiteralPath (Join-Path $taskEvidence 'index.json') -Raw | ConvertFrom-Json
if ((Get-FileHash -LiteralPath (Join-Path $taskEvidence 'raw.tar.gz') -Algorithm SHA256).Hash.ToLowerInvariant() -ne $taskIndex.rawSha256) { throw 'Archive SHA changed' }
$taskPairs = Get-Content -LiteralPath (Join-Path $taskNative 'private-pairs-verification.json') -Raw | ConvertFrom-Json
if (-not $taskPairs.accepted -or $taskPairs.pairs.Count -ne 8 -or $taskPairs.privateTarCount -ne 16) { throw 'Private restoration not verified' }
if (Get-CimInstance Win32_Process -Filter 'ProcessId = 15768') { throw 'Owned emulator identity still live or reused' }
$taskPlan = @($taskPairs.pairs | ForEach-Object { $_.files })
if ($taskPlan.Count -ne 16 -or @($taskPlan.path | Select-Object -Unique).Count -ne 16) { throw 'Cleanup targets not exactly unique' }
foreach ($taskFile in $taskPlan) {
    $taskResolved = [IO.Path]::GetFullPath($taskFile.path)
    $taskRoot = @($taskNative, $taskControl) | Where-Object { $taskResolved.StartsWith($_ + '\', [StringComparison]::OrdinalIgnoreCase) }
    if (@($taskRoot).Count -ne 1 -or [IO.Path]::GetFileName($taskResolved) -notin @('private-data-before.tar','private-data-restored.tar')) { throw 'Cleanup target outside exact task roots' }
    $taskItem = Get-Item -LiteralPath $taskResolved -Force
    if ($taskItem.PSIsContainer -or $taskItem.Length -ne $taskFile.bytes) { throw 'Cleanup target type/length changed' }
    $taskAncestor = $taskItem
    while ($null -ne $taskAncestor) {
        if (($taskAncestor.Attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0) { throw 'Reparse point in cleanup path' }
        if ($taskAncestor.FullName -eq $taskRoot[0]) { break }
        $taskAncestor = if ($taskAncestor.PSIsContainer) { $taskAncestor.Parent } else { $taskAncestor.Directory }
    }
    if ($null -eq $taskAncestor) { throw 'Cleanup root not reached' }
    if ((Get-FileHash -LiteralPath $taskResolved -Algorithm SHA256).Hash.ToLowerInvariant() -ne $taskFile.sha256) { throw 'Cleanup target SHA changed' }
}
foreach ($taskFile in $taskPlan) {
    Remove-Item -LiteralPath ([IO.Path]::GetFullPath($taskFile.path)) -Force
    if (Test-Path -LiteralPath $taskFile.path) { throw 'Exact cleanup did not complete' }
}
$taskReceipt = [ordered]@{ removedCount=16; removedFiles=$taskPlan; allEightRestorationsVerified=$true; archiveSha256=$taskIndex.rawSha256; nonRecursive=$true; allAncestorsNonReparse=$true; originalAvdAndAppDataPreserved=$true; at=[DateTime]::UtcNow.ToString('o') }
$taskReceipt | ConvertTo-Json -Depth 6 | Set-Content -LiteralPath (Join-Path $taskEvidence 'cleanup.json') -Encoding UTF8
Copy-Item -LiteralPath $PSCommandPath -Destination (Join-Path $taskEvidence 'cleanup-source.ps1')
Write-Output 'Removed 16 exact verified redundant private TAR copies; original AVD/app data and APK artifacts preserved'
