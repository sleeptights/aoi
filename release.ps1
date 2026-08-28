# Release: ship + publish update manifest to Cloudflare Worker (+ optional GitHub Release)
$ErrorActionPreference = 'Stop'
$root = $PSScriptRoot
Set-Location $root

$conf = Get-Content "$root\src-tauri\tauri.conf.json" -Raw | ConvertFrom-Json
$version = $conf.version
if (-not $version) { throw 'version missing in tauri.conf.json' }

Write-Host "Releasing aoi v$version" -ForegroundColor Cyan
& "$root\ship.ps1"
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

$setup = "$root\dist\installers\aoi-setup-win-x64.exe"
if (-not (Test-Path $setup)) { throw "missing $setup" }

$sha = (Get-FileHash -Algorithm SHA256 -Path $setup).Hash.ToLowerInvariant()
Write-Host "sha256: $sha" -ForegroundColor Green

$notes = "aoi $version"
$changelog = @(
  'nav dock top: flat bar, win controls in nav, titlebar slides on side dock',
  'appearance: nav transparency/dim for top dock, no glass panels',
  'proxy moved to System → Network with mirror auto-fallback',
  'minimal player animations: iris, vinyl rim, moon ring, view fog, corner leak',
  'presence: push update notifications to online clients on release',
  'bugfixes: top nav clicks, settings layout, proxy retry on SC/HLS'
)

$gh = "$env:LOCALAPPDATA\aoi-tools\gh\gh.exe"
$releaseUrl = ''
if (Test-Path $gh) {
  $status = & $gh auth status 2>&1 | Out-String
  if ($status -match 'Logged in') {
    Write-Host 'Creating GitHub release...' -ForegroundColor Cyan
    $tag = "v$version"
    $body = ($changelog | ForEach-Object { "- $_" }) -join "`n"
    try {
      & $gh release create $tag $setup --title "aoi $version" --notes $body 2>&1
      $repo = (& $gh repo view --json nameWithOwner -q .nameWithOwner 2>$null)
      if ($repo) {
        $releaseUrl = "https://github.com/$repo/releases/download/$tag/aoi-setup-win-x64.exe"
      }
    } catch {
      Write-Host "GitHub release skipped: $_" -ForegroundColor Yellow
    }
  } else {
    Write-Host 'GitHub CLI not logged in — run: gh auth login' -ForegroundColor Yellow
  }
}

if (-not $releaseUrl) {
  Write-Host 'No GitHub asset URL yet — set manifest url after uploading the installer.' -ForegroundColor Yellow
}

$manifest = @{
  key = 'aoi-ship-2026'
  version = $version
  url = $releaseUrl
  sha256 = $sha
  notes = $notes
  changelog = $changelog
} | ConvertTo-Json -Depth 5

$manifestPath = "$root\update\latest.json"
New-Item -ItemType Directory -Force -Path "$root\update" | Out-Null
@{
  version = $version
  url = $releaseUrl
  sha256 = $sha
  notes = $notes
  changelog = $changelog
} | ConvertTo-Json -Depth 5 | Set-Content -Encoding utf8 $manifestPath
Write-Host "Wrote $manifestPath" -ForegroundColor Green

if ($releaseUrl) {
  Write-Host 'Publishing update manifest to worker...' -ForegroundColor Cyan
  try {
    $res = Invoke-RestMethod -Method POST -Uri 'https://aoi-rooms.elvishedcc.workers.dev/update/set' `
      -ContentType 'application/json' -Body $manifest
    $res | ConvertTo-Json -Depth 5
  } catch {
    Write-Host "Worker update/set failed: $_" -ForegroundColor Yellow
  }
} else {
  Write-Host 'Skipped worker publish (no download URL). Upload installer to GitHub Releases, then POST /update/set.' -ForegroundColor Yellow
}

Write-Host "Done. Version $version" -ForegroundColor Green
