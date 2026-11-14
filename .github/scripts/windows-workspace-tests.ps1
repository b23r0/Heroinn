function Limit-DiagnosticText([string]$Text, [int]$Maximum) {
    if ($Text.Length -le $Maximum) { return $Text }
    $marker = "`n[truncated; complete raw logs retained]"
    $length = $Maximum - $marker.Length
    if ([char]::IsHighSurrogate($Text[$length - 1])) { $length-- }
    return $Text.Substring(0, $length) + $marker
}

function Write-DiagnosticLine([string]$Text) {
    $bytes = [Text.Encoding]::UTF8.GetBytes($Text + "`n")
    $output = [Console]::OpenStandardOutput()
    $output.Write($bytes, 0, $bytes.Length)
    $output.Flush()
}

function Get-TestLogExcerpt([string]$Path) {
    $selected = [Collections.Generic.List[string]]::new()
    $tail = [Collections.Generic.Queue[string]]::new()
    $state = @{line=[Text.StringBuilder]::new(); shortened=$false; lines=0; shortened_lines=0}
    $acceptLine = {
        $text = $state.line.ToString()
        $text = [regex]::Replace($text, '\x1b\[[0-?]*[ -/]*[@-~]', '')
        $text = [regex]::Replace($text, '\x1b\][^\x07]*(?:\x07|$)', '')
        $text = [regex]::Replace($text, '[\x00-\x08\x0b-\x1f\x7f]', '')
        if ($state.shortened) {
            $text += ' [line truncated]'
            $state.shortened_lines++
        }
        if ($text -match 'FAILED|panicked at|^error:|^Caused by:|^failures:|^\s+[A-Za-z_][A-Za-z0-9_:]+$|process didn.t exit successfully|exit code|signal:|fatal runtime error') {
            if ($selected.Count -lt 24) { $selected.Add($text) }
        }
        if ($tail.Count -eq 12) { [void]$tail.Dequeue() }
        $tail.Enqueue($text)
        [void]$state.line.Clear()
        $state.shortened = $false
        $state.lines++
    }
    $reader = [IO.StreamReader]::new($Path, [Text.Encoding]::UTF8, $true, 4096)
    try {
        $buffer = [char[]]::new(4096)
        while (($count = $reader.Read($buffer, 0, $buffer.Length)) -gt 0) {
            for ($offset = 0; $offset -lt $count; $offset++) {
                $character = $buffer[$offset]
                if ($character -eq "`n") {
                    & $acceptLine
                } elseif ($state.line.Length -lt 2048) {
                    [void]$state.line.Append($character)
                } else {
                    $state.shortened = $true
                }
            }
        }
        if ($state.line.Length -or $state.shortened) { & $acceptLine }
    } finally {
        $reader.Dispose()
    }
    $details = ($selected -join "`n") + "`nLast lines:`n" + ($tail.ToArray() -join "`n")
    return Limit-DiagnosticText "lines=$($state.lines), truncated_lines=$($state.shortened_lines), failure_lines_limit=24, tail_lines_limit=12`n$details" 6000
}

function Write-ProtectedRawLogs([string[]]$Paths) {
    $token = [guid]::NewGuid().ToString('N')
    Write-DiagnosticLine "::stop-commands::$token"
    try {
        $output = [Console]::OpenStandardOutput()
        foreach ($path in $Paths) {
            Write-DiagnosticLine "Raw $([IO.Path]::GetFileName($path)):"
            if (-not [IO.File]::Exists($path)) { continue }
            $rawStream = [IO.File]::OpenRead($path)
            try {
                $rawStream.CopyTo($output, 81920)
                $output.Flush()
            } finally {
                $rawStream.Dispose()
            }
            Write-DiagnosticLine ''
        }
    } finally {
        Write-DiagnosticLine ''
        Write-DiagnosticLine "::${token}::"
    }
}

function Invoke-WorkspaceTestDiagnostics {
    param(
        [string]$FilePath = 'cargo',
        [string]$Arguments = 'test --workspace --exclude heroinn_shell_terminal --locked',
        [Parameter(Mandatory)][string]$LogDirectory,
        [string]$SummaryPath = $env:GITHUB_STEP_SUMMARY
    )
    $ErrorActionPreference = 'Stop'
    $utf8 = [Text.UTF8Encoding]::new($false)
    $cargoExit = $null
    $diagnosticError = $null
    $process = $null
    $started = $false
    $stdoutFile = $null
    $stderrFile = $null
    $stdoutPath = Join-Path $LogDirectory 'stdout.log'
    $stderrPath = Join-Path $LogDirectory 'stderr.log'
    Write-DiagnosticLine 'Running unchanged workspace test command with default concurrency.'
    try {
        [void][IO.Directory]::CreateDirectory($LogDirectory)
        $stdoutFile = [IO.File]::Open($stdoutPath, [IO.FileMode]::CreateNew, [IO.FileAccess]::Write, [IO.FileShare]::Read)
        $stderrFile = [IO.File]::Open($stderrPath, [IO.FileMode]::CreateNew, [IO.FileAccess]::Write, [IO.FileShare]::Read)
        $info = [Diagnostics.ProcessStartInfo]::new()
        $info.FileName = $FilePath
        $info.Arguments = $Arguments
        $info.WorkingDirectory = (Get-Location).ProviderPath
        $info.UseShellExecute = $false
        $info.RedirectStandardOutput = $true
        $info.RedirectStandardError = $true
        $process = [Diagnostics.Process]::new()
        $process.StartInfo = $info
        [void]$process.Start()
        $started = $true
        $stdoutCopy = $process.StandardOutput.BaseStream.CopyToAsync($stdoutFile, 81920)
        $stderrCopy = $process.StandardError.BaseStream.CopyToAsync($stderrFile, 81920)
        while (-not $process.WaitForExit(200)) {
            if ($stdoutCopy.IsFaulted -or $stderrCopy.IsFaulted) { throw 'Raw log capture failed while the test command was running' }
        }
        $cargoExit = $process.ExitCode
        [void]$stdoutCopy.GetAwaiter().GetResult()
        [void]$stderrCopy.GetAwaiter().GetResult()
    } catch {
        $diagnosticError = Limit-DiagnosticText $_.Exception.Message 1000
        if ($started -and -not $process.HasExited) {
            $process.Kill($true)
            $process.WaitForExit()
        }
    } finally {
        if ($stdoutFile) { $stdoutFile.Dispose() }
        if ($stderrFile) { $stderrFile.Dispose() }
        if ($process) { $process.Dispose() }
    }

    $excerpt = ''
    try {
        Write-ProtectedRawLogs @($stdoutPath, $stderrPath)
        foreach ($path in @($stdoutPath, $stderrPath)) {
            if ([IO.File]::Exists($path)) {
                $excerpt += "`n$([IO.Path]::GetFileName($path)):`n" + (Get-TestLogExcerpt $path)
            }
        }
    } catch {
        $diagnosticError = Limit-DiagnosticText "$diagnosticError`n$($_.Exception.Message)" 1000
    }
    $finalExit = if ($null -ne $cargoExit -and $cargoExit -ne 0) { $cargoExit } elseif ($diagnosticError -or $null -eq $cargoExit) { 1 } else { 0 }
    $exitHex = if ($null -eq $cargoExit) { $null } else { '0x{0:x8}' -f ([long]$cargoExit -band 4294967295L) }
    $exitText = if ($null -eq $cargoExit) { 'not started or no natural result' } else { "$cargoExit ($exitHex)" }
    $details = "Cargo exit code: $exitText`nFinal exit code: $finalExit`nDiagnostic error: $diagnosticError`n$excerpt"
    try {
        if ([string]::IsNullOrEmpty($SummaryPath)) { throw 'GITHUB_STEP_SUMMARY is not available' }
        $summary = "### Windows workspace tests`n`n"
        $summary += "Command: ``cargo test --workspace --exclude heroinn_shell_terminal --locked``. Default concurrency and inherited toolchain are unchanged.`n`n"
        $summary += "Complete stdout/stderr are retained separately; cross-stream ordering is not reconstructed. Public excerpts are bounded.`n`n<pre>"
        $summary += [Net.WebUtility]::HtmlEncode((Limit-DiagnosticText $details 12000))
        $summary += "</pre>`n"
        [IO.File]::AppendAllText($SummaryPath, $summary, $utf8)
    } catch {
        $diagnosticError = Limit-DiagnosticText "$diagnosticError`n$($_.Exception.Message)" 1000
        if ($null -eq $cargoExit -or $cargoExit -eq 0) { $finalExit = 1 }
    }
    try {
        $record = [ordered]@{
            command_file=$FilePath
            command_arguments=$Arguments
            cargo_exit_code=$cargoExit
            cargo_exit_hex=$exitHex
            final_exit_code=$finalExit
            diagnostic_error=$diagnosticError
            powershell_version=$PSVersionTable.PSVersion.ToString()
            stdout_bytes=if ([IO.File]::Exists($stdoutPath)) { ([IO.FileInfo]$stdoutPath).Length } else { 0 }
            stderr_bytes=if ([IO.File]::Exists($stderrPath)) { ([IO.FileInfo]$stderrPath).Length } else { 0 }
        }
        [IO.File]::WriteAllText((Join-Path $LogDirectory 'result.json'), ($record | ConvertTo-Json -Depth 4), $utf8)
    } catch {
        $diagnosticError = Limit-DiagnosticText "$diagnosticError`n$($_.Exception.Message)" 1000
        if ($null -eq $cargoExit -or $cargoExit -eq 0) { $finalExit = 1 }
    }
    $annotation = Limit-DiagnosticText "Cargo exit code: $exitText; final exit code: $finalExit; diagnostic error: $diagnosticError`n$excerpt" 4000
    $annotation = $annotation.Replace('%', '%25').Replace("`r", '%0D').Replace("`n", '%0A')
    $level = if ($finalExit -eq 0) { 'notice' } else { 'error' }
    Write-DiagnosticLine "::$level title=Windows workspace tests::$annotation"
    return [int]$finalExit
}

if ($MyInvocation.InvocationName -ne '.') {
    $directory = Join-Path $env:RUNNER_TEMP 'heroinn-workspace-tests'
    exit (Invoke-WorkspaceTestDiagnostics -LogDirectory $directory)
}
