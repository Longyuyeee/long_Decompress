param([Parameter(Mandatory=$true)][string]$TestBinary)
$ErrorActionPreference = 'Stop'
if ([Threading.Thread]::CurrentThread.ApartmentState -ne 'STA') { throw 'Run this bounded clipboard check with powershell.exe -STA.' }
Add-Type -AssemblyName System.Windows.Forms
Add-Type -AssemblyName System.Drawing
$taskRoot = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '../test-results/explorer-clipboard-1740'))
[IO.Directory]::CreateDirectory($taskRoot) | Out-Null
# Materialize the user's current formats before the short test changes the clipboard.
$previous = [Windows.Forms.Clipboard]::GetDataObject()
$saved = New-Object Windows.Forms.DataObject
if ($null -ne $previous) {
    foreach ($format in $previous.GetFormats($false)) {
        $value = $previous.GetData($format, $false)
        if ($value -is [IO.MemoryStream]) { $value = New-Object IO.MemoryStream -ArgumentList @(,$value.ToArray()) }
        elseif ($value -is [Drawing.Image]) { $value = $value.Clone() }
        if ($null -ne $value) { $saved.SetData($format, $false, $value) }
    }
}
$oldRoot = $env:LONG_COPY_TEST_ROOT
try {
    $env:LONG_COPY_TEST_ROOT = $taskRoot
    & $TestBinary clipboard1740_windows_file_drop_roundtrip_and_paste_after_cache_drop --ignored --nocapture --test-threads=1
    if ($LASTEXITCODE -ne 0) { throw 'Native file clipboard check failed.' }
    $receipt = Get-Content -LiteralPath (Join-Path $taskRoot 'clipboard-receipt.json') -Raw -Encoding UTF8 | ConvertFrom-Json
    # The producing process has exited. Read its actual CF_HDROP using another Windows consumer.
    $files = [Windows.Forms.Clipboard]::GetFileDropList()
    if ($files.Count -ne 2) { throw 'Windows did not receive both selected roots.' }
    $effect = [Windows.Forms.Clipboard]::GetDataObject().GetData('Preferred DropEffect')
    if ($effect -isnot [IO.MemoryStream] -or [BitConverter]::ToInt32($effect.ToArray(), 0) -ne 1) { throw 'Clipboard must request copy, never move.' }
    $destination = Join-Path $taskRoot ('paste-' + [Guid]::NewGuid().ToString('N'))
    [IO.Directory]::CreateDirectory($destination) | Out-Null
    foreach ($file in $files) {
        if ($receipt.paths -notcontains $file) { throw 'Clipboard contained an unexpected path.' }
        Copy-Item -LiteralPath $file -Destination $destination -Recurse
    }
    $text = [IO.File]::ReadAllText((Join-Path $destination '资料/a.txt'))
    if ($text -ne $receipt.text -or -not (Test-Path -LiteralPath (Join-Path $destination '空目录') -PathType Container)) { throw 'Pasted tree is incomplete.' }
    if ([IO.File]::ReadAllText((Join-Path $destination '资料/子目录/b.txt')) -ne 'second') { throw 'Nested file changed.' }
    if (Test-Path -LiteralPath (Join-Path $destination 'outside.txt')) { throw 'Unselected content was copied.' }
    [ordered]@{producerExited=$true; consumer='Windows Forms CF_HDROP + filesystem copy'; selectedRoots=$files.Count; preferredDropEffect='copy'; nestedContent=$true; emptyDirectory=$true; unselectedExcluded=$true} | ConvertTo-Json | Set-Content -LiteralPath (Join-Path $taskRoot 'clipboard-consumer.json') -Encoding UTF8
    Write-Output 'Independent Windows clipboard consumer and paste readback passed after producer exit.'
} finally {
    $env:LONG_COPY_TEST_ROOT = $oldRoot
    if ($null -ne $previous) { [Windows.Forms.Clipboard]::SetDataObject($saved, $true, 10, 100) }
    else { [Windows.Forms.Clipboard]::Clear() }
}
