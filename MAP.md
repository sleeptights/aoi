# aoi — карта проекта

Tauri 2 + Rust. UI — один `ui/index.html` (React + Babel в браузере). Старый Electron назывался seWer; `ui/index.md` — его renderer-шпаргалка, строки там уже плывут.

Идентификатор: `aoi.player`  
Данные: `%APPDATA%\aoi.player\`  
Плеер после установщика: `%LOCALAPPDATA%\aoi\aoi.exe`

---

## как запускать (важно)

| режим | команда | когда |
|--------|---------|--------|
| **dev (день-день)** | `.\dev.ps1` | правки UI / Rust |
| **ship (друзьям)** | `.\ship.ps1` | один раз перед раздачей |

**`.\dev.ps1`** поднимает лёгкий HTTP-сервер на `ui/` + `cargo tauri dev`.  
Правки в `ui/` → **F5** (или Ctrl+R) в окне плеера. Rust **не** пересобирается.  
Пересборка Rust только если трогал `src-tauri/`.

**`.\ship.ps1`** = `cargo tauri build --no-bundle` и копия в `dist\aoi.exe` (UI вшивается в exe).  
`%LOCALAPPDATA%\aoi\aoi.exe` и ярлык установщика **сами не обновляются** — это всегда старый билд, пока не прогонишь ship + установщик/копию.

Не открывай для UI-работы вчерашний `dist\aoi.exe` / установленный aoi — там замороженный фронт.

---

## дерево

```
aoi/
  dev.ps1 / ship.ps1     быстрый цикл / релиз
  scripts/ui-dev-server.ps1
  ui/index.html          весь фронт
  ui/tauri-api.js        window.electronAPI → invoke Tauri
  ui/index.md            старая карта seWer (не доверять номерам строк)
  src-tauri/src/lib.rs   старт окна, трей, медиаклавиши, crash log
  src-tauri/src/soundcloud/
    mod.rs               sc_login / sc_fetch + кеш обложек/лайков
    browser.rs           окно логина WebView, хук OAuth, Firefox cookies
    fetch.rs             reqwest + jar cookies (sc_cookies.json)
    write.rs             скрытый sc-bridge webview, PUT/DELETE через fetch в странице
  src-tauri/src/discord.rs
  src-tauri/src/music.rs           локальная папка
  src-tauri/src/media_protocol.rs  media:// файлы
  src-tauri/src/settings.rs
  loader/                aoi-setup.exe, пакует dist/aoi.exe
  dist/aoi.exe
  dist/installers/aoi-setup-win-x64.exe
```

---

## данные на диске

| файл | зачем |
|------|--------|
| `settings.json` | всё, включая `soundcloudAuth` |
| `sc_cookies.json` | `_soundcloud_session`, `datadome`, `oauth_token` |
| `sc_likes.json` | кеш лайков |
| `sc_covers/` | jpg обложек |
| `sc_write.log` | PUT/DELETE SoundCloud |
| `aoi.log` | старт / ошибки трея / media shortcuts |

Легаси импорт настроек из `%APPDATA%\aoi` и `%APPDATA%\sewer`.

---

## UI — `ui/index.html` (актуальные строки)

| строки | что |
|--------|-----|
| 1–125 | head, Proxima Soft, vendor, css `:root`, `#app-shell`, titlebar |
| 16 | `tauri-api.js` |
| 163 | `extractAccentColor` |
| 308 | `mapScTrack` — ещё `artworkUrl` для Discord |
| 343 | `parseDiscordAppId` / `pickDiscordCover` |
| 371+ | `STRINGS` ru/en |
| 536 | MarqueeText |
| 584 | PlayerLikeBtn |
| 605 | ProgressBar |
| 786 | ThinVolumeSlider |
| 914 | AlbumArt |
| 975 | AmbientGlow |
| 1006 | MagBtn (`locked` есть, транспорт **не** лочить без SC) |
| 1027 | PlayBtn |
| 1062 | NavIcon — search лочится без SC, home/player нет |
| 1080 | TrackRow |
| 1143 | TrackContextMenu |
| 1238 | VirtualTrackList |
| 1347 | HomeCard |
| 1408 | HeroClone (GSAP) |
| 1489 | Sidebar |
| 1607 | CrossfadeSlider |
| 1705 | SettingsView |
| 2145 | AvatarFlyClone |
| 2169 | SoundCloudView — `scLogin()` |
| 2260 | SearchTrackRow |
| ~2370 | SearchView |
| 2536 | ArtistView + follow |
| 2920 | `App()` |
| 3217+ | sleep 100 мин |
| 3360+ | Discord update (не clear на пустой title) |
| 3768 | handleScLogin / logout |
| 3792 | `sourceMode`: sc только если есть auth |
| 4080 | handleLike → `users/{uid}/track_likes/{tid}` PUT/DELETE |
| 4131 | handleFollow → `me/followings/{id}` |
| 4165 | handleStartStation |
| 4284 | handleNav — search без auth запрещён |
| 4774 | кнопки плеера (play/next не locked) |

IPC: `window.electronAPI.*` в `ui/tauri-api.js` → команды в `lib.rs`.

---

## SoundCloud

**Логин** (`browser.rs`): окно `sc-login` на `soundcloud.com/signin`. Хук перехватывает `Authorization: OAuth …` и `client_id`. Куки WebView2 на Windows могут зависать — не ждать только их. Fallback: cookies Firefox `_soundcloud_session`. Успех → окно закрыть.

**Чтение API** (`fetch.rs`): OAuth + cookies + x-datadome. GET ок, write часто 403 captcha.

**Запись** (`mod.rs` → `write.rs`): при 401/403/429/0 — скрытый webview `sc-bridge` делает `fetch` с credentials. Captcha выезжает на экран. DELETE 404 = ок. `error: 0` нельзя отдавать в UI (JS считает это успехом).

**Лайк UI**: оптимистичный список, откат при `res.error`. User id только из `/me`, не хардкодить.

Друзья на Chrome ломались, когда логин читал только Firefox.

---

## Discord

`DEFAULT_APP_ID = 1539444732248203345` (приложение **aoi**).  
Тип: **Playing**. Поток `aoi-discord`, UI не блокировать.  
Не включать aoi в Registered Games Discord — это «Играет в aoi» с вопросиком, не RPC.  
Обложка: публичный https (`artworkUrl`), не localhost cache.

---

## старт / краш

`lib.rs`: нет `.expect` на иконке трея и media keys. Нет WebView2 → MessageBox + ссылка Evergreen. Лог: `aoi.log`.

Транспорт плеера должен работать без SC (локальные файлы).

---

## установщик

`loader/` — тёмное окно, gzip `dist/aoi.exe` внутрь, ставит в `%LOCALAPPDATA%\aoi`, ярлык, запуск, сам закрывается. Пересобирать **после** `.\ship.ps1` (свежий `dist/aoi.exe`).

```
cd loader && cargo build --release
```

---

## куда лезть

| задача | файлы |
|--------|--------|
| кнопки/экраны | `ui/index.html` по таблице строк |
| логин SC | `soundcloud/browser.rs` |
| лайки 403 | `write.rs` + `fetch.rs` + `sc_write.log` |
| Discord | `discord.rs` + кусок App ~3360 |
| локальные треки | `music.rs` + `media_protocol.rs` |
| не стартует | `lib.rs` + `aoi.log` |
| кинуть кентам | `dist/installers/aoi-setup-win-x64.exe` |
