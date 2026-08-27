# Ship build: embeds ui/ into exe, copies to dist\aoi.exe
# Use only when giving to friends / installer — not for day-to-day UI edits.
$ErrorActionPreference = 'Stop'
$root = $PSScriptRoot

function Get-TomlVersion([string]$path) {
  $m = Select-String -Path $path -Pattern '^\s*version\s*=\s*"([^"]+)"' | Select-Object -First 1
  if (-not $m) { throw "version not found in $path" }
  return $m.Matches[0].Groups[1].Value
}

$confPath = Join-Path $root 'src-tauri\tauri.conf.json'
$conf = Get-Content -Raw $confPath | ConvertFrom-Json
$vConf = [string]$conf.version
$vApp = Get-TomlVersion (Join-Path $root 'src-tauri\Cargo.toml')
$vLoader = Get-TomlVersion (Join-Path $root 'loader\Cargo.toml')
if ($vConf -ne $vApp -or $vConf -ne $vLoader) {
  Write-Error "Version mismatch: tauri.conf=$vConf src-tauri/Cargo=$vApp loader/Cargo=$vLoader"
  exit 1
}
Write-Host "Version check OK → $vConf" -ForegroundColor Green

Set-Location $root\src-tauri

Write-Host 'Building release (embeds ui/)...' -ForegroundColor Cyan
cargo tauri build --no-bundle
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

$candidates = [System.Collections.Generic.List[string]]::new()
if ($env:CARGO_TARGET_DIR) {
  $candidates.Add((Join-Path $env:CARGO_TARGET_DIR 'release\aoi.exe'))
  $candidates.Add((Join-Path $env:CARGO_TARGET_DIR 'x86_64-pc-windows-msvc\release\aoi.exe'))
}
$candidates.Add("$root\src-tauri\target\release\aoi.exe")
$candidates.Add("$root\src-tauri\target\x86_64-pc-windows-msvc\release\aoi.exe")
Get-ChildItem -Path "$env:LOCALAPPDATA\Temp\cursor-sandbox-cache" -Recurse -Filter 'aoi.exe' -ErrorAction SilentlyContinue |
  Where-Object { $_.FullName -match '\\release\\aoi\.exe$' } |
  Sort-Object LastWriteTime -Descending |
  Select-Object -First 3 |
  ForEach-Object { $candidates.Add($_.FullName) }

$existing = $candidates | Where-Object { $_ -and (Test-Path $_) } | ForEach-Object { Get-Item $_ }
$built = $existing | Sort-Object LastWriteTime -Descending | Select-Object -First 1 -ExpandProperty FullName
if (-not $built) {
  Write-Error 'aoi.exe not found under target/release'
  exit 1
}

New-Item -ItemType Directory -Force -Path "$root\dist" | Out-Null
Get-Process -Name aoi -ErrorAction SilentlyContinue | Stop-Process -Force -ErrorAction SilentlyContinue
Start-Sleep -Milliseconds 400
Copy-Item -Force $built "$root\dist\aoi.exe"
Write-Host "OK → dist\aoi.exe from $built ($((Get-Item "$root\dist\aoi.exe").Length) bytes)" -ForegroundColor Green

$installed = Join-Path $env:LOCALAPPDATA 'aoi\aoi.exe'
if (Test-Path (Split-Path $installed -Parent)) {
  New-Item -ItemType Directory -Force -Path (Split-Path $installed -Parent) | Out-Null
  Copy-Item -Force $built $installed
  Write-Host "OK → $installed" -ForegroundColor Green
} else {
  Write-Host 'Installed copy (%LOCALAPPDATA%\aoi) missing — skipped.' -ForegroundColor Yellow
}

Write-Host 'Building installer (embeds dist\aoi.exe)...' -ForegroundColor Cyan
Set-Location $root\loader
cargo build --release
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

$setupCandidates = [System.Collections.Generic.List[string]]::new()
if ($env:CARGO_TARGET_DIR) {
  $setupCandidates.Add((Join-Path $env:CARGO_TARGET_DIR 'release\aoi-setup.exe'))
  $setupCandidates.Add((Join-Path $env:CARGO_TARGET_DIR 'x86_64-pc-windows-msvc\release\aoi-setup.exe'))
}
$setupCandidates.Add("$root\loader\target\release\aoi-setup.exe")
$setupCandidates.Add("$root\loader\target\x86_64-pc-windows-msvc\release\aoi-setup.exe")
$setupExisting = $setupCandidates | Where-Object { $_ -and (Test-Path $_) } | ForEach-Object { Get-Item $_ }
$setup = $setupExisting | Sort-Object LastWriteTime -Descending | Select-Object -First 1 -ExpandProperty FullName
if (-not $setup) {
  Write-Error 'aoi-setup.exe not found after loader build'
  exit 1
}
New-Item -ItemType Directory -Force -Path "$root\dist\installers" | Out-Null
Copy-Item -Force $setup "$root\dist\installers\aoi-setup-win-x64.exe"
Write-Host "OK → dist\installers\aoi-setup-win-x64.exe from $setup ($((Get-Item "$root\dist\installers\aoi-setup-win-x64.exe").Length) bytes)" -ForegroundColor Green
