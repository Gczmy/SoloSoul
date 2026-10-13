$ErrorActionPreference = 'Stop'
$taskRoot = 'D:\SoloSoul\build\android-native-feasibility-20261010'
$plan = Get-Content -Raw -LiteralPath (Join-Path $taskRoot 'rf1132-private-cleanup-plan.json') | ConvertFrom-Json
$stage = [IO.Path]::GetFullPath($plan.ownedStage).TrimEnd('\')
if ($stage -ne 'D:\SoloSoul\build\android-native-feasibility-20261010\rf1132-card-high-contrast-native-20261012') { throw 'Unexpected owned stage' }
$published = 'D:\SoloSoul\docs\verification\rf1132-android-card-high-text-contrast-2026-10-12'
$summary = Get-Content -Raw -LiteralPath ($published + '.json') | ConvertFrom-Json
$stop = Get-Content -Raw -LiteralPath (Join-Path $taskRoot 'api31-host-rf1132-20261012/rf1132-stop.receipt.json') | ConvertFrom-Json
if (!$summary.accepted -or !$stop.ownedProcessExited -or !$stop.originalSystemStillExact) { throw 'Missing verified restoration/exit' }
if ((Get-FileHash -LiteralPath (Join-Path $published 'raw.tar.gz') -Algorithm SHA256).Hash.ToLowerInvariant() -ne $summary.archive.sha256) { throw 'Published archive differs' }
if (Get-CimInstance Win32_Process -Filter 'ProcessId=27820') { throw 'Owned process ID still present; inspect identity' }
if ($plan.files.Count -ne 2) { throw 'Unexpected cleanup count' }
$verified = @()
foreach ($row in $plan.files) {
    $absolute = [IO.Path]::GetFullPath($row.path)
    if (!$absolute.StartsWith($stage + '\', [StringComparison]::OrdinalIgnoreCase)) { throw 'Outside owned stage' }
    if ([IO.Path]::GetFileName($absolute) -notin @('private-before.tar', 'private-restored.tar')) { throw 'Unexpected file' }
    $ancestor = Get-Item -LiteralPath $absolute -Force
    while ($ancestor) {
        if (($ancestor.Attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0) { throw 'Reparse ancestor' }
        if ($ancestor -is [IO.FileInfo]) { $ancestor = $ancestor.Directory } else { $ancestor = $ancestor.Parent }
    }
    $item = Get-Item -LiteralPath $absolute -Force
    if ($item.PSIsContainer -or $item.Length -ne $row.bytes) { throw 'Size/type mismatch' }
    if ((Get-FileHash -LiteralPath $absolute -Algorithm SHA256).Hash.ToLowerInvariant() -ne $row.sha256) { throw 'Snapshot hash mismatch' }
    $verified += [ordered]@{path=$absolute;sha256=$row.sha256;bytes=$row.bytes;allAncestorsNonReparse=$true}
}
foreach ($row in $verified) { Remove-Item -LiteralPath $row.path -Force }
foreach ($row in $verified) { if (Test-Path -LiteralPath $row.path) { throw 'Deletion not confirmed' } }
$receipt = [ordered]@{task='RF-1132';onlyExactPrivateCopiesRemoved=$true;removedCount=2;files=$verified;publishedArchiveHashVerified=$true;ownedEmulatorAbsent=$true;nativeAvdDataUnchanged=$true;recursiveDeleteUsed=$false;atUtc=[DateTime]::UtcNow.ToString('o')}
$receipt | ConvertTo-Json -Depth 6 | Set-Content -LiteralPath (Join-Path $published 'cleanup.json') -Encoding utf8
Copy-Item -LiteralPath $PSCommandPath -Destination (Join-Path $published 'cleanup-source.ps1')
Write-Output 'RF1132 cleanup verified: only two exact local private TAR copies removed'
