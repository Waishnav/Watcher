# Stops/removes installed executables and OpenTUI, but preserves activity history.
$ErrorActionPreference = 'Stop'
$binDir = if ($env:WATCHER_BIN_DIR) { $env:WATCHER_BIN_DIR } else { Join-Path $env:LOCALAPPDATA 'Programs\Watcher' }
$tuiDir = if ($env:WATCHER_TUI_DIR) { $env:WATCHER_TUI_DIR } else { Join-Path $env:LOCALAPPDATA 'watcher\tui' }
$watcher = Join-Path $binDir 'watcher.exe'
if (Test-Path $watcher) { & $watcher shutdown }
Remove-Item -Force -ErrorAction SilentlyContinue (Join-Path $binDir 'watcher.exe'), (Join-Path $binDir 'watcherd.exe')
Remove-Item -Force -Recurse -ErrorAction SilentlyContinue $tuiDir
Write-Output 'Executables/TUI removed. Activity database retained.'
