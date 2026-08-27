<div align="center">

<br/>

# aoi

<br/>

*music player for windows.*<br/>
*soundcloud · local · rooms · presence*

<br/>

![](https://img.shields.io/badge/windows-only-0d0d12?style=flat-square&logoColor=white)
![](https://img.shields.io/badge/tauri-2-0d0d12?style=flat-square&logo=tauri&logoColor=white)
![](https://img.shields.io/badge/react-18-0d0d12?style=flat-square&logo=react&logoColor=white)
![](https://img.shields.io/badge/rust-backend-0d0d12?style=flat-square&logo=rust&logoColor=white)

<br/>

</div>

---

<br/>

aoi is a dark, quiet music player built on **Tauri 2** — not another bloated Electron shell. Stream SoundCloud likes, play local files, hang out in rooms, and see who's online. The UI stays out of the way; the audio doesn't.

<br/>

## features

- **soundcloud** — sign in, sync likes, stream tracks and stations
- **local library** — scan a folder; tags and covers come along
- **rooms** — listen together with shared playback and invites
- **presence** — see friends online, what they're hearing, invite them in
- **discord rpc** — show the track in your Discord status
- **cover accents** — interface tint follows album art
- **eq + mini player** — six-band EQ and a floating always-on-top mini
- **auto-update** — in-app update when a new build ships

<br/>

## build

```powershell
.\ship.ps1
```

Outputs `dist\aoi.exe` and `dist\installers\aoi-setup-win-x64.exe`. Day-to-day UI work: `.\dev.ps1` (F5).

Version lives in three places and must match — `ship.ps1` checks them every run:

- `src-tauri/tauri.conf.json`
- `src-tauri/Cargo.toml`
- `loader/Cargo.toml`

<br/>

## structure

```
aoi/
  ui/            frontend (React in index.html)
  src-tauri/     Tauri + Rust
  loader/        Windows installer
  rooms-worker/  Cloudflare presence / update API
```

Data: `%APPDATA%\aoi.player\`

<br/>

<div align="center">
<sub>aoi</sub>
</div>
