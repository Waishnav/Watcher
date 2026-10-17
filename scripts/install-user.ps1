# Windows user-only install; no machine-wide service and no autostart.
$ErrorActionPreference = 'Stop'
$repo = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
$binDir = if ($env:WATCHER_BIN_DIR) { $env:WATCHER_BIN_DIR } else { Join-Path $env:LOCALAPPDATA 'Programs\Watcher' }
$tuiDir = if ($env:WATCHER_TUI_DIR) { $env:WATCHER_TUI_DIR } else { Join-Path $env:LOCALAPPDATA 'watcher\tui' }
if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) { throw 'Rust/Cargo is required' }
if (-not (Get-Command bun -ErrorAction SilentlyContinue)) { throw 'Bun is required for OpenTUI' }
Push-Location $repo
try {
  cargo build --locked --release -p watcher -p watcherd
  if ($LASTEXITCODE -ne 0) { throw 'Rust build failed' }
  New-Item -Force -ItemType Directory $binDir, (Join-Path $tuiDir 'src') | Out-Null
  Copy-Item 'target\release\watcher.exe', 'target\release\watcherd.exe' -Destination $binDir
  Copy-Item 'apps\tui\package.json', 'apps\tui\bun.lock', 'apps\tui\tsconfig.json' -Destination $tuiDir
  Copy-Item 'apps\tui\src\index.ts' -Destination (Join-Path $tuiDir 'src\index.ts')
  Push-Location $tuiDir
  try {
    bun install --frozen-lockfile --production
    if ($LASTEXITCODE -ne 0) { throw 'Bun install failed' }
  } finally { Pop-Location }
  Write-Output "Installed to $binDir and $tuiDir. No autostart changes made."
} finally { Pop-Location }
