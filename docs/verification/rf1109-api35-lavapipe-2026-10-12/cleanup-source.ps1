$ErrorActionPreference = 'Stop'
$taskRoot = 'D:\SoloSoul\build\android-native-feasibility-20261010'
$taskStage = Join-Path $taskRoot 'api35-home-lavapipe-20261012'
$taskPublic = 'D:\SoloSoul\docs\verification\rf1109-api35-lavapipe-2026-10-12'
$taskPlan = Get-Content -LiteralPath (Join-Path $taskRoot 'rf1109-lavapipe-cleanup-plan.json') -Raw | ConvertFrom-Json
$taskIndex = Get-Content -LiteralPath (Join-Path $taskPublic 'index.json') -Raw | ConvertFrom-Json
if ((Get-FileHash -LiteralPath (Join-Path $taskPublic 'raw.tar.gz') -Algorithm SHA256).Hash.ToLowerInvariant() -ne $taskIndex.archiveSha256) { throw 'Archive identity differs' }
if ($taskPlan.files.Count -ne 8) { throw 'Expected four exact private backup pairs' }
$taskResolved = [System.IO.Path]::GetFullPath($taskStage) + '\'
foreach ($taskFile in $taskPlan.files) {
    $taskPath = [System.IO.Path]::GetFullPath($taskFile.path)
    if (-not $taskPath.StartsWith($taskResolved, [System.StringComparison]::OrdinalIgnoreCase)) { throw 'Target outside owned stage' }
    $taskItem = Get-Item -LiteralPath $taskPath -Force
    if ($taskItem.PSIsContainer -or $taskItem.Name -notin @('private-data-before.tar','private-data-restored.tar')) { throw 'Unexpected target' }
    $taskAncestor = $taskItem
    while ($null -ne $taskAncestor) {
        if ($taskAncestor.Attributes -band [System.IO.FileAttributes]::ReparsePoint) { throw 'Reparse ancestor refused' }
        $taskAncestor = if ($taskAncestor -is [System.IO.FileInfo]) { $taskAncestor.Directory } else { $taskAncestor.Parent }
    }
    if ($taskItem.Length -ne $taskFile.bytes -or (Get-FileHash -LiteralPath $taskPath -Algorithm SHA256).Hash.ToLowerInvariant() -ne $taskFile.sha256) { throw 'Backup identity differs' }
}
foreach ($taskFile in $taskPlan.files) { Remove-Item -LiteralPath $taskFile.path -Force }
foreach ($taskFile in $taskPlan.files) { if (Test-Path -LiteralPath $taskFile.path) { throw 'Target still exists' } }
$taskReceipt = [ordered]@{ deletedExactFiles=8; recursive=$false; allAncestorsChecked=$true; archiveVerified=$true; files=$taskPlan.files; at=[DateTimeOffset]::UtcNow.ToString('o') }
$taskReceipt | ConvertTo-Json -Depth 6 | Set-Content -LiteralPath (Join-Path $taskPublic 'cleanup.json') -Encoding utf8
Write-Output 'Eight exact owned private backups removed after archive and restoration verification'
