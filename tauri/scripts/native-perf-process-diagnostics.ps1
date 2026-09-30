# RF-312：仅诊断调用方已验证的自有进程；不启动、终止或修改进程。
# 输入仅来自 SOLOSOUL_NATIVE_PERF_DIAGNOSTICS_* 环境变量；离线模式不读取进程。
Set-StrictMode -Version 2.0
$ErrorActionPreference = 'Stop'
$diagnosticScope = 'windows-native-perf-process-diagnostics'
$diagnosticMode = [Environment]::GetEnvironmentVariable('SOLOSOUL_NATIVE_PERF_DIAGNOSTICS_MODE')
$diagnosticStage = 'input validation'
$nativeHandles = @()
$managedProcesses = @()
$liveQueriesPerformed = $false

function Write-Result($value, [int]$exitCode) {
    $value | ConvertTo-Json -Depth 12 -Compress | Write-Output
    exit $exitCode
}

function Assert-True([bool]$condition, [string]$reason) {
    if (!$condition) { throw $reason }
}

function Is-LocalAbsolutePath([string]$value) {
    if ([string]::IsNullOrWhiteSpace($value) -or $value.Contains([char]0)) { return $false }
    $plain = $value
    if ($plain.StartsWith('\\?\')) { $plain = $plain.Substring(4) }
    return ($plain -match '^[A-Za-z]:[\\/]') -and ($plain.IndexOf(':', 2) -lt 0)
}

function Normalize-LocalPath([string]$value) {
    Assert-True (Is-LocalAbsolutePath $value) 'Expected a local absolute Windows path'
    if ($value.StartsWith('\\?\')) { $value = $value.Substring(4) }
    return [IO.Path]::GetFullPath($value).TrimEnd('\').ToLowerInvariant()
}

function Parse-Owned([string]$json) {
    Assert-True (![string]::IsNullOrWhiteSpace($json)) 'Owned identity list is required'
    $trimmed = $json.Trim()
    Assert-True ($trimmed.StartsWith('[') -and $trimmed.EndsWith(']')) 'Owned identities must be a JSON array'
    try { $parsed = $json | ConvertFrom-Json } catch { throw 'Owned identity JSON is invalid' }
    $rows = @($parsed)
    Assert-True ($rows.Count -ge 1 -and $rows.Count -le 32) 'Owned identity count must be 1..32'
    $byPid = @{}
    foreach ($row in $rows) {
        Assert-True ($null -ne $row -and $row -is [System.Management.Automation.PSCustomObject]) 'Owned identity must be an object'
        $names = @($row.PSObject.Properties | ForEach-Object { $_.Name })
        Assert-True ($names.Count -eq 4) 'Owned identity must contain exactly four identity fields'
        foreach ($name in @('pid', 'parentPid', 'creationMs', 'executablePath')) {
            Assert-True ($names -ccontains $name) 'Owned identity fields are invalid'
        }
        Assert-True (($row.pid -is [int] -or $row.pid -is [long]) -and $row.pid -gt 0 -and $row.pid -le [int]::MaxValue) 'Owned PID is invalid'
        Assert-True (($row.parentPid -is [int] -or $row.parentPid -is [long]) -and $row.parentPid -ge 0 -and $row.parentPid -le [int]::MaxValue -and $row.parentPid -ne $row.pid) 'Owned parent PID is invalid'
        Assert-True (($row.creationMs -is [int] -or $row.creationMs -is [long]) -and $row.creationMs -gt 0 -and $row.creationMs -le 32503680000000) 'Owned creation time is invalid'
        Assert-True ($row.executablePath -is [string] -and (Is-LocalAbsolutePath $row.executablePath) -and [IO.Path]::GetExtension($row.executablePath) -ieq '.exe') 'Owned executable path is invalid'
        Assert-True (!$byPid.ContainsKey([int]$row.pid)) 'Duplicate owned PID'
        $byPid[[int]$row.pid] = $row
    }
    $roots = @($rows | Where-Object { !$byPid.ContainsKey([int]$_.parentPid) })
    Assert-True ($roots.Count -eq 1) 'Owned identities must describe one connected process tree'
    $rootPid = [int]$roots[0].pid
    foreach ($row in $rows) {
        $seen = @{}
        $cursor = [int]$row.pid
        while ($cursor -ne $rootPid) {
            Assert-True (!$seen.ContainsKey($cursor) -and $byPid.ContainsKey($cursor)) 'Owned process tree is disconnected or cyclic'
            $seen[$cursor] = $true
            $cursor = [int]$byPid[$cursor].parentPid
        }
    }
    return ,$rows
}

function Match-Identity($actual, $expected) {
    if ($null -eq $actual -or !$actual.CreationDate -or !$actual.ExecutablePath) { return $false }
    $created = ([DateTimeOffset]$actual.CreationDate.ToUniversalTime()).ToUnixTimeMilliseconds()
    return ([int]$actual.ProcessId -eq [int]$expected.pid -and
        [int]$actual.ParentProcessId -eq [int]$expected.parentPid -and
        $created -eq [long]$expected.creationMs -and
        [string]::Equals([string]$actual.ExecutablePath, [string]$expected.executablePath, [StringComparison]::OrdinalIgnoreCase))
}

function Read-Flag([string]$command, [string]$name) {
    $pattern = '(?:^|\s)--' + [regex]::Escape($name) + '(?:=|\s+)(?:"(?<quoted>[^"]*)"|(?<bare>[^\s]+))'
    $matches = [regex]::Matches($command, $pattern)
    Assert-True ($matches.Count -eq 1) 'Required browser flag is missing or duplicated'
    if ($matches[0].Groups['quoted'].Success) { return $matches[0].Groups['quoted'].Value }
    return $matches[0].Groups['bare'].Value
}

function Parse-BrowserFlags([string]$command, [string]$expectedData) {
    try {
        $portText = Read-Flag $command 'remote-debugging-port'
        Assert-True ($portText -match '^[1-9][0-9]{0,4}$') 'Browser debugging port is invalid'
        $port = [int]$portText
        Assert-True ($port -ge 1024 -and $port -le 65535) 'Browser debugging port is invalid'
        $address = Read-Flag $command 'remote-debugging-address'
        Assert-True ($address -ceq '127.0.0.1') 'Browser debugging address is not controlled loopback'
        $data = Read-Flag $command 'user-data-dir'
        Assert-True ((Normalize-LocalPath $data) -ceq (Normalize-LocalPath $expectedData)) 'Browser UDF does not match the verified owned directory'
        return @{port=$port; address=$address; userDataDirectoryMatched=$true; observedDirectory=$data; reason=$null}
    } catch {
        # 不输出命令行或不匹配的目录。
        return @{port=$null; address=$null; userDataDirectoryMatched=$false; observedDirectory=$null; reason='Browser debugging flags are missing, ambiguous, or outside the verified owned contract'}
    }
}

$nativeSource = @'
using System;
using System.Collections.Generic;
using System.Runtime.InteropServices;
using System.Text;

public sealed class SoloSoulNativePerfProcessHandle : IDisposable {
    private IntPtr handle;
    private const uint QueryLimitedInformation = 0x1000;
    private const uint TokenQuery = 0x0008;
    private const int InsufficientBuffer = 122;
    private const int NoPackage = 15700;
    [StructLayout(LayoutKind.Sequential)]
    private struct NativeFileTime { public uint low; public uint high; }
    [DllImport("kernel32.dll", SetLastError=true)]
    private static extern IntPtr OpenProcess(uint access, bool inherit, int processId);
    [DllImport("kernel32.dll", SetLastError=true)]
    private static extern bool CloseHandle(IntPtr value);
    [DllImport("kernel32.dll", SetLastError=true)]
    private static extern bool GetProcessTimes(IntPtr process, out NativeFileTime creation, out NativeFileTime exit, out NativeFileTime kernel, out NativeFileTime user);
    [DllImport("advapi32.dll", SetLastError=true)]
    private static extern bool OpenProcessToken(IntPtr process, uint access, out IntPtr token);
    [DllImport("advapi32.dll", SetLastError=true)]
    private static extern bool GetTokenInformation(IntPtr token, int kind, IntPtr buffer, uint bufferLength, out uint returnedLength);
    [DllImport("advapi32.dll")]
    private static extern bool IsValidSid(IntPtr sid);
    [DllImport("advapi32.dll")]
    private static extern IntPtr GetSidSubAuthorityCount(IntPtr sid);
    [DllImport("advapi32.dll")]
    private static extern IntPtr GetSidSubAuthority(IntPtr sid, uint index);
    [DllImport("kernel32.dll", CharSet=CharSet.Unicode)]
    private static extern int GetPackageFamilyName(IntPtr process, ref uint length, StringBuilder name);

    public SoloSoulNativePerfProcessHandle(int processId, long expectedCreationMs) {
        handle = OpenProcess(QueryLimitedInformation, false, processId);
        if (handle == IntPtr.Zero) throw new InvalidOperationException("OpenProcess failed");
        try {
            NativeFileTime creation, exit, kernel, user;
            if (!GetProcessTimes(handle, out creation, out exit, out kernel, out user))
                throw new InvalidOperationException("GetProcessTimes failed");
            long fileTime = checked((long)(((ulong)creation.high << 32) | creation.low));
            long actual = new DateTimeOffset(DateTime.FromFileTimeUtc(fileTime)).ToUnixTimeMilliseconds();
            if (actual != expectedCreationMs) throw new InvalidOperationException("Process creation time changed");
        } catch { Dispose(); throw; }
    }

    private static IntPtr ReadTokenBuffer(IntPtr token, int kind) {
        uint length;
        GetTokenInformation(token, kind, IntPtr.Zero, 0, out length);
        if (Marshal.GetLastWin32Error() != InsufficientBuffer || length == 0 || length > 65536)
            throw new InvalidOperationException("Token size query failed");
        IntPtr buffer = Marshal.AllocHGlobal((int)length);
        if (!GetTokenInformation(token, kind, buffer, length, out length)) {
            Marshal.FreeHGlobal(buffer);
            throw new InvalidOperationException("Token query failed");
        }
        return buffer;
    }

    private static string IntegrityLabel(uint rid) {
        switch (rid) {
            case 0x0000: return "Untrusted";
            case 0x1000: return "Low";
            case 0x2000: return "Medium";
            case 0x2100: return "MediumPlus";
            case 0x3000: return "High";
            case 0x4000: return "System";
            case 0x5000: return "Protected";
            default: return "Unknown";
        }
    }

    public Dictionary<string, object> ReadSecurity() {
        if (handle == IntPtr.Zero) throw new ObjectDisposedException("Owned process handle");
        var tokenResult = new Dictionary<string, object> {
            {"integrityRid", null}, {"integrityLabel", null}, {"isAppContainer", null}, {"reason", null}
        };
        IntPtr token = IntPtr.Zero;
        if (!OpenProcessToken(handle, TokenQuery, out token)) {
            tokenResult["reason"] = "Token query unavailable";
        } else {
            IntPtr buffer = IntPtr.Zero;
            try {
                buffer = ReadTokenBuffer(token, 25); // TokenIntegrityLevel
                IntPtr sid = Marshal.ReadIntPtr(buffer);
                if (!IsValidSid(sid)) throw new InvalidOperationException("Invalid integrity SID");
                byte count = Marshal.ReadByte(GetSidSubAuthorityCount(sid));
                if (count == 0) throw new InvalidOperationException("Empty integrity SID");
                uint rid = unchecked((uint)Marshal.ReadInt32(GetSidSubAuthority(sid, (uint)(count - 1))));
                tokenResult["integrityRid"] = rid;
                tokenResult["integrityLabel"] = IntegrityLabel(rid);
                Marshal.FreeHGlobal(buffer);
                buffer = IntPtr.Zero;
                buffer = ReadTokenBuffer(token, 29); // TokenIsAppContainer
                tokenResult["isAppContainer"] = Marshal.ReadInt32(buffer) != 0;
            } catch { tokenResult["reason"] = "Token information unavailable"; }
            finally {
                if (buffer != IntPtr.Zero) Marshal.FreeHGlobal(buffer);
                CloseHandle(token);
            }
        }
        var packageResult = new Dictionary<string, object> {{"value", null}, {"status", "unknown"}, {"reason", null}};
        try {
            uint length = 0;
            int result = GetPackageFamilyName(handle, ref length, null);
            if (result == NoPackage) {
                packageResult["status"] = "unpackaged";
            } else if (result == InsufficientBuffer && length > 0 && length <= 32768) {
                var name = new StringBuilder((int)length);
                result = GetPackageFamilyName(handle, ref length, name);
                if (result == 0) { packageResult["value"] = name.ToString(); packageResult["status"] = "packaged"; }
                else { packageResult["reason"] = "Package identity query failed"; }
            } else { packageResult["reason"] = "Package identity query unavailable"; }
        } catch { packageResult["reason"] = "Package identity API unavailable"; }
        return new Dictionary<string, object> {{"token", tokenResult}, {"packageFamilyName", packageResult}};
    }

    public void Dispose() {
        if (handle != IntPtr.Zero) { CloseHandle(handle); handle = IntPtr.Zero; }
        GC.SuppressFinalize(this);
    }
    ~SoloSoulNativePerfProcessHandle() { Dispose(); }
}
'@

try {
    Assert-True ($args.Count -eq 0) 'Only fixed environment inputs are accepted'
    Assert-True ($diagnosticMode -cin @('live', 'parse-only', 'self-test')) 'Explicit diagnostics mode is required'
    $ownedJson = [Environment]::GetEnvironmentVariable('SOLOSOUL_NATIVE_PERF_DIAGNOSTICS_OWNED')
    $expectedBrowserData = [Environment]::GetEnvironmentVariable('SOLOSOUL_NATIVE_PERF_DIAGNOSTICS_BROWSER_DATA_DIRECTORY')
    if ($diagnosticMode -ceq 'self-test') {
        $ownedJson = '[{"pid":101,"parentPid":1,"creationMs":1790740537053,"executablePath":"C:\\rf312\\solo_soul.exe"},{"pid":102,"parentPid":101,"creationMs":1790740539478,"executablePath":"C:\\rf312\\msedgewebview2.exe"}]'
        $expectedBrowserData = 'C:\rf312\webview\EBWebView'
    }
    $owned = Parse-Owned $ownedJson
    Assert-True ((Is-LocalAbsolutePath $expectedBrowserData) -and [IO.Path]::GetFileName($expectedBrowserData.TrimEnd('\')) -ceq 'EBWebView') 'Verified owned browser data directory is required'
    $diagnosticStage = 'C# compilation'
    Add-Type -TypeDefinition $nativeSource -Language CSharp -ErrorAction Stop | Out-Null
    if ($diagnosticMode -ceq 'parse-only') {
        Write-Result @{schemaVersion=1; scope=$diagnosticScope; mode=$diagnosticMode; success=$true; reason=$null; compiled=$true; powerShellVersion=$PSVersionTable.PSVersion.ToString(); clrVersion=[Environment]::Version.ToString(); inputCount=$owned.Count; processes=@(); listeners=@(); liveQueriesPerformed=$false} 0
    }
    if ($diagnosticMode -ceq 'self-test') {
        $diagnosticStage = 'offline self-test'
        $passed = 0
        $good = '"C:\rf312\msedgewebview2.exe" --remote-debugging-port=50376 --remote-debugging-address=127.0.0.1 --user-data-dir="\\?\C:\rf312\webview\EBWebView"'
        Assert-True ((Parse-BrowserFlags $good $expectedBrowserData).userDataDirectoryMatched) 'Controlled browser flag fixture failed'; $passed++
        foreach ($bad in @(
            ($good + ' --remote-debugging-port=50377'),
            ($good.Replace('127.0.0.1', '0.0.0.0')),
            ($good.Replace('50376', '0')),
            ($good.Replace('rf312\webview', 'other\webview')),
            ($good.Replace('--remote-debugging-port=50376', ''))
        )) {
            Assert-True (!(Parse-BrowserFlags $bad $expectedBrowserData).userDataDirectoryMatched) 'Unsafe browser flag fixture was accepted'; $passed++
        }
        $badJsons = @(
            '{}', '[]',
            $ownedJson.Replace('"pid":102', '"pid":101'),
            $ownedJson.Replace('"parentPid":101', '"parentPid":102'),
            $ownedJson.Replace('"parentPid":101', '"parentPid":3'),
            $ownedJson.Replace('C:\\rf312\\solo_soul.exe', 'solo_soul.exe'),
            $ownedJson.Replace('"pid":101', '"unexpected":0,"pid":101'),
            $ownedJson.Replace('"creationMs":1790740537053', '"creationMs":"1790740537053"')
        )
        foreach ($badJson in $badJsons) {
            $rejected = $false
            try { $null = Parse-Owned $badJson } catch { $rejected = $true }
            Assert-True $rejected 'Unsafe owned identity fixture was accepted'; $passed++
        }
        $actual = [pscustomobject]@{ProcessId=101; ParentProcessId=1; CreationDate=([DateTimeOffset]::FromUnixTimeMilliseconds(1790740537053).UtcDateTime); ExecutablePath='C:\rf312\solo_soul.exe'}
        Assert-True (Match-Identity $actual $owned[0]) 'Matching synthetic identity failed'; $passed++
        $actual.ParentProcessId = 2
        Assert-True (!(Match-Identity $actual $owned[0])) 'Changed synthetic identity was accepted'; $passed++
        Write-Result @{schemaVersion=1; scope=$diagnosticScope; mode=$diagnosticMode; success=$true; reason=$null; compiled=$true; powerShellVersion=$PSVersionTable.PSVersion.ToString(); clrVersion=[Environment]::Version.ToString(); assertionsPassed=$passed; processes=@(); listeners=@(); liveQueriesPerformed=$false} 0
    }

    $diagnosticStage = 'fresh owned identity verification'
    $verifiedRows = @()
    # 全列表身份检查完成前，不读取 CommandLine、Token、Package 或版本。
    foreach ($expected in $owned) {
        $liveQueriesPerformed = $true
        $actual = Get-CimInstance Win32_Process -Filter ('ProcessId = ' + [int]$expected.pid) -Property ProcessId,ParentProcessId,CreationDate,ExecutablePath -ErrorAction Stop
        Assert-True (Match-Identity $actual $expected) 'Fresh CIM owned identity mismatch'
        $process = [Diagnostics.Process]::GetProcessById([int]$expected.pid)
        $managedProcesses += $process
        Assert-True (([DateTimeOffset]$process.StartTime.ToUniversalTime()).ToUnixTimeMilliseconds() -eq [long]$expected.creationMs) 'Process StartTime owned identity mismatch'
        # 有限查询句柄保持到输出结束，并再次核对创建时间。
        $handle = New-Object SoloSoulNativePerfProcessHandle ([int]$expected.pid), ([long]$expected.creationMs)
        $nativeHandles += $handle
        $verifiedRows += @{expected=$expected; handle=$handle}
    }
    # 任何一个根/后代不匹配，整个调用通过 catch 失败关闭。
    foreach ($entry in $verifiedRows) {
        $expected = $entry.expected
        $actual = Get-CimInstance Win32_Process -Filter ('ProcessId = ' + [int]$expected.pid) -Property ProcessId,ParentProcessId,CreationDate,ExecutablePath -ErrorAction Stop
        Assert-True (Match-Identity $actual $expected) 'Owned identity changed before diagnostics'
    }

    $diagnosticStage = 'owned process diagnostics'
    $outputRows = @()
    $ownedPidList = @($owned | ForEach-Object { [int]$_.pid })
    $rootOwned = @($owned | Where-Object { $ownedPidList -notcontains [int]$_.parentPid })[0]
    $rootOwnedPid = [int]$rootOwned.pid
    $mainBrowserIds = @()
    foreach ($entry in $verifiedRows) {
        $expected = $entry.expected
        $security = $entry.handle.ReadSecurity()
        $version = @{fileVersion=$null; productVersion=$null; reason=$null}
        try {
            $info = [Diagnostics.FileVersionInfo]::GetVersionInfo([string]$expected.executablePath)
            $version.fileVersion = $info.FileVersion
            $version.productVersion = $info.ProductVersion
        } catch { $version.reason = 'Owned executable file version unavailable' }
        $role = if ([int]$expected.pid -eq $rootOwnedPid) { 'root' } else { 'webview-child' }
        $flags = @{port=$null; address=$null; userDataDirectoryMatched=$null; observedDirectory=$null; reason=if ($role -ceq 'root') { 'Not applicable to the host root process' } else { 'Not applicable to a WebView2 subprocess' }}
        if ([IO.Path]::GetFileName([string]$expected.executablePath) -ieq 'msedgewebview2.exe') {
            $details = Get-CimInstance Win32_Process -Filter ('ProcessId = ' + [int]$expected.pid) -Property ProcessId,ParentProcessId,CreationDate,ExecutablePath,CommandLine -ErrorAction Stop
            Assert-True (Match-Identity $details $expected) 'Owned browser identity changed before flags'
            $command = [string]$details.CommandLine
            if ($command -notmatch '(?:^|\s)--type(?:=|\s|$)') {
                $role = 'browser'
                $mainBrowserIds += [int]$expected.pid
                $flags = Parse-BrowserFlags $command $expectedBrowserData
            }
            $command = $null
            $details = $null
        }
        $outputRows += @{pid=[int]$expected.pid; parentPid=[int]$expected.parentPid; creationMs=[long]$expected.creationMs; executablePath=[string]$expected.executablePath; role=$role; identityMatched=$true; token=$security['token']; packageFamilyName=$security['packageFamilyName']; version=$version; browserFlags=$flags}
    }
    $listeners = @()
    $listenerReason = $null
    $socketErrors = @()
    $diagnosticStage = 'owned root/browser listener verification'
    Assert-True ($mainBrowserIds.Count -eq 1) 'Exactly one owned main WebView2 browser is required'
    $listenerIds = [uint32[]]@($rootOwnedPid, $mainBrowserIds[0])
    foreach ($listenerId in $listenerIds) {
        $expected = @($owned | Where-Object { [uint32]$_.pid -eq $listenerId })[0]
        $fresh = Get-CimInstance Win32_Process -Filter ('ProcessId = ' + $listenerId) -Property ProcessId,ParentProcessId,CreationDate,ExecutablePath -ErrorAction Stop
        Assert-True (Match-Identity $fresh $expected) 'Owned listener process identity changed'
    }
    $sockets = @(Get-NetTCPConnection -State Listen -OwningProcess $listenerIds -ErrorAction SilentlyContinue -ErrorVariable socketErrors)
    foreach ($socketError in $socketErrors) {
        if ($socketError.FullyQualifiedErrorId -notlike 'CmdletizationQuery_NotFound*') { $listenerReason = 'Owned listener query unavailable' }
    }
    foreach ($socket in $sockets) {
        Assert-True ($listenerIds -contains [uint32]$socket.OwningProcess) 'Listener owner is outside the verified owned list'
        $listeners += @{owningPid=[int]$socket.OwningProcess; localAddress=[string]$socket.LocalAddress; localPort=[int]$socket.LocalPort}
    }
    Write-Result @{schemaVersion=1; scope=$diagnosticScope; mode=$diagnosticMode; success=$true; reason=$null; identityVerified=$true; processes=$outputRows; listeners=$listeners; listenersReason=$listenerReason; liveQueriesPerformed=$liveQueriesPerformed} 0
} catch {
    # 不传播异常正文，避免源码、完整命令行或 EncodedCommand 进入证据。
    Write-Result @{schemaVersion=1; scope=$diagnosticScope; mode=$diagnosticMode; success=$false; reason=('Diagnostics failed during ' + $diagnosticStage); processes=@(); listeners=@(); liveQueriesPerformed=$liveQueriesPerformed} 1
} finally {
    foreach ($handle in $nativeHandles) { if ($null -ne $handle) { $handle.Dispose() } }
    foreach ($process in $managedProcesses) { if ($null -ne $process) { $process.Dispose() } }
}
