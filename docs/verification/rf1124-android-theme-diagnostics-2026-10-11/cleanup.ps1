$ErrorActionPreference = 'Stop'
$taskRoot = [IO.Path]::GetFullPath('D:\SoloSoul\build\android-native-feasibility-20261010')
$plan = Get-Content -LiteralPath "$taskRoot\rf1124-owned-cleanup-plan.json" -Raw | ConvertFrom-Json
if ($plan.task -ne 'RF-1124' -or -not $plan.privateFilesRestored -or -not $plan.emulatorsExited -or $plan.targets.Count -ne 39) { throw 'Invalid cleanup plan' }
foreach ($target in $plan.targets) {
    $resolved = [IO.Path]::GetFullPath($target.path)
    if (-not $resolved.StartsWith($taskRoot + '\', [StringComparison]::OrdinalIgnoreCase)) { throw 'Outside owned directory' }
    $item = Get-Item -LiteralPath $resolved
    if ($item.PSIsContainer -or $item.Length -ne $target.bytes) { throw 'Not exact regular file' }
    $cursor = $item
    while ($null -ne $cursor) {
        if (($cursor.Attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0) { throw 'Reparse ancestor' }
        $cursor = if ($cursor.PSIsContainer) { $cursor.Parent } else { $cursor.Directory }
    }
    if ((Get-FileHash -LiteralPath $resolved -Algorithm SHA256).Hash.ToLowerInvariant() -ne $target.sha256) { throw 'Changed file' }
}
$archive = 'D:\SoloSoul\docs\verification\rf1124-android-theme-diagnostics-2026-10-11\raw.tar.gz'
if ((Get-FileHash -LiteralPath $archive -Algorithm SHA256).Hash.ToLowerInvariant() -ne $plan.archiveSha256) { throw 'Archive changed' }
$removed = @()
foreach ($target in $plan.targets) {
    Remove-Item -LiteralPath $target.path
    if (Test-Path -LiteralPath $target.path) { throw 'Removal incomplete' }
    $removed += $target
}
$receipt = [ordered]@{task='RF-1124';removedFiles=$removed.Count;removedBytes=($removed | Measure-Object bytes -Sum).Sum;removed=$removed;privatePairs=$plan.privatePairs;archiveSha256=$plan.archiveSha256;recursiveRemoval=$false;userProcessesTerminated=0;ownedEmulatorsExited=$true;at=[DateTime]::UtcNow.ToString('o')}
$output = 'D:\SoloSoul\docs\verification\rf1124-android-theme-diagnostics-2026-10-11\cleanup.json'
if (Test-Path -LiteralPath $output) { throw 'Receipt already exists' }
[IO.File]::WriteAllText($output, ($receipt | ConvertTo-Json -Depth 8) + "`n", [Text.UTF8Encoding]::new($false))
Write-Output "Removed 39 verified owned temporary files; archive and public logs retained"
