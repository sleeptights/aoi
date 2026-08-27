# Daily UI/Rust work — no release embed.
# First launch compiles Rust once; after that ui/* edits = F5 / Ctrl+R (no rebuild).
# Full cargo rebuild only when you change src-tauri/.
$ErrorActionPreference = 'Stop'
Set-Location $PSScriptRoot\src-tauri
Write-Host 'aoi dev — edit ui\, then F5 in the window. Ctrl+C to stop.' -ForegroundColor Cyan
cargo tauri dev
