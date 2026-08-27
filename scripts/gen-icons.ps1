# Rasterize the titlebar 14-grid mark (2px bars, 2px gaps) into PNG + classic DIB ICO.
$ErrorActionPreference = 'Stop'

$cs = @'
using System;
using System.Drawing;
using System.Drawing.Drawing2D;
using System.Drawing.Imaging;
using System.IO;

public static class AoiIconGen {
  static void AddRoundRect(GraphicsPath p, float x, float y, float w, float h, float r) {
    if (r < 0.5f) { p.AddRectangle(new RectangleF(x, y, w, h)); return; }
    r = Math.Min(r, Math.Min(w, h) / 2f);
    float d = r * 2f;
    p.AddArc(x, y, d, d, 180, 90);
    p.AddArc(x + w - d, y, d, d, 270, 90);
    p.AddArc(x + w - d, y + h - d, d, d, 0, 90);
    p.AddArc(x, y + h - d, d, d, 90, 90);
    p.CloseFigure();
  }

  public static Bitmap Mark(int n) {
    int scale = Math.Max(1, n / 14);
    int inner = 14 * scale;
    int off = (n - inner) / 2;
    var bmp = new Bitmap(n, n, PixelFormat.Format32bppArgb);
    using (var g = Graphics.FromImage(bmp)) {
      g.Clear(Color.FromArgb(255, 7, 7, 10));
      if (n >= 64) {
        g.SmoothingMode = SmoothingMode.AntiAlias;
        g.PixelOffsetMode = PixelOffsetMode.HighQuality;
      } else {
        g.SmoothingMode = SmoothingMode.None;
        g.PixelOffsetMode = PixelOffsetMode.None;
      }
      using (var fg = new SolidBrush(Color.FromArgb(255, 237, 237, 244))) {
        int[] xs = { 2, 6, 10 };
        int[] ys = { 6, 2, 4 };
        int[] hs = { 6, 10, 8 };
        float bw = 2 * scale;
        float rad = bw / 2f;
        for (int i = 0; i < 3; i++) {
          using (var path = new GraphicsPath()) {
            AddRoundRect(path, off + xs[i] * scale, off + ys[i] * scale, bw, hs[i] * scale, rad);
            g.FillPath(fg, path);
          }
        }
      }
    }
    return bmp;
  }

  static byte[] Dib(Bitmap bmp) {
    int w = bmp.Width, h = bmp.Height;
    int xorSize = w * 4 * h;
    int andStride = ((w + 31) / 32) * 4;
    int andSize = andStride * h;
    using (var ms = new MemoryStream())
    using (var bw = new BinaryWriter(ms)) {
      bw.Write(40);
      bw.Write(w);
      bw.Write(h * 2);
      bw.Write((ushort)1);
      bw.Write((ushort)32);
      bw.Write(0);
      bw.Write(xorSize + andSize);
      bw.Write(0); bw.Write(0);
      bw.Write(0); bw.Write(0);
      for (int y = h - 1; y >= 0; y--) {
        for (int x = 0; x < w; x++) {
          Color c = bmp.GetPixel(x, y);
          bw.Write(c.B); bw.Write(c.G); bw.Write(c.R); bw.Write(c.A);
        }
      }
      bw.Write(new byte[andSize]);
      bw.Flush();
      return ms.ToArray();
    }
  }

  public static void WriteIco(string path, Bitmap[] bmps) {
    var blobs = new byte[bmps.Length][];
    for (int i = 0; i < bmps.Length; i++) blobs[i] = Dib(bmps[i]);
    using (var ms = new MemoryStream())
    using (var bw = new BinaryWriter(ms)) {
      bw.Write((ushort)0);
      bw.Write((ushort)1);
      bw.Write((ushort)bmps.Length);
      int offset = 6 + 16 * bmps.Length;
      for (int i = 0; i < bmps.Length; i++) {
        int w = bmps[i].Width, h = bmps[i].Height;
        bw.Write((byte)(w >= 256 ? 0 : w));
        bw.Write((byte)(h >= 256 ? 0 : h));
        bw.Write((byte)0);
        bw.Write((byte)0);
        bw.Write((ushort)1);
        bw.Write((ushort)32);
        bw.Write(blobs[i].Length);
        bw.Write(offset);
        offset += blobs[i].Length;
      }
      for (int i = 0; i < blobs.Length; i++) bw.Write(blobs[i]);
      bw.Flush();
      File.WriteAllBytes(path, ms.ToArray());
    }
  }

  public static void SavePng(Bitmap bmp, string path) {
    bmp.Save(path, ImageFormat.Png);
  }
}
'@

Add-Type -AssemblyName System.Drawing
try { Add-Type -TypeDefinition $cs -ReferencedAssemblies System.Drawing } catch {
  if ($_.Exception.Message -notmatch 'already exists') { throw }
}

$root = Split-Path $PSScriptRoot -Parent
$icons = Join-Path $root 'src-tauri\icons'

$bmp16 = [AoiIconGen]::Mark(16)
$bmp32 = [AoiIconGen]::Mark(32)
$bmp48 = [AoiIconGen]::Mark(48)
$bmp128 = [AoiIconGen]::Mark(128)
$bmp256 = [AoiIconGen]::Mark(256)
$bmp512 = [AoiIconGen]::Mark(512)

[AoiIconGen]::SavePng($bmp32, (Join-Path $icons '32x32.png'))
[AoiIconGen]::SavePng($bmp128, (Join-Path $icons '128x128.png'))
[AoiIconGen]::SavePng($bmp256, (Join-Path $icons '128x128@2x.png'))
[AoiIconGen]::SavePng($bmp512, (Join-Path $icons 'icon.png'))
[AoiIconGen]::SavePng($bmp512, (Join-Path $root 'ui\assets\icon.png'))
[AoiIconGen]::WriteIco((Join-Path $icons 'icon.ico'), @($bmp16, $bmp32, $bmp48, $bmp256))

$bmp16.Dispose(); $bmp32.Dispose(); $bmp48.Dispose()
$bmp128.Dispose(); $bmp256.Dispose(); $bmp512.Dispose()
Write-Output 'ok'
