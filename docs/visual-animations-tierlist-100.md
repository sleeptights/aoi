# aoi — тирлист 100 визуальных анимаций

Каждая анимация привязана к **уникальному месту** в UI. Тиры: **S** (must-have) → **F** (не стоит).

---

## S — лучшие

| # | Анимация | Где |
|---|----------|-----|
| 1 | Breathing vinyl rim | Край рамки обложки в плеере |
| 2 | Moon-phase progress | Прогресс вокруг обложки серпом луны |
| 3 | Silence snow | Фон плеера в паузе между треками |
| 4 | Monochrome listen mode | Весь UI кроме обложки |
| 5 | Quiet chrome hide | Titlebar + боковая навигация при бездействии |
| 6 | Reduced-motion cinema cuts | Смена трека при `prefers-reduced-motion` |
| 7 | Desaturated library until play | Грид треков на Home до первого play |
| 8 | Edge glow on buffer | Край окна только при буферизации |
| 9 | Slow iris open on play | Старт воспроизведения от центра обложки |
| 10 | Liquid progress morph | Полоска прогресса под обложкой |

## A — отличные

| # | Анимация | Где |
|---|----------|-----|
| 11 | Ink bleed title | Название трека над обложкой |
| 12 | Fog between views | Переход Home ↔ Player ↔ Settings |
| 13 | Letterpress track titles | Заголовок в центре плеера |
| 14 | Dual-tone cover split | Обложка по двум доминантным цветам |
| 15 | Waveform negative space | Фон за контролами воспроизведения |
| 16 | Depth parallax album stack | Мини-превью следующих в очереди |
| 17 | Color-blind safe accents | Индикаторы статуса и акценты |
| 18 | Typography hierarchy lock | Все заголовки настроек |
| 19 | Album art window mask | Mini-player окно |
| 20 | Silent film intertitles | Между плейлистами при автопереходе |
| 21 | Black crush lift slider | Превью в настройках OLED |
| 22 | Hand-drawn focus ring | Фокус клавиатуры в поиске |
| 23 | Shadow type behind cover | Крупный текст артиста за обложкой |
| 24 | Soft vignette by energy | Виньетка краёв окна от громкости |
| 25 | Accent smoke trail | Курсор в зоне альбом-арта |
| 26 | Cover ash particles | Обложка на паузе |
| 27 | Paper grain overlay | `#app-shell` фон |
| 28 | Corner light leak | Верхний правый угол окна |
| 29 | Tape hiss visualizer | Тонкая полоска под EQ |
| 30 | Animated SVG logo mark | Экран «О приложении» |

## B — хорошие

| # | Анимация | Где |
|---|----------|-----|
| 31 | Matte vs gloss cover toggle | Переключатель в оформлении |
| 32 | Frame rate rain on pause | Оверлей `#app-shell.pause-rain` |
| 33 | Ink stamp LIVE ROOM | Бейдж в комнатах |
| 34 | Posterize cover on AFK | Обложка при статусе AFK |
| 35 | Ultra-thin hairline grid | Фон вкладки плейлистов |
| 36 | Cursor custom glyph set | Зона seek-бара |
| 37 | Slow zoom Ken Burns cover | Фон player-only scope |
| 38 | Micro film-grain on toast | Всплывающие уведомления |
| 39 | Subtitles as film captions | Статус комнаты внизу плеера |
| 40 | Halftone artist avatar | Аватар в боковой навигации |
| 41 | Dock reflection mini | Под mini-плеером |
| 42 | Stencil cut nav icons | Иконки верхней панели |
| 43 | Blink cursor on seek | Thumb прогресс-бара |
| 44 | Fold crease on playlist | Карточка плейлиста SC |
| 45 | Tape hiss edge | Правая полоска громкости |
| 46 | Shimmer skeleton cards | Загрузка лайков на Home |
| 47 | Ripple on like | Кнопка сердца в плеере |
| 48 | Marquee scroll title | Длинные названия в плеере |
| 49 | Hero fly album art | Переход Home → Player |
| 50 | Avatar fly to SC tab | Клик по аватару в доке |

## C — нишевые

| # | Анимация | Где |
|---|----------|-----|
| 51 | CRT soft scanlines | Опция в оформлении (off по умолчанию) |
| 52 | Glyph clock in titlebar | Titlebar слева |
| 53 | Typography kerning animate | Поиск SC |
| 54 | Vertical Japanese title mode | Метаданные JP-треков |
| 55 | Seasonal UI frost | Рамка окна зимой |
| 56 | Holographic foil titles | Заголовок плейлиста |
| 57 | Oblique projection cards | Сетка Discover |
| 58 | Cover mosaic home | Фон приветственного экрана |
| 59 | Particle burst skip | Кнопка Next |
| 60 | Confetti on like | Лайк в списке SC (opt-in) |
| 61 | Animated emoji reactions | Комнаты (чат реакций) |
| 62 | Rainbow spectrum bg | Экспериментальный фон (off) |
| 63 | 3D tilt covers | Карточки Home при hover |
| 64 | Neon equalizer bars | Фон вкладки поиска (off) |
| 65 | Inverted night flash | Событие смены трека ночью |
| 66 | Breathing border shell | `#app-shell` при play |
| 67 | Equalizer bar loading | Спиннер в строке трека |
| 68 | Track meta fade-in | Артист + title при смене |
| 69 | Online panel slide-in | Панель «В сети» справа |
| 70 | Notif panel slide-in | Центр уведомлений |

## D — сомнительные

| # | Анимация | Где |
|---|----------|-----|
| 71 | Glassmorphism panels | *(удалено — не использовать)* |
| 72 | Particle burst skip (heavy) | Весь экран при skip |
| 73 | Oblique isometric queue | Панель очереди слева |
| 74 | Cover mosaic settings | Фон настроек |
| 75 | Holographic nav pill | Док навигации |
| 76 | 3D carousel playlists | Вкладка плейлистов |
| 77 | Lava lamp background | Player background image |
| 78 | Matrix rain lyrics | Область lyrics (если появится) |
| 79 | Bounce elastic scroll | Список треков |
| 80 | Spinning vinyl disc | Иконка play в доке |

## F — не стоит

| # | Анимация | Где |
|---|----------|-----|
| 81 | Full-screen confetti | Любой лайк |
| 82 | Screen shake on bass | Весь UI от баса |
| 83 | Strobe accent flash | Акцент при каждом beat |
| 84 | Comic sans title morph | Название трека |
| 85 | Random rotation covers | Home grid |
| 86 | Explosion on pause | Кнопка pause |
| 87 | Rainbow cursor trail | Везде |
| 88 | Windows 95 bevel skin | Весь chrome |
| 89 | Clippy assistant | Угол настроек |
| 90 | Matrix code titlebar | Titlebar текст |
| 91 | Disco ball overlay | Player fullscreen |
| 92 | Fireworks on repeat-one | Repeat indicator |
| 93 | Jiggle all icons | Навигация |
| 94 | Zoom blur every click | Глобально |
| 95 | ASCII art progress bar | Прогресс плеера |
| 96 | Spinning 3D logo loading | Splash screen |
| 97 | Heart eyes on every track | Список треков |
| 98 | Screen invert on hover | Карточки |
| 99 | Random color shift UI | Каждые 5 сек |
| 100 | Fake BSOD on error | Ошибка SC |

---

*Сгенерировано для aoi. Места не повторяются. Glassmorphism (#71) исключён по решению продукта.*
