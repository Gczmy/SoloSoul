$ErrorActionPreference = 'Stop'
$taskRoot = 'D:\SoloSoul\build\android-native-feasibility-20261010'
$taskPublic = 'D:\SoloSoul\docs\verification\rf121-api31-console-font-restore-stop-2026-10-12'
$taskSummary = Get-Content -LiteralPath ($taskPublic + '.json') -Raw | ConvertFrom-Json
$taskPlan = Get-Content -LiteralPath (Join-Path $taskRoot 'rf121-api31-font-stop-cleanup-plan.json') -Raw | ConvertFrom-Json
$taskScopes = @('api31-home-console-20261012','api31-home-console-full-20261012') | ForEach-Object { [System.IO.Path]::GetFullPath((Join-Path $taskRoot $_)) + '\' }
if ((Get-FileHash -LiteralPath (Join-Path $taskPublic 'raw.tar.gz') -Algorithm SHA256).Hash.ToLowerInvariant() -ne $taskSummary.archive.sha256) { throw 'Archive bytes differ' }
if ($taskPlan.files.Count -ne 47) { throw 'Expected exactly 47 verified private snapshot files' }
$taskSeen = @{}
foreach ($taskFile in $taskPlan.files) {
    $taskPath = [System.IO.Path]::GetFullPath($taskFile.path)
    if (-not @($taskScopes | Where-Object { $taskPath.StartsWith($_,[System.StringComparison]::OrdinalIgnoreCase) }).Count) { throw 'Target outside exact owned stages' }
    if ($taskSeen.ContainsKey($taskPath)) { throw 'Duplicate cleanup target' }
    $taskSeen[$taskPath] = $true
    $taskItem = Get-Item -LiteralPath $taskPath -Force
    if ($taskItem.PSIsContainer -or $taskItem.Name -notin @('private-data-before.tar','private-data-restored.tar','private-data-after-compensation.tar')) { throw 'Unexpected private snapshot target' }
    $taskAncestor = $taskItem
    while ($null -ne $taskAncestor) {
        if ($taskAncestor.Attributes -band [System.IO.FileAttributes]::ReparsePoint) { throw 'Reparse ancestor refused' }
        $taskAncestor = if ($taskAncestor -is [System.IO.FileInfo]) { $taskAncestor.Directory } else { $taskAncestor.Parent }
    }
    if ($taskItem.Length -ne $taskFile.bytes -or (Get-FileHash -LiteralPath $taskPath -Algorithm SHA256).Hash.ToLowerInvariant() -ne $taskFile.sha256) { throw 'Snapshot identity differs' }
}
foreach ($taskFile in $taskPlan.files) { Remove-Item -LiteralPath $taskFile.path -Force }
foreach ($taskFile in $taskPlan.files) { if (Test-Path -LiteralPath $taskFile.path) { throw 'Target still exists' } }
[ordered]@{ deletedExactFiles=47; recursive=$false; allAncestorsChecked=$true; archiveVerified=$true; emulatorStopped=$false; emulatorRetainedForNextOwnedRestorationTask=$true; files=$taskPlan.files; at=[DateTimeOffset]::UtcNow.ToString('o') } | ConvertTo-Json -Depth 6 | Set-Content -LiteralPath (Join-Path $taskPublic 'cleanup.json') -Encoding utf8
Write-Output '47 exact verified private snapshots removed; native failure retained; owned emulator retained'
