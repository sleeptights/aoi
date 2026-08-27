# Static UI server for `cargo tauri dev` (Cache-Control: no-store).
$ErrorActionPreference = 'Stop'
$port = 1420
$root = (Resolve-Path (Join-Path $PSScriptRoot '..\ui')).Path

$listener = [System.Net.HttpListener]::new()
$prefix = "http://127.0.0.1:$port/"
$listener.Prefixes.Add($prefix)
try {
  $listener.Start()
} catch {
  Write-Error "Port $port busy. Close the old aoi/dev process and retry. $_"
  exit 1
}
Write-Host "aoi ui → $prefix  ($root)"

$mime = @{
  '.html' = 'text/html; charset=utf-8'
  '.js'   = 'application/javascript; charset=utf-8'
  '.css'  = 'text/css; charset=utf-8'
  '.json' = 'application/json'
  '.png'  = 'image/png'
  '.jpg'  = 'image/jpeg'
  '.jpeg' = 'image/jpeg'
  '.gif'  = 'image/gif'
  '.webp' = 'image/webp'
  '.svg'  = 'image/svg+xml'
  '.ico'  = 'image/x-icon'
  '.ttf'  = 'font/ttf'
  '.woff' = 'font/woff'
  '.woff2'= 'font/woff2'
  '.map'  = 'application/json'
  '.md'   = 'text/plain; charset=utf-8'
}

try {
  while ($listener.IsListening) {
    $ctx = $listener.GetContext()
    $req = $ctx.Request
    $res = $ctx.Response
    try {
      $rel = [Uri]::UnescapeDataString($req.Url.AbsolutePath.TrimStart('/'))
      if ([string]::IsNullOrWhiteSpace($rel)) { $rel = 'index.html' }
      $rel = $rel -replace '/', '\'
      if ($rel.Contains('..')) { throw 'bad path' }
      $path = Join-Path $root $rel
      if (-not (Test-Path -LiteralPath $path -PathType Leaf)) {
        $res.StatusCode = 404
        $buf = [Text.Encoding]::UTF8.GetBytes('not found')
        $res.ContentLength64 = $buf.Length
        $res.OutputStream.Write($buf, 0, $buf.Length)
      } else {
        $ext = [IO.Path]::GetExtension($path).ToLowerInvariant()
        $bytes = [IO.File]::ReadAllBytes($path)
        $res.StatusCode = 200
        $res.ContentType = $(if ($mime.ContainsKey($ext)) { $mime[$ext] } else { 'application/octet-stream' })
        $res.Headers['Cache-Control'] = 'no-store'
        $res.ContentLength64 = $bytes.Length
        $res.OutputStream.Write($bytes, 0, $bytes.Length)
      }
    } catch {
      try { $res.StatusCode = 500 } catch {}
    } finally {
      try { $res.OutputStream.Close() } catch {}
    }
  }
} finally {
  try { $listener.Stop() } catch {}
  try { $listener.Close() } catch {}
}
