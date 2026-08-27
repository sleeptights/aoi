
# карта renderer/index.html

весь ui — один файл ~4500 строк. jsx компилируется babel standalone в браузере.

## структура файла

| строки    | что                                                        |
|-----------|------------------------------------------------------------|
| 1–95      | `<head>`: локальные vendor-скрипты (react production, babel, framer-motion, gsap), `hls.min.js`, css (`:root` vars, `#app-shell`, titlebar, scrollbar) |
| 96–125    | `#app-shell` + `#titlebar` html |
| 126–end   | `<script type="text/babel">`: весь react |

## css-переменные

```css
--bg: #07070a   --border: rgba(255,255,255,0.055)
--text: #ededf4   --accent: #b2b2b2   --accent-rgb: 178, 178, 178
--titlebar-h: 28px
```

`--accent` / `--accent-rgb` **динамически переписываются** через `useEffect` в `App` при смене `accentMode` или extracted цвета обложки. все места с `var(--accent)` (Sidebar pill, NavIcon active, MagBtn active, TrackRow active, лайк, caret) меняют цвет автоматически.

**анимации:** `breathe`, `spin`, `fadeInUp`, `fadeOutDown`, `artEntrance` (легаси — частично заменены Motion-компонентами)

**скроллбары:** `.scroll-thin` (8px зона, 2px визуально), `.scroll-home` (отступы под хедер сетки)

шрифт: **Proxima Soft** (`renderer/fonts/ProximaSoft-Bold.ttf`)

## vendor

```
vendor/
  react.production.min.js
  react-dom.production.min.js
  babel.min.js
  framer-motion.min.js     — UMD build (11.18.2), глобал window.Motion
  gsap.min.js              — для HeroClone timeline и GSAP track slide
```

на верху babel-скрипта:
```js
const { useState, useEffect, useLayoutEffect, useRef, useCallback, useMemo } = React;
const { motion, AnimatePresence, LayoutGroup } = Motion;
```

## assets

```
assets/
  icon.png / icon.svg       — иконка приложения
  play.png / pause.png      — кнопки плеера
  forward.png / rewind.png  — prev/next
  shuffle.png / repeat.png / repeat1.png — управление
  tracks.png / search.png / player.png   — sidebar nav
  local.png / settings.png               — sidebar bottom
  note.png                  — заглушка обложки (TrackRow, AlbumArt, HomeCard, HeroClone)
  vosproizvedenie.png / system.png / about.png — секции настроек + иконка прослушиваний в SearchTrackRow
  heart0.png / heart1.png   — лайки
```

## утилиты

- `smoothScroll(el, targetTop, duration)` — плавный скролл easeInOutQuad
- `fmt(s)` → `M:SS`
- `fmtCount(n)` → `12.4K` / `1.2M` / `null` если 0
- `splitArtists(str)` → `[{name, sep}]` — парсит мультиартистную строку
- `SoundCloudIcon({ size, fill })` — инлайн SVG логотип SC
- `scHashColor(id)` → `hsl(...)` по id трека
- `mapScTrack(t, noTitle)` → SC track object (см. ниже)
- `extractAccentColor(url)` → Promise<`{r,g,b}` | null>. 3-пасса с relaxing thresholds (sat/lum/count) + mean fallback + dark-boost. кеширует по URL в `_accentCache`. crossOrigin=anonymous.
- `ACCENT_PRESETS` — `{ default, lavender, mint, rose, amber }` → `{r,g,b}`

## i18n

```
STRINGS        — объект { ru: {...}, en: {...} }, ~100 ключей
LangContext    — React.createContext('ru')
useLang()      — хук: возвращает t(key), читает LangContext
```

язык берётся из `settings.language` ('RU' | 'EN', default 'RU'), нормализуется в lowercase.
`App` вычисляет `lang` и `t` на каждом рендере, оборачивает JSX в `<LangContext.Provider value={lang}>`.
компоненты вызывают `const t = useLang()` внутри себя.
**исключение**: `SearchView` использует `const T = useLang()` — буква `t` занята переменной трека в `.map(t => ...)`.

ключи добавленные в сессиях: `back`, `subscribe`, `subscribed`, `artist_label`, `follow_err`, `unfollow_err`, `copy_link`, `link_copied`, `copy_link_err`, `start_station`, `station_for`, `station_exit`, `station_err`, `accent_title`, `accent_sub`, `accent_default/lavender/mint/rose/amber/cover`, `accent_cover_sub`.

## компоненты

### `MarqueeText`
`{text, style, onClick, maxWidth?}`. ellipsis + tooltip. `useLayoutEffect` проверяет overflow. tooltip → portal-like absolute div с `fadeInUp 0.15s`. центрирование через CSS `translate: -50% 0` (не transform — конфликтовал с keyframes).

### `PlayerLikeBtn`
`{liked, onLike, style?}`. сердечко 19px в плеере (для SC треков). scale 1.18 при hover.

### `ProgressBar` (бывш. WaveProgressBar)
`{progressRef, total, onSeek, isPlaying, visible, accentRGB}`.
div-based pill bar. высота **10px → 14px** при hover/drag (spring overshoot). external padding 10px = hit-zone ~30px. rAF обновляет `fillRef.style.transform = scaleX(p)` + thumb `left = p*100%` + текстовые таймкоды.
- **fill gradient** использует `accentRGB` (lighter справа) с fallback на бело-серый
- **glow layer** (отдельный div, не клиппится track-overflow) — accent box-shadow усиливается на hover/drag
- **thumb** — белый круг 14→16px, opacity 0 при idle, появляется при hover/drag, центрирован через `translate: -50% -50%`
- **time tooltip** — при hover/drag над курсором показывается dark pill с `fmt(time)`

### `ThinVolumeSlider`
canvas TW=28px, вертикальный. drag вверх = больше. `#c8c8c8` fill с glow, без thumb.

### `AlbumArt`
`{track, isPlaying}`. контейнер всегда с `#111116` фоном + `note.png` placeholder сзади (opacity 0.13).
- если `track.coverUrl` есть — `<img>` поверх. на `onError` ставит `opacity:0` (не display:none) → placeholder виден.
- **retry до 2 раз** с задержкой 0.5с/1с при ошибке (cache-bust query string)
- **focus/visibilitychange listener**: когда окно вернулось из фона и `<img>` не загружен (`naturalWidth===0`), форсит reload с cache-bust. защита от Chromium image-cache eviction в фоне.

### `MagBtn`
`{onClick, active, children, size=52}`. **motion.button** с `whileTap={{scale:0.82}}`, spring (600/22/0.4).

### `PlayBtn`
`{isPlaying, onToggle}`. **motion.button** с `whileTap={{scale:0.88}}`. crossfade play↔pause через два `<img>` со scale/rotate.

### `NavIcon`
`{icon, label, active, onClick, size=38, noActiveBg=false}`. активность через `opacity` (1/0.3).

### `TrackRow`
`{track, isActive, isLoading, isError, onClick, onContextMenu}`.
высота 50px. миниатюра 34px. спиннер при `isLoading`. `isError` → «недоступен» красным.
**onContextMenu** → `e.preventDefault()` + вызов проп `onContextMenu(track, x, y)`.

### `VirtualTrackList`
`{items, activeId, loadingId, errorId, onClickItem, onContextMenuItem, scrollToActive, scrollTargetId}`.
ROW_H=50. `content-visibility:auto`. абсолютный пилл с transition. `onContextMenuItem` пробрасывается в `TrackRow`.
- **edge fade-out**: `mask-image: linear-gradient(...)` динамически из state `edges={top,bottom}`. top fade видим если `scrollTop > 4`, bottom — если есть скрытый контент снизу. transition `mask-image 0.2s ease`. если содержимое влезает целиком — mask='none'.

### `HomeCard`
`{track, onSelect, artRef, artHidden, isLiked, onLike}`. **motion.div** с entry/exit spring (380/32/0.6): `scale 0.92→1`, `opacity 0→1`, exit `scale 0.88`. **layout prop убран** — был perf-bottleneck на больших гридах.

### `HeroClone` ⚡
`{hero, exiting, reverse=false}`. portal → document.body. **GSAP timeline** `expo.inOut`, `force3D:true`.
- **одиночный div** (без inner): static `border-radius:16` + `overflow:hidden` для cover-clip. clipPath-анимация убрана — была CPU-bound через rasterize.
- animation **только transform** (translate3d + scale) — GPU-only композит
- `useLayoutEffect` + pre-paint inline `transform: translate3d(dx,dy,0) scale(sc)` → no flash при mount
- exit: `gsap.to(opacity:0)` с `power2.out` 140мс
- `contain:'layout style paint'` + `backface-visibility:hidden` — изоляция от соседнего DOM, sub-pixel AA
- `<img loading="eager" decoding="sync">` — обложка готова к старту анимации

### `Sidebar`
`{navActive, onNav, libCollapsed, onToggleLib, inPlayer, scAuth, profileRef, sourceMode, onToggleSource, avatarFlying}`.
левая панель 58px без border (разделители убраны для чистоты).
- SC аватар 44×44 вверху → `onNav('soundcloud')`
- NAV `[home, search, library]` с анимирующимся пиллом
- collapse-кнопка (только в player)
- toggle source (только при scAuth)
- settings.png → настройки

### `CrossfadeSlider`
`{value [0–12], onChange}`. drag через `setPointerCapture`. wheel (passive:false). 6px трек без thumb.

### `SettingsView`
`{settings, onSettings, visible, onScanTracks, onClearFolder, onClearCoversCache, onClearLikesCache, appAccent}`.
секции: `playback`→`vosproizvedenie.png`, `appearance`→`theme.png`, `system`→`system.png`, `about`→`about.png`.

**playback**: только CrossfadeSlider.

**appearance** (порядок):
1. **карточка «Акцентный цвет»** (новое):
   - 5 swatches (default/lavender/mint/rose/amber) — круги 32px с Apple-style focus ring при active
   - divider
   - выделенная карточка **«От обложки»** с conic-gradient rainbow border (CSS mask `xor` trick), палитра-иконка в круглом gradient, live preview swatch (`appAccent` real-time)
   - выбор пишет в `settings.accentMode`
2. карточка «Интерфейс» — `hideDividers` toggle (влияет на разделители в SettingsView **и** SearchTrackRow)
3. карточка «Discord» — toggle `discordRpc` + анимированный блок (timestamp chips, pause chips, cover toggle)

**system**: язык, кэш, запуск, локальная музыка.

### `AvatarFlyClone`
portal-анимация аватара (SoundCloudView → Sidebar), 440мс.

### `SearchTrackRow`
`{track, isLiked, onLike, onClick, onCoverClick, isLoading, isError, hideDividers}`.
**motion.div** с `layout` + initial/animate/exit spring (380/34/0.6). `height:'auto'`, exit сжимается в 0.
- обложка 44px с play-оверлеем (hover) или спиннером
- title + artist (flex:1)
- stats: плей-иконка `vosproizvedenie.png` + count, длительность, `tabular-nums`
- **лайк в pill-обёртке справа** (отдельный блок): фон/бордер при active, min-width фиксирован (62px/32px) чтобы не дёргался
- `hideDividers` — `borderBottom` исчезает
- `isError` — opacity 0.55

### `SearchView`
`{visible, scAuth, likedIds, onLike, onPlayTrack, onSelectTrack, onResultsLoaded, onArtistClick, loadingTrackId, errorTrackId, hideDividers}`.
рефы `hasMoreRef`, `offsetRef`. `useImperativeHandle({focus, loadMore})`. карточки артистов кликабельны.
- **inputs**: иконка `position:absolute left`, input `width:100% padding`, `text-align:center`. placeholder = `t('search_ph')` = "поиск"/"search".
- **artist cards**: 42px avatar, 13.5px name, gap 14, padding 12/16, ellipsis на длинных именах. при hover тонкий бордер.
- результаты завёрнуты в `<AnimatePresence initial={false}>`.

### `ArtistView`
`{artist, visible, onClose, scAuth, likedIds, onLike, onPlayTrack, onSelectTrack, loadingTrackId, errorTrackId, artistCacheRef, onFollow, onCheckFollow, hideDividers}`.

**artist объект**: `{ id?, username, avatarUrl?, followersCount?, bannerUrl? }`

при открытии: если `artist.id` есть — `/users/{id}`. иначе `search/users?q=username&limit=1` → `/users/{id}`. кеш в `artistCacheRef` (Map по id или username).

**hero (asymmetric)**:
- blur-фон (banner > avatar > пусто) + тёмный градиент (apple-style)
- круглая кнопка «назад» 32px с `backdrop-filter:blur(10px)` + `rgba(0,0,0,0.42)` — видна на любом фоне
- слева аватар 148px (`borderRadius:16`), справа info-колонка:
  - caps "АРТИСТ" 10.5px
  - имя `clamp(26px, 3.4vw, 38px)` bold
  - followers + **кнопка «Подписаться»/«Вы подписаны»** в строку

**подписка**:
- эффект на `[visible, profileId]` → `onCheckFollow(profileId)` (GET /me/followings/{id})
- клик кнопки → `onFollow(profileId, isFollowing)` (PUT/DELETE через scFetch)
- состояние `isFollowing`/`followBusy` локально, кеш `followedIdsRef` глобально

**табы (LayoutGroup + layoutId)**:
- 2 вкладки: Популярные / Треки
- активная имеет `<motion.div layoutId="artistTabPill">` (spring 420/34/0.7) — Motion сам анимирует пилл между табами, нет ручных измерений

**загрузка треков** (useEffect на `[tab, profileId, visible, fetchKey]`):
- Popular → `/users/{id}/toptracks?limit=20`
- Tracks → `/users/{id}/tracks?limit=20` (+ пагинация через `next_href`)

треки в `<AnimatePresence initial={false}>` через `SearchTrackRow`.

### `TrackContextMenu`
`{menu, onClose, onCopyLink, onStartStation}`. portal → document.body.
- **motion.div** с pop-in (`scale 0.92→1`, opacity, transform-origin top-left), spring (520/36/0.5). exit `scale 0.95`.
- позиция у курсора через `useLayoutEffect` (clamp за края экрана с PAD=8)
- закрывается: клик вне, Escape, scroll wheel, `blur` окна
- pill-стиль: `rgba(24,24,24,0.97)` + backdrop-blur, padding 4px, borderRadius 10
- пункты:
  - **Скопировать ссылку** (disabled если нет `permalinkUrl`)
  - **Запустить станцию** (disabled если нет id)

---

### `App` — state

```
view              — 'home'|'player'|'search'|'soundcloud'|'settings'|'artist'
tracks, trackIdx, isPlaying, progress, shuffle, repeat
navActive, search, volume, hero, playerVisible, homeVisible
heroExiting, reverseHero, reverseHeroExiting
avatarFly, avatarFlyExiting, avatarFlying
libCollapsed, settings, sort, editingTitle, editValue
scTracks, scLoading, scUpdating, scError
scPlayingTrack, scPlayingIdx
libScrollTrigger, libScrollTargetId, searchFocused
toast, toastExiting
loadingTrackId, errorTrackId
artEntranceKey
artistView      — null | { id?, username, avatarUrl?, followersCount?, bannerUrl? }
trackMenu       — null | { track, x, y }
stationActive   — null | { origTrack, tracks }
accentRGB       — null | { r, g, b } extracted from track.coverUrl
```

`lang` и `t` — не state, вычисляются при каждом рендере из `settings.language`.

**appAccent** — `useMemo([settings.accentMode, accentRGB])`:
- если `accentMode === 'cover'` и `accentRGB` есть → boost luminance to ≥110, returns `{r,g,b}`
- иначе → `ACCENT_PRESETS[settings.accentMode] || ACCENT_PRESETS.default`

**useEffect [appAccent]** → `document.documentElement.style.setProperty('--accent', 'rgb(r,g,b)')` + `--accent-rgb`

**useEffect [track.id, track.coverUrl]** → `extractAccentColor(coverUrl)` → `setAccentRGB(c)`

**settings**:
```js
{
  autoplay, crossfade: 0,
  startWithWindows, minimizeToTray,
  musicFolder, customTitles, minDuration: 30,
  soundcloudAuth, sourceMode: 'local',
  language: 'RU',
  hideDividers,
  discordRpc, discordTimestamp, discordPause, discordCover,
  accentMode: 'default',  // 'default'|'lavender'|'mint'|'rose'|'amber'|'cover'
}
```

### `App` — refs

```
artRefs, artRefCacheRef   — refs на обложки HomeCard
playerArtRef              — ref на AlbumArt в плеере
slideWrapRef              — обёртка обложки в плеере (GSAP slide)
playerInfoRef             — блок артист+название (GSAP анимация входа)
trackDirRef               — 'next'|'prev' направление GSAP slide
slideReadyRef             — пропуск первого mount
audioRef, hlsRef
handleNextRef, handlePrevRef, isPlayingRef
homeScrollRef, scAvatarRef, sidebarProfileRef
scTracksRef, scAuthRef, scCacheRef, scCacheMapRef
artistCacheRef            — Map<id|username, profile> для ArtistView кеша
followedIdsRef            — Set<userId> кеш статуса подписок
filteredRef, filteredScRef, searchRef
volumeRef, settingsRef, langRef
crossfadeRafRef, trackSwitchingRef
toastTimerRef
discordTimerRef, discordProgressRef
prevSearchRef, homeSearchRef, libSearchRef
searchQueueRef            — queue из SearchView/ArtistView (для next/prev)
stationQueueRef           — queue станции (приоритет над searchQueue в next/prev)
scShuffleOrderRef, scShuffleIdxRef
prevViewRef               — view из которого открылся ArtistView
customTitlesRef, minDurationRef, editCancelRef
```

### `App` — helpers

- **`startFadeIn(audio)`** — fade-in для crossfade
- **`destroyHls()`** — уничтожает hls.js
- **`showToast(msg)`** — 2.2с показ, 0.24с fade-out
- **`handleSearchResultsLoaded(newTracks)`** — append к searchQueueRef + дошафливание
- **`handleOpenArtist(artist)`** / **`handleCloseArtist()`** — навигация ArtistView
- **`handleFollow(userId, currentlyFollowing)`** — POST/DELETE `/me/followings/{id}`, обновляет `followedIdsRef`
- **`checkFollow(userId)`** — GET `/me/followings/{id}`, кешируется в `followedIdsRef`
- **`handleCopyTrackLink(track)`** — GET `/share/short-link?url=...` через scFetch → `navigator.clipboard.writeText`; fallback на `track.permalinkUrl` если short-link API упал
- **`handleStartStation(track)`** — GET `/stations/soundcloud:track-stations:{id}/tracks?limit=50`, mapScTrack каждый, ставит `stationQueueRef` и `stationActive`, играет `tracks[0]`
- **`handleExitStation()`** — чистит `stationQueueRef` и `stationActive`

### `App` — mouse side-buttons

useEffect на `[view, tracks, trackIdx, handleCloseArtist]` слушает `window.mouseup`:
- **button=3** (XButton1 back): artist → handleCloseArtist; player/settings/soundcloud/search → home
- **button=4** (XButton2 forward): если есть текущий трек и `view ∉ {player, artist}` → плеер

### `App` — crossfade

- **fade-out**: в `timeupdate` — если `remaining ≤ crossfade`, `audio.volume = volume * (remaining/crossfade)`
- **fade-in SC**: `trackSwitchingRef=true`, `audio.volume=0`, `startFadeIn` в обработчике `playing` event
- **fade-in local**: `startFadeIn(audio)` перед `audio.play()`
- **пауза**: отменяет rAF; восстанавливает volume только если `!trackSwitchingRef.current`

### `App` — handleScTrackClick

1. `trackSwitchingRef=true`, `setIsPlaying(false)`, `setLoadingTrackId(id)`
2. fetch `streamUrl` → fallback `hlsUrl`
3. если оба недоступны → markError, skipInDirection если autoSkip
4. `setScPlayingTrack(resolved)`
5. `audio.volume = cf>0 ? 0 : volumeRef.current`
6. `'playing'` event → `trackSwitchingRef=false`, `startFadeIn`
7. HLS → hls.js manifest → play; иначе progressive → `audio.src` → play
8. `setLoadingTrackId(null)` в `.then()`

list priority в next/prev и handleScTrackClick: `stationQueueRef > searchQueueRef > scTracks/filteredSc`

### `App` — handleLike

`PUT /users/{userId}/track_likes/{id}` / `DELETE`. через `sc-fetch` (main process с DataDome cookie). при ошибке — тост.

### `App` — initScLikes (кеш-миграция)

Если у `cache[0]` нет `hlsUrl` ИЛИ `permalinkUrl` как own property → старый формат → `loadScLikes(auth)` (полная перезагрузка). Защищает от устаревших полей при апдейтах.

### `App` — selectTrack (home grid → player)

async функция:
1. если `view==='player'` → просто `startPlay()`, return
2. **scroll target card в видимую область** (mirror logic из handleNav home→player) + `await rAF`
3. если `artEl` и `playerArtRef.current` есть → setHero с правильными rect'ами + переход в player
4. иначе → skip hero, обычный переход (защита от (0,0) glitch при пустых рефах)

### `App` — SC трек-объект

```js
{ id, title, artist, duration, color, coverUrl,
  artistId, artistAvatarUrl, uploaderUsername,
  likesCount, playCount,
  streamUrl, hlsUrl,
  permalinkUrl,      // для context menu copy-link и short-link API
  resolvedUrl,       // только у scPlayingTrack
}
```

### `App` — имя артиста в плеере

`splitArtists(track.artist)` разбивает строку. каждый name — отдельный кликабельный спан.
- `name === uploaderUsername` → `handleOpenArtist({id: artistId, username, avatarUrl})`
- иначе → `handleOpenArtist({username})` (ArtistView найдёт через поиск)

### `App` — Discord RPC

useEffect зависит от `[track?.id, isPlaying, settings.discordRpc, discordTimestamp, discordPause, discordCover]`. `timestamp` в main: `'progress'` → start+end; `'elapsed'` → только start; `'none'` → без.

### `App` — лейаут плеера

центральный блок:
- `width: calc(100% - 48px)`, `maxWidth: clamp(380px, 55vw, 900px)`
- порядок: playerInfoRef (артист → название) → slideWrapRef (обложка + **ambient glow blob**) → ProgressBar → controls
- **ambient glow** (за обложкой, `zIndex:0`, opacity 1 кроме hero):
  - `position:absolute inset:-30%, borderRadius:50%`
  - `background: radial-gradient` с `appAccent` (4 stops: 0.55 → 0.26 → 0.08 → 0)
  - `filter: blur(42px)`
  - `transition: opacity 0.6s ease, background 0.6s ease`
- **обложка**: `width: min(100%, clamp(320px, 54vw, 760px), clamp(240px, 58vh, 760px))`
- **прогресс-бар**: `width: min(100%, clamp(260px, 34vw, 520px))`
- **библиотека**: фиксированный 260px, схлопывается через `width:0` (overflow:hidden), кнопка collapse в сайдбаре

### `App` — анимации

- **Motion (framer-motion)** — на компонент-level:
  - `motion.div` с layout/initial/animate/exit/spring: HomeCard, SearchTrackRow, ProgressBar thumb (косвенно), MagBtn, PlayBtn, TrackContextMenu, toast
  - `AnimatePresence` обёртки: home grid, SearchView results, ArtistView tracks, toast, context menu
  - `LayoutGroup` + `layoutId="artistTabPill"` — sliding pill в ArtistView tabs
- **GSAP** (timeline-сложные):
  - **home→player**: HeroClone `expo.inOut` 0.38s + playerInfoRef fade+slide (`sine.out` 0.42s delay 0.08s)
  - **player→home**: reverse HeroClone `expo.inOut`
  - **смена трека**: GSAP `fromTo` на `slideWrapRef` (`y: ±14 → 0`, `power2.out` 0.5s); направление из `trackDirRef`
- **SC логин**: AvatarFlyClone 440мс
- **artEntrance**: при `view→'player'` без hero → `artEntranceKey++` → scale/opacity, 420мс
- **toast**: Motion entrance/exit с spring (380/30/0.6)

### рендер
```js
ReactDOM.createRoot(document.getElementById('root')).render(<App/>)
``