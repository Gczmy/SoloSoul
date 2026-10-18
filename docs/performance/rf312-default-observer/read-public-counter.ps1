param([Parameter(Mandatory=$true)][string]$ImagePath,[Parameter(Mandatory=$true)][string]$OutputPath,[Parameter(Mandatory=$true)][string]$EvidenceRoot)
$ErrorActionPreference='Stop'
Add-Type -AssemblyName System.Runtime.WindowsRuntime
[void][Windows.Storage.StorageFile,Windows.Storage,ContentType=WindowsRuntime]
[void][Windows.Storage.FileAccessMode,Windows.Storage,ContentType=WindowsRuntime]
[void][Windows.Storage.Streams.IRandomAccessStream,Windows.Storage.Streams,ContentType=WindowsRuntime]
[void][Windows.Graphics.Imaging.BitmapDecoder,Windows.Graphics.Imaging,ContentType=WindowsRuntime]
[void][Windows.Graphics.Imaging.SoftwareBitmap,Windows.Graphics.Imaging,ContentType=WindowsRuntime]
[void][Windows.Media.Ocr.OcrEngine,Windows.Media.Ocr,ContentType=WindowsRuntime]
[void][Windows.Media.Ocr.OcrResult,Windows.Media.Ocr,ContentType=WindowsRuntime]
[void][Windows.Globalization.Language,Windows.Globalization,ContentType=WindowsRuntime]
$asTask=@([System.WindowsRuntimeSystemExtensions].GetMethods() | Where-Object {
 $_.Name -eq 'AsTask' -and $_.IsGenericMethod -and $_.GetGenericArguments().Count -eq 1 -and
 $_.GetParameters().Count -eq 1 -and $_.GetParameters()[0].ParameterType.Name -eq 'IAsyncOperation`1'
})
if($asTask.Count -ne 1){throw 'One WinRT async adapter required'}
function Await-Operation($operation,[Type]$resultType) {
 $task=$asTask[0].MakeGenericMethod($resultType).Invoke($null,@($operation))
 if(-not $task.Wait([TimeSpan]::FromSeconds(10))){throw 'Bounded native OCR operation timed out'}
 return $task.Result
}
$root=(Get-Item -LiteralPath $EvidenceRoot -Force)
if(-not $root.PSIsContainer){throw 'Existing evidence directory required'}
function Assert-OrdinaryParents([string]$path) {
 $current=Get-Item -LiteralPath $path -Force
 while($null -ne $current) {
  if($current.Attributes -band [IO.FileAttributes]::ReparsePoint){throw 'Ordinary evidence and parents required'}
  $parent=[IO.Directory]::GetParent($current.FullName)
  if($null -eq $parent){break}
  $current=Get-Item -LiteralPath $parent.FullName -Force
 }
}
Assert-OrdinaryParents $root.FullName
$prefix=$root.FullName.TrimEnd([IO.Path]::DirectorySeparatorChar,[IO.Path]::AltDirectorySeparatorChar)+[IO.Path]::DirectorySeparatorChar
$image=(Get-Item -LiteralPath $ImagePath -Force)
if($image.PSIsContainer -or -not $image.FullName.StartsWith($prefix,[StringComparison]::OrdinalIgnoreCase)){throw 'Image outside declared evidence root'}
Assert-OrdinaryParents $image.FullName
$output=[IO.Path]::GetFullPath($OutputPath)
if(-not $output.StartsWith($prefix,[StringComparison]::OrdinalIgnoreCase) -or (Test-Path -LiteralPath $output)){throw 'New output inside declared evidence root required'}
Assert-OrdinaryParents ([IO.Path]::GetDirectoryName($output))
function Read-Sha256([string]$path) {
 $algorithm=[Security.Cryptography.SHA256]::Create()
 $source=[IO.File]::OpenRead($path)
 try { return [BitConverter]::ToString($algorithm.ComputeHash($source)).Replace('-','').ToLowerInvariant() }
 finally { $source.Dispose();$algorithm.Dispose() }
}
$before=Read-Sha256 $image.FullName
$file=Await-Operation ([Windows.Storage.StorageFile]::GetFileFromPathAsync($image.FullName)) ([Windows.Storage.StorageFile])
$stream=$null;$bitmap=$null
try {
 $stream=Await-Operation ($file.OpenAsync([Windows.Storage.FileAccessMode]::Read)) ([Windows.Storage.Streams.IRandomAccessStream])
 $decoder=Await-Operation ([Windows.Graphics.Imaging.BitmapDecoder]::CreateAsync($stream)) ([Windows.Graphics.Imaging.BitmapDecoder])
 $bitmap=Await-Operation ($decoder.GetSoftwareBitmapAsync()) ([Windows.Graphics.Imaging.SoftwareBitmap])
 $engine=[Windows.Media.Ocr.OcrEngine]::TryCreateFromLanguage([Windows.Globalization.Language]::new('en-GB'))
 if($null -eq $engine){throw 'Native English recognizer unavailable'}
 $observed=Await-Operation ($engine.RecognizeAsync($bitmap)) ([Windows.Media.Ocr.OcrResult])
 $lines=@($observed.Lines | ForEach-Object {
  [ordered]@{Text=$_.Text;Words=@($_.Words | ForEach-Object {
   [ordered]@{Text=$_.Text;X=$_.BoundingRect.X;Y=$_.BoundingRect.Y;Width=$_.BoundingRect.Width;Height=$_.BoundingRect.Height}
  })}
 })
 $after=Read-Sha256 $image.FullName
 if($before -ne $after){throw 'Screenshot changed during native read'}
 Assert-OrdinaryParents $image.FullName
 Assert-OrdinaryParents ([IO.Path]::GetDirectoryName($output))
 $json=[ordered]@{Engine='Windows.Media.Ocr';Language='en-GB';ImageSha256=$before;
  PixelWidth=$decoder.PixelWidth;PixelHeight=$decoder.PixelHeight;Text=$observed.Text;Lines=$lines} | ConvertTo-Json -Depth 7
 $bytes=[Text.UTF8Encoding]::new($false).GetBytes($json)
 $destination=[IO.File]::Open($output,[IO.FileMode]::CreateNew,[IO.FileAccess]::Write,[IO.FileShare]::None)
 try {$destination.Write($bytes,0,$bytes.Length)} finally {$destination.Dispose()}
} finally {
 if($null -ne $bitmap){$bitmap.Dispose()}
 if($null -ne $stream){$stream.Dispose()}
}
