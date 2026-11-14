param([Parameter(Mandatory)][string]$EvidenceDirectory)

$ErrorActionPreference = 'Stop'
$wrapper = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..\windows-workspace-tests.ps1'))
if (-not (Test-Path -LiteralPath $wrapper)) { throw 'Diagnostic implementation is missing' }
New-Item -ItemType Directory -Path $EvidenceDirectory | Out-Null
$utf8 = [Text.UTF8Encoding]::new($false)
$nativeShell = (Get-Process -Id $PID).Path
$fixture = Join-Path $EvidenceDirectory 'fixture.ps1'
$driver = Join-Path $EvidenceDirectory 'driver.ps1'
[IO.File]::WriteAllText($fixture, @'
param([string]$Scenario, [int]$ExitCode, [string]$ExpectedRoot)
$utf8 = [Text.UTF8Encoding]::new($false)
$stdout = ''
$stderr = ''
switch ($Scenario) {
    'success' { $stdout = "running 1 test`r`ntest fixture::ok ... ok`r`n" }
    'empty' { }
    'failure' {
        $stdout = "running 1 test`r`ntest fixture::failure ... FAILED`r`nfailures:`r`n    fixture::failure`r`n"
        $stderr = "thread 'fixture::failure' panicked at fixture.rs:7: assertion failed`r`nerror: test failed`r`n"
    }
    'special' {
        $stdout = "$([char]27)[31mtest fixture::unicode_中文_😀 ... FAILED$([char]27)[0m`r`n"
        $stderr = "thread 'fixture::unicode_中文_😀' panicked at <tag>& `u{0060}`u{0060}`u{0060}`r`n100% %0A %0D , : ::error title=forged::not-a-command`r`n::set-env name=FORGED::secret`r`n</pre><script>alert(1)</script>`r`n"
    }
    'large' {
        $stdout = "test fixture::early_failure ... FAILED`r`n" + ('x' * 50000) + "`r`n" + ("ordinary line`r`n" * 10000)
        $stderr = "error: test failed`r`n"
    }
}
$stdoutBytes = $utf8.GetBytes($stdout)
$stderrBytes = $utf8.GetBytes($stderr)
[void][IO.Directory]::CreateDirectory($ExpectedRoot)
[IO.File]::WriteAllBytes((Join-Path $ExpectedRoot 'stdout.log'), $stdoutBytes)
[IO.File]::WriteAllBytes((Join-Path $ExpectedRoot 'stderr.log'), $stderrBytes)
$outStream = [Console]::OpenStandardOutput()
$errStream = [Console]::OpenStandardError()
$outStream.Write($stdoutBytes, 0, $stdoutBytes.Length)
$errStream.Write($stderrBytes, 0, $stderrBytes.Length)
$outStream.Flush()
$errStream.Flush()
[Environment]::Exit($ExitCode)
'@, $utf8)
[IO.File]::WriteAllText($driver, @'
param([string]$Wrapper, [string]$Shell, [string]$Fixture, [string]$Scenario, [int]$Code, [string]$Logs, [string]$Summary, [string]$Executable)
. $Wrapper
$file = if ($Executable) { $Executable } else { $Shell }
$expected = Join-Path ([IO.Path]::GetDirectoryName($Logs)) 'expected'
$arguments = "-NoProfile -File `"$Fixture`" -Scenario $Scenario -ExitCode $Code -ExpectedRoot `"$expected`""
$status = Invoke-WorkspaceTestDiagnostics -FilePath $file -Arguments $arguments -LogDirectory $Logs -SummaryPath $Summary
if ($status -isnot [int]) { throw 'Diagnostic return pipeline must contain exactly one integer' }
exit $status
'@, $utf8)

function Assert-Contract([bool]$Condition, [string]$Message) {
    if (-not $Condition) { throw $Message }
}

$results = [Collections.Generic.List[object]]::new()
function Run-Case([string]$Name, [string]$Scenario, [int]$Code, [int]$Expected, [switch]$BadSummary, [switch]$MissingExecutable) {
    $caseRoot = Join-Path $EvidenceDirectory $Name
    New-Item -ItemType Directory $caseRoot | Out-Null
    $logs = Join-Path $caseRoot 'logs'
    $summary = Join-Path $caseRoot 'summary.md'
    if ($BadSummary) { New-Item -ItemType Directory $summary | Out-Null }
    $file = if ($MissingExecutable) { Join-Path $caseRoot 'missing.exe' } else { '' }
    $info = [Diagnostics.ProcessStartInfo]::new()
    $info.FileName = $nativeShell
    $info.Arguments = "-NoProfile -File `"$driver`" -Wrapper `"$wrapper`" -Shell `"$nativeShell`" -Fixture `"$fixture`" -Scenario $Scenario -Code $Code -Logs `"$logs`" -Summary `"$summary`""
    if ($file) { $info.Arguments += " -Executable `"$file`"" }
    $info.UseShellExecute = $false
    $info.RedirectStandardOutput = $true
    $info.RedirectStandardError = $true
    $info.StandardOutputEncoding = $utf8
    $info.StandardErrorEncoding = $utf8
    $process = [Diagnostics.Process]::new()
    $process.StartInfo = $info
    [void]$process.Start()
    $stdoutTask = $process.StandardOutput.ReadToEndAsync()
    $stderrTask = $process.StandardError.ReadToEndAsync()
    $process.WaitForExit()
    $public = $stdoutTask.GetAwaiter().GetResult()
    $errorText = $stderrTask.GetAwaiter().GetResult()
    [IO.File]::WriteAllText((Join-Path $caseRoot 'wrapper.stdout.log'), $public, $utf8)
    [IO.File]::WriteAllText((Join-Path $caseRoot 'wrapper.stderr.log'), $errorText, $utf8)
    Assert-Contract ($process.ExitCode -eq $Expected) "$Name exit: $($process.ExitCode), expected $Expected"
    $record = Get-Content -Raw -LiteralPath (Join-Path $logs 'result.json') | ConvertFrom-Json
    Assert-Contract ($record.final_exit_code -eq $Expected) "$Name receipt exit mismatch"
    if (-not $MissingExecutable) {
        Assert-Contract ($record.cargo_exit_code -eq $Code) "$Name original exit was lost"
        Assert-Contract ($record.command_arguments -like "*-Scenario $Scenario -ExitCode $Code *") "$Name command mutated"
        foreach ($stream in @('stdout.log', 'stderr.log')) {
            $actual = (Get-FileHash -LiteralPath (Join-Path $logs $stream)).Hash
            $expectedHash = (Get-FileHash -LiteralPath (Join-Path (Join-Path $caseRoot 'expected') $stream)).Hash
            Assert-Contract ($actual -eq $expectedHash) "$Name $stream raw bytes changed"
        }
    }
    $annotations = @($public -split '\r?\n' | Where-Object { $_ -match '^::(error|notice) title=Windows workspace tests::' })
    Assert-Contract ($annotations.Count -eq 1) "$Name expected exactly one diagnostic annotation"
    Assert-Contract ($annotations[0].Length -le 17000) "$Name annotation is unbounded"
    if ($Expected -eq 0) {
        Assert-Contract ($annotations[0].StartsWith('::notice ')) "$Name success emitted an error"
    } else {
        Assert-Contract ($annotations[0].StartsWith('::error ')) "$Name failure was hidden"
    }
    Assert-Contract ($public -match '::stop-commands::[a-f0-9]{32}') "$Name raw output was not protected"
    $suspended = $null
    foreach ($line in $public -split '\r?\n') {
        if ($suspended) {
            if ($line -eq "::${suspended}::") { $suspended = $null }
        } elseif ($line -match '^::stop-commands::([a-f0-9]{32})$') {
            $suspended = $Matches[1]
        } else {
            Assert-Contract ($line -notmatch '^::(?:set-env|error title=forged)') "$Name forged workflow command escaped protection"
        }
    }
    Assert-Contract ($null -eq $suspended) "$Name workflow commands were not resumed"
    if (-not $BadSummary) {
        $markdown = [IO.File]::ReadAllText($summary)
        Assert-Contract ([Text.Encoding]::UTF8.GetByteCount($markdown) -le 100000) "$Name summary is unbounded"
        Assert-Contract ($markdown.Contains("Final exit code: $Expected")) "$Name summary exit missing"
    }
    if ($Scenario -eq 'failure' -or $Scenario -eq 'special') {
        Assert-Contract ($annotations[0].Contains('fixture::')) "$Name failed test identity missing"
        Assert-Contract ($annotations[0].Contains('panicked at')) "$Name panic missing"
    }
    if ($Scenario -eq 'special') {
        $raw = [IO.File]::ReadAllText((Join-Path $logs 'stdout.log'))
        Assert-Contract ($raw.Contains([string][char]27)) 'Raw ANSI output was modified'
        Assert-Contract ($annotations[0] -notmatch '\x1b') 'ANSI leaked into annotation'
        Assert-Contract ($annotations[0].Contains('%250A')) 'Literal percent escape was interpreted'
        Assert-Contract ($annotations[0].Contains('%0A')) 'Newline was not escaped'
        Assert-Contract ($annotations[0].Contains('中文_😀')) 'Unicode was lost'
        Assert-Contract ($markdown.Contains('&lt;script&gt;')) 'HTML was not escaped'
        Assert-Contract (-not $markdown.Contains('</pre><script>')) 'Markdown escaped its container'
    }
    if ($Scenario -eq 'large') {
        Assert-Contract ((Get-Item (Join-Path $logs 'stdout.log')).Length -gt 180000) 'Raw log was truncated'
        Assert-Contract ($markdown.Contains('truncated')) 'Summary truncation was not disclosed'
        Assert-Contract ($annotations[0].Contains('fixture::early_failure')) 'Early failure was lost behind tail output'
    }
    if ($BadSummary) {
        Assert-Contract ($record.diagnostic_error -ne $null) "$Name diagnostic failure invisible"
    }
    if ($Code -eq -1073741819 -and -not $MissingExecutable) {
        Assert-Contract ($record.cargo_exit_hex -eq '0xc0000005') 'Native exception code was not retained'
        Assert-Contract ($annotations[0].Contains('0xc0000005')) 'Native exception code missing from annotation'
    }
    $results.Add(@{name=$Name; cargo_exit=$record.cargo_exit_code; final_exit=$process.ExitCode; annotation_chars=$annotations[0].Length; passed=$true})
    $process.Dispose()
}

Run-Case 'success' 'success' 0 0
Run-Case 'failure101' 'failure' 101 101
Run-Case 'empty-success' 'empty' 0 0
Run-Case 'empty-failure' 'empty' 7 7
Run-Case 'escaped-output' 'special' 101 101
Run-Case 'bounded-output' 'large' 101 101
Run-Case 'summary-error-on-success' 'success' 0 1 -BadSummary
Run-Case 'summary-error-on-failure' 'failure' 101 101 -BadSummary
Run-Case 'start-error' 'empty' 0 1 -MissingExecutable
Run-Case 'native-exception-code' 'empty' -1073741819 -1073741819
$results | ConvertTo-Json -Depth 5 | Set-Content -Encoding UTF8 (Join-Path $EvidenceDirectory 'results.json')
'WINDOWS_WORKSPACE_DIAGNOSTICS_PASS cases=10'
