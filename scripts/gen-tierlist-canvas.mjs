import fs from 'fs';

const out =
  'C:/Users/ransomeware/.cursor/projects/c-Users-ransomeware-Projects-aoi/canvases/aoi-300-features-tierlist.canvas.tsx';

const cats = {
  visual: 'Визуал',
  tabs: 'Новые вкладки',
  extend: 'Дополнения',
  logic: 'Логика',
  ritual: 'Ритуалы и контекст',
};

// [name, tier, whatItDoes] — колонка = только что делает фича
const visual = [
  ['Breathing vinyl rim', 'S', 'Край обложки чуть пульсирует в такт BPM трека — как винил, без полосок эквалайзера.'],
  ['Ink bleed title', 'A', 'При смене трека название на долю секунды «растекается» чернилами, потом встаёт на место.'],
  ['Moon-phase progress', 'S', 'Прогресс трека рисуется серпом луны вокруг обложки вместо обычной полоски снизу.'],
  ['Fog between views', 'A', 'Переход home ↔ player ↔ settings идёт через лёгкий туман, а не слайд панели.'],
  ['Cover ash particles', 'B', 'На паузе с обложки медленно сыпятся редкие частицы «пепла»; на play пропадают.'],
  ['CRT soft scanlines (opt-in)', 'C', 'Опциональные тонкие строки как у ЭЛТ-монитора поверх UI; по умолчанию выкл.'],
  ['Paper grain overlay', 'B', 'Поверх тёмного фона лёгкая бумажная зернистость, чтобы экран не был плоским чёрным.'],
  ['Letterpress track titles', 'A', 'Названия выглядят чуть вдавленными в фон (letterpress), как в печатном макете.'],
  ['Slow iris open on play', 'A', 'Старт воспроизведения открывает кадр круговой диафрагмой от центра обложки.'],
  ['Silence snow', 'S', 'В паузе между треками на фоне редко мигают одиночные пиксели, как снег в тишине.'],
  ['Accent smoke trail', 'B', 'В режиме плеера за курсором тянется короткий шлейф цвета акцента обложки.'],
  ['Dual-tone cover split', 'A', 'Обложка визуально делится на две половины по двум главным цветам палитры.'],
  ['Glyph clock in titlebar', 'C', 'В titlebar вместо цифр — стилизованные глифы часов текущего времени.'],
  ['Waveform as negative space', 'A', 'Форма волны «вырезана» из фона (негатив), а не нарисована яркой линией сверху.'],
  ['Soft vignette by energy', 'B', 'На тихих/спокойных треках края экрана темнеют сильнее; на громких виньетка слабее.'],
  ['Monochrome listen mode', 'S', 'Режим: весь интерфейс серый, цвет остаётся только у текущей обложки.'],
  ['Typography kerning animate', 'C', 'Межбуквенные интервалы названия чуть плывут при появлении текста.'],
  ['Corner light leak', 'B', 'В углу окна лёгкая засветка цветом акцента обложки, как световая утечка плёнки.'],
  ['Depth parallax album stack', 'A', 'Очередь/следующие треки — стопка обложек; при движении мыши слои чуть смещаются.'],
  ['Quiet chrome hide', 'S', 'Если мышь не двигается N секунд, titlebar и лишний хром плавно исчезают, остаётся музыка.'],
  ['Subtitles as film captions', 'B', 'Статус (комната, AFK, буфер) показывается как субтитры внизу кадра, не тостом.'],
  ['Halftone artist avatar', 'C', 'Аватар артиста/presence рисуется полутоновой сеткой, как газетная печать.'],
  ['Liquid progress morph', 'A', 'Полоска прогресса ведёт себя как жидкость: догоняет позицию с инерцией, не прыгает.'],
  ['Shadow type behind cover', 'B', 'За обложкой крупным полупрозрачным шрифтом имя артиста или название альбома.'],
  ['Seasonal UI frost', 'D', 'Зимой UI покрывается инеем/снежными оверлеями по календарю.'],
  ['Blink cursor on seek', 'C', 'Пока тянешь seek, на точке появляется мигающий «курсор» как в тексте.'],
  ['Matte vs gloss cover toggle', 'B', 'Переключатель: обложка матовая или с лёгким глянцевым бликом.'],
  ['Frame rate of rain on pause', 'A', 'Только на паузе по экрану идёт редкий пиксельный дождь; на play сразу стоп.'],
  ['Oblique projection cards', 'D', 'Карточки треков в изометрии/косой проекции вместо плоского грида.'],
  ['Tape hiss visualizer', 'B', 'На очень тихих фрагментах лёгкий визуальный «шум ленты» по краю волны.'],
  ['Fold crease on playlist', 'C', 'У плейлиста визуальный сгиб бумаги по краю списка.'],
  ['Color-blind safe accents', 'A', 'Акценты и статусы проверяются на контраст для дальтонизма; запасные палитры.'],
  ['Reduced-motion cinema cuts', 'S', 'Если в ОС включён reduced motion — смена трека жёстким кадром, без tween-анимаций.'],
  ['Micro film-grain on toast', 'C', 'На тостах едва заметное кинозерно.'],
  ['Inverted night flash', 'F', 'Короткая инверсия цветов экрана при событии (лайк/скип).'],
  ['3D tilt covers', 'D', 'Обложки наклоняются за курсором в 3D.'],
  ['Neon equalizer bars', 'F', 'Классические неоновые столбики эквалайзера на весь UI.'],
  ['Confetti on like', 'F', 'При лайке сыплется конфетти.'],
  ['Animated emoji reactions', 'F', 'Анимированные эмодзи-реакции на треки и в комнатах.'],
  ['Rainbow spectrum bg', 'F', 'Фон радужным спектром / радужным градиентом.'],
  ['Glassmorphism panels', 'D', 'Панели с размытым стеклом (glassmorphism).'],
  ['Particle burst skip', 'D', 'При скипе взрыв частиц от обложки.'],
  ['Holographic foil titles', 'C', 'Заголовки с эффектом голографической фольги.'],
  ['Cursor custom glyph set', 'B', 'В зоне плеера свой набор курсоров (play/seek/drag) вместо системного.'],
  ['Album art as window mask', 'A', 'Окно mini обрезано/замаскировано формой обложки, не просто прямоугольник.'],
  ['Slow zoom Ken Burns cover', 'B', 'В полном плеере обложка медленно зумится/плывёт (Ken Burns), пока играет трек.'],
  ['Typography hierarchy lock', 'A', 'Жёсткая шкала кеглей/весов шрифта по всему приложению, как в журнале.'],
  ['Ink stamp LIVE ROOM', 'B', 'Пока ты в активной комнате, на UI появляется штамп «LIVE ROOM».'],
  ['Desaturated library until play', 'S', 'До первого play библиотека обесцвечена; с стартом музыки появляется цвет.'],
  ['Edge glow only on buffer', 'A', 'Свечение края окна только пока трек буферизуется — честный индикатор сети.'],
  ['Hand-drawn focus ring', 'B', 'Кольцо клавиатурного фокуса выглядит как карандашная обводка, не системный outline.'],
  ['Stencil cut nav icons', 'C', 'Иконки навигации в стиле трафаретной вырезки.'],
  ['Posterize cover on AFK', 'B', 'В статусе AFK обложка упрощается posterize (мало цветов), как «заснула».'],
  ['Vertical Japanese title mode', 'C', 'Для JP-треков название можно показать вертикальным набором.'],
  ['Ultra-thin hairline grid', 'B', 'Фон с очень тонкой волосяной сеткой, как в макете/чертёже.'],
  ['Black crush lift slider', 'A', 'Слайдер «поднять чёрный» для OLED: чтобы тени не проваливались в 0.'],
  ['Cover mosaic home', 'D', 'Главный экран — коллаж/мозаика из многих обложек.'],
  ['Animated SVG logo mark', 'B', 'В About марка aoi чуть анимирована (дыхание/черточка), не статичный png.'],
  ['Silent film intertitles', 'A', 'Между альбомами/плейлистами короткий межзаголовок как в немом кино.'],
  ['Dock reflection mini', 'C', 'Под mini-плеером отражение, как у иконок в macOS Dock.'],
];

const tabs = [
  ['Listening diary', 'S', 'Отдельная вкладка: личный дневник что слушал, без ленты друзей и лайков.'],
  ['Tape deck workspace', 'A', 'Экран «кассетник»: плейлисты как сторона A/B, переворот кассеты = смена набора.'],
  ['Field notes', 'A', 'Вкладка заметок к трекам — текст, таймкоды, как полевой журнал.'],
  ['Cartography of taste', 'S', 'Карта вкуса: артисты/жанры как территории, по которым можно «ходить».'],
  ['Night desk', 'A', 'Ночной экран: только очередь, sleep timer и play/pause — без библиотеки.'],
  ['Archive bay', 'B', 'Архив: снятые лайки и удалённые локальные треки, чтобы вернуть или посмотреть.'],
  ['Signal room', 'A', 'Отдельная вкладка онлайн/presence без экрана аккаунта SoundCloud.'],
  ['Studio clock', 'B', 'Вкладка с большим часами/метрономом и BPM текущего трека.'],
  ['Courier inbox', 'B', 'Входящие (инвайты, апдейты) оформлены как почтовый ящик, не колокольчик.'],
  ['Vinyl crate', 'A', 'Локальная библиотека как ящик пластинок: полки, корешки, доставание пластинки.'],
  ['Ghost queue', 'S', 'Список треков, которые слушал наполовину и бросил — можно дослушать оттуда.'],
  ['Compare takes', 'B', 'Два трека на одном экране для быстрого A/B сравнения звука/версии.'],
  ['Radio telescope', 'A', 'Станции SoundCloud как шкала частот: крутишь «тюнер», ловишь станцию.'],
  ['Passport stamps', 'C', 'Коллекция «штампов» стран/городов по артистам из метаданных.'],
  ['Darkroom', 'A', 'Полноэкранная галерея обложек без кнопок — только листать арт.'],
  ['Score sheet', 'B', 'Партитура трека: твои пометки и таймкоды на временной шкале.'],
  ['Relay board', 'A', 'Комнаты как коммутатор: слоты, замки, кто на какой линии.'],
  ['Museum wing', 'C', 'Галерея старых состояний UI / скринов своих сессий.'],
  ['Training ground', 'D', 'Постоянная вкладка онбординга и обучения жестам.'],
  ['Marketplace skin', 'F', 'Магазин скинов и тем за деньги/поинты.'],
  ['Social feed wall', 'F', 'Лента активности друзей как соцсеть.'],
  ['AI DJ chat tab', 'F', 'Чат с ИИ-диджеем, который подбирает треки текстом.'],
  ['Crypto tip jar', 'F', 'Чаевые артистам криптой из плеера.'],
  ['Ads dashboard', 'F', 'Кабинет рекламы / промо внутри клиента.'],
  ['Clan wars', 'F', 'Кланы слушателей и соревнования между ними.'],
  ['Mood orchard', 'B', 'Дерево настроений: ветки → плейлисты/фильтры, без смайликов-кнопок.'],
  ['Transit map queue', 'A', 'Очередь как схема метро: станции = треки, пересадки = плейлисты.'],
  ['Ledger of skips', 'S', 'Таблица скипов: что, когда, на какой секунде — честный учёт привычек.'],
  ['Breathing room', 'A', 'Пустой экран на таймер (например 60с) тишины — отдых от интерфейса.'],
  ['Collaborative crate', 'A', 'Общий ящик треков с другом без полной комнаты синхронного плеера.'],
  ['Offline island', 'B', 'Только то, что уже в кеше/на диске — отдельный «остров» без сети.'],
  ['Repair bench', 'B', 'Мастерская: битые пути, пустые теги, битые обложки — чинить пачкой.'],
  ['Season capsule', 'C', 'Капсула сезона: что слушал этой зимой/летом, закрывается по дате.'],
  ['Stamp collection', 'D', 'Коллекционирование бейджей за действия в плеере.'],
  ['Lyrics atelier', 'B', 'Экран текстов с ручной разметкой строк и таймингом.'],
  ['Cue book', 'A', 'Книга cue-точек: сохранённые позиции внутри треков для быстрого прыжка.'],
  ['Mirror lobby', 'B', 'Превью: как тебя видят другие в presence (ник, статус, трек).'],
  ['Time capsule send', 'A', 'Отправить себе трек/плейлист на будущую дату — откроется потом.'],
  ['Parallel shelves', 'B', 'Две библиотеки рядом (например SC и local) на одном экране.'],
  ['Focus chamber', 'A', 'Огромная одна кнопка play/pause на весь экран, без остального UI.'],
  ['History filmstrip', 'B', 'История прослушиваний как киноплёнка кадров-обложек.'],
  ['Guest lounge', 'A', 'Отдельный экран режима гостя комнаты: что можно, что нельзя.'],
  ['Export atelier', 'B', 'Мастер экспорта плейлистов, обложек, дневника в файлы.'],
  ['Policy chapel', 'C', 'Тихий экран приватности: блоки, presence off, что шарится.'],
  ['Debug balcony', 'D', 'Открытая вкладка дебага логов и стейта для всех.'],
  ['Podcast annex', 'D', 'Отдельное крыло под подкасты/длинные речи.'],
  ['Video lounge', 'F', 'Просмотр видеоклипов внутри aoi.'],
  ['Shopping bag', 'F', 'Корзина мерча артистов.'],
  ['News ticker tab', 'F', 'Лента новостей музыки/индустрии.'],
  ['Achievement hall', 'F', 'Зал достижений и ачивок.'],
  ['Friend story rings', 'F', 'Сториз друзей кольцами как в Instagram.'],
  ['Map of listeners realtime', 'C', 'Карта мира с точками кто онлайн в aoi прямо сейчас.'],
  ['Sound design lab', 'B', 'Песочница эффектов (reverb и т.п.) отдельно от основного EQ.'],
  ['Ritual calendar', 'A', 'Календарь твоих ритуалов слушания (утро/ночь/воскресенье).'],
  ['Inbox for stems', 'C', 'Приёмка stem-файлов (дорожек) к трекам.'],
  ['Karaoke booth', 'D', 'Режим караоке с текстом подпевания.'],
  ['VR theater', 'F', 'VR-режим прослушивания.'],
  ['Multi-account switcher bay', 'A', 'Переключение нескольких аккаунтов SoundCloud без полного логаута.'],
  ['Local network jukebox', 'A', 'Джукбокс по LAN: другие устройства в сети кладут треки в очередь.'],
  ['Print studio', 'B', 'Сверстать и напечатать/сохранить постер текущего трека.'],
];

const extend = [
  ['Cover-aware EQ presets', 'S', 'По цветам обложки предлагается пресет EQ (тёплый/яркий/тёмный и т.д.).'],
  ['Station breadcrumb trail', 'A', 'Над станцией SC цепочка: откуда пришёл (артист → похожие → станция).'],
  ['Like with aftertaste fade', 'B', 'Лайк сопровождается тем же коротким aftertaste-фейдом, что и скип.'],
  ['Mini player chapter marks', 'A', 'На полоске mini можно ставить метки-главы и прыгать по ним.'],
  ['Room suggestion veto', 'A', 'Гость комнаты может soft-veto предложенный трек без кика.'],
  ['Presence listening radius', 'B', 'В онлайне видны только люди со схожим вкусом (настраиваемый «радиус»).'],
  ['Discord chapter timestamps', 'A', 'В Discord RPC уходят метки глав/кусков трека, не только название.'],
  ['Crossfade by genre match', 'S', 'Между похожими треками кроссфейд длиннее/мягче, между далёкими — короче.'],
  ['Local folder watch live', 'A', 'Папка музыки следится вживую: новый файл появляется без ручного скана.'],
  ['SC likes delta toast', 'B', 'После синка тост: сколько новых лайков с прошлого раза.'],
  ['Playlist cover collage auto', 'C', 'Обложка плейлиста собирается коллажем из треков автоматически.'],
  ['Sleep timer by tracks left', 'A', 'Таймер сна: «ещё N треков», а не только минуты.'],
  ['Needle peek scrub upgrade', 'A', 'Улучшенный peek при наведении на прогресс: точнее превью и громкость.'],
  ['Blocked artists library dim', 'B', 'Треки заблокированных артистов в ленте затемнены/скрыты по правилу.'],
  ['Invite with track attached', 'S', 'Инвайт в комнату несёт конкретный трек: заходишь уже в контексте этой песни.'],
  ['EQ per source mode', 'A', 'Отдельные настройки EQ для SoundCloud и для локальной папки.'],
  ['Tray progress pie', 'B', 'Иконка в трее показывает прогресс трека маленьким кольцом/pie.'],
  ['Accent lock per playlist', 'B', 'У плейлиста свой зафиксированный цвет акцента, не от текущего трека.'],
  ['Room host baton pass', 'A', 'Хост комнаты одним жестом передаёт управление другому участнику.'],
  ['Update changelog cinema', 'C', 'После обновления changelog показывается титрами, как в кино.'],
  ['Offline SC art priority', 'A', 'Без сети сначала берутся локально сохранённые обложки SC, не битые URL.'],
  ['Smart repeat A-B loop', 'A', 'Петля A–B: отмечаешь две точки в треке, крутится только этот кусок.'],
  ['Volume duck on invite', 'B', 'При входящем инвайте музыка на секунду приглушается.'],
  ['Search by color', 'S', 'Поиск/фильтр библиотеки по цвету обложки (пипетка или палитра).'],
  ['Duplicates merge assistant', 'A', 'Находит один и тот же трек в local и SC и предлагает склеить/выбрать источник.'],
  ['Tag paintbrush', 'B', 'Кисть: выделяешь несколько треков и красить их одним тегом.'],
  ['Custom title history undo', 'B', 'История кастомных названий с отменой на шаг назад.'],
  ['Presence idle artwork freeze', 'C', 'В idle у presence «замораживается» арт, чтобы не мелькал.'],
  ['Mini always-on-top per monitor', 'B', 'Mini запоминает, на каком мониторе висел и поверх каких окон.'],
  ['Keyboard cinema mode', 'A', 'Полный набор хоткеев без огромных подсказок на экране.'],
  ['HLS quality ladder', 'A', 'Явный выбор качества HLS-потока (и авто по сети).'],
  ['Room locks UI clarity', 'A', 'Замки слотов комнаты читаются сразу: кто залочил, что нельзя трогать.'],
  ['Likes cache age badge', 'C', 'Бейдж «кешу N часов» рядом с лайками SC.'],
  ['Discord pause privacy', 'A', 'На паузе Discord может скрывать трек или показывать «paused» — довести до конца.'],
  ['Cover extract for missing tags', 'B', 'Если тегов нет, из цвета обложки ставится временный тег настроения.'],
  ['Shuffle seed share', 'S', 'Код seed шаффла: у всех в комнате одинаковый случайный порядок.'],
  ['Gapless local album', 'A', 'Альбом из локальных файлов играет без паузы между треками (gapless).'],
  ['Waveform cache on disk', 'B', 'Посчитанная волна трека кешируется на диск, не пересчитывается каждый раз.'],
  ['SC relogin soft banner', 'B', 'Если сессия SC протухла — мягкий баннер «войти снова», без жёсткой ошибки.'],
  ['Playlist export M3U+', 'B', 'Экспорт плейлиста в M3U плюс рядом папка/ссылки на обложки.'],
  ['Accent from desktop wallpaper', 'D', 'Цвет акцента берётся с обоев рабочего стола Windows.'],
  ['Spotify import', 'C', 'Импорт плейлистов/лайков из Spotify в aoi.'],
  ['YouTube side load', 'F', 'Подтягивание аудио с YouTube в библиотеку.'],
  ['TikTok sounds panel', 'F', 'Панель звуков TikTok внутри плеера.'],
  ['Auto-post to Twitter', 'F', 'Автопост «сейчас слушаю» в X/Twitter.'],
  ['NFT cover verify', 'F', 'Проверка NFT-обложек / web3-метаданных.'],
  ['AI playlist name gen', 'D', 'ИИ сам придумывает название плейлиста.'],
  ['Loudness war normalize always', 'B', 'Всегда нормализовать громкость треков, чтобы не прыгала.'],
  ['ReplayGain local', 'A', 'Для локальных файлов читать/считать ReplayGain и выравнивать уровень.'],
  ['SC comment read-only pane', 'C', 'Панель комментариев SoundCloud только для чтения.'],
  ['Artist radio from local seed', 'A', 'Из локального трека запускаешь SC-станцию «похожего» артиста.'],
  ['Friends-only station', 'B', 'Станция видна только друзьям из presence, не всем.'],
  ['Mute member in room audio', 'A', 'Если появится voice в комнате — мьют конкретного участника.'],
  ['Room transcript of track IDs', 'B', 'Лог id треков, что играли в комнате за сессию, с экспортом.'],
  ['Backup settings to file', 'A', 'Кнопка: сохранить settings.json бэкапом и восстановить из файла.'],
  ['Import seWer settings deep', 'B', 'Глубже перенос настроек/данных со старого seWer.'],
  ['Per-track speed 0.9–1.1', 'C', 'Скорость трека в узком диапазоне без сильного питча.'],
  ['Bookmark moment share code', 'A', 'Код момента: трек + секунда; друг открывает сразу в этой точке.'],
  ['Smart folder playlists', 'A', 'Виртуальные плейлисты по правилам (папка, тег, год, длительность).'],
  ['Cover download pack', 'B', 'Скачать пачкой обложки текущей библиотеки/плейлиста на диск.'],
];

const logic = [
  ['Skip regret undo 5s', 'S', 'После скипа ~5 секунд можно вернуть предыдущий трек одной кнопкой/жестом.'],
  ['Almost-finished resume smart', 'S', 'Треки, дослушанные на 70–90%, предлагаются дослушать с места остановки — без пыльной полки.'],
  ['Anti-whiplash shuffle', 'A', 'Шаффл старается не ставить подряд треки с очень разным BPM/энергией.'],
  ['Cooldown on same artist', 'A', 'Правило: не крутить одного артиста три раза подряд в шаффле.'],
  ['Silent track detect', 'A', 'Детект почти тихих/битых файлов: скип или пометка, а не минута тишины.'],
  ['Prebuffer next intelligently', 'A', 'Следующий трек пребуферится раньше на плохой сети и позже на хорошей.'],
  ['Offline grace for SC', 'A', 'Краткий обрыв сети не рвёт сессию SC сразу — короткий grace-период.'],
  ['Seek keyframe snap HLS', 'B', 'Seek по HLS притягивается к ближайшему ключкадру, меньше артефактов.'],
  ['Queue persistence crash-safe', 'S', 'Очередь и позиция пишутся так, что после краша восстанавливаются.'],
  ['Atomic settings write', 'A', 'settings.json пишется атомарно (tmp + rename), чтобы не портился при выключении.'],
  ['Backoff SC rate limits', 'A', 'При 429/лимитах SC запросы замедляются по backoff, а не спамят.'],
  ['Corrupt cache quarantine', 'A', 'Битая обложка в кеше уходит в карантин, не ломает UI на каждом рендере.'],
  ['Deterministic room sync clock', 'S', 'Синхрон комнаты по единым monotonic-часам, меньше рассинхрона play/pause.'],
  ['Guest lag compensator', 'A', 'Гость с лагом сети догоняет хоста с компенсацией задержки.'],
  ['Invite idempotency', 'B', 'Повтор одного инвайта не создаёт дубликаты уведомлений.'],
  ['Presence token rotate', 'A', 'Токен presence периодически ротируется, старый перестаёт работать.'],
  ['Update signature verify strict', 'A', 'Строже проверять цепочку обновления (hash/источник) перед установкой.'],
  ['Rollback last update', 'A', 'Откатить на предыдущий установленный билд одной кнопкой.'],
  ['Memory cap for art decode', 'B', 'Лимит памяти на декод обложек: старые выгружаются из RAM.'],
  ['Debounced library rescan', 'B', 'Скан папки не стартует на каждый file event — debounce пачкой.'],
  ['Play intent vs UI race fix', 'S', 'Убрать гонки: быстрые клики play/pause/next не дают рассинхрон UI и звука.'],
  ['Focus steal prevention', 'A', 'Тосты и панели не крадут фокус клавиатуры/окна во время печати.'],
  ['IME-safe hotkeys', 'B', 'Глобальные хоткеи не срабатывают, пока печатаешь через IME (JP/CN и т.д.).'],
  ['Multi-window audio single owner', 'A', 'Main и mini делят один audio owner: не два параллельных потока.'],
  ['Clock drift fix sleep timer', 'B', 'Sleep timer считает по performance.now, не по системным часам, которые могут прыгать.'],
  ['Partial download resume setup', 'B', 'Скачивание инсталлятора обновления можно продолжить после обрыва.'],
  ['Hash verify before quit-replace', 'A', 'Сначала полная проверка hash, потом выход и замена exe — не наоборот.'],
  ['SC cookie jar healthcheck', 'A', 'Перед тяжёлыми SC-запросами проверка, что cookie jar живой и не пустой.'],
  ['Local tag read timeout', 'B', 'Чтение тегов файла обрывается по таймауту, чтобы скан не вис.'],
  ['Playlist cycle detect', 'C', 'Детект циклов, если плейлист ссылается сам на себя через вложенность.'],
  ['Smart crossfade cancel on seek', 'A', 'Seek посреди кроссфейда корректно отменяет фейд, без двойного звука.'],
  ['Aftertaste cancel taxonomy', 'B', 'Явные причины отмены aftertaste (next/seek/stop) для предсказуемого поведения.'],
  ['Volume curve perceptual', 'A', 'Слайдер громкости по логарифмической/перцептивной кривой, не линейной.'],
  ['Equal-loudness preview peek', 'B', 'Peek превью на seek с выровненной громкостью относительно основного трека.'],
  ['Sticky error track quarantine', 'A', 'Трек с ошибкой стрима не крутится в бесконечном retry — в карантин списка.'],
  ['Network quality class', 'B', 'Класс сети (good/ok/bad) меняет буфер, качество и частоту presence.'],
  ['Room state CRDT-lite', 'A', 'Стейт комнаты сходится при конфликтах (кто что нажал) без полного хаоса.'],
  ['Undo stack for library ops', 'A', 'Undo удаления из плейлиста / переименования на несколько шагов.'],
  ['Safe empty-state machines', 'B', 'Пустые экраны (нет лайков/папки) — явные состояния, не «белый экран».'],
  ['Hydration vs live settings merge', 'A', 'Загрузка settings с диска не затирает только что изменённые в UI поля.'],
  ['Idle GC for blob URLs', 'B', 'Неиспользуемые blob: URL обложек освобождаются, меньше утечек памяти.'],
  ['Strict media:// allowlist', 'A', 'media:// отдаёт только разрешённые пути библиотеки, ничего лишнего.'],
  ['Crash log breadcrumb UI', 'B', 'Перед крашем пишутся последние UI-действия — проще отладить.'],
  ['Startup warm path', 'A', 'Быстрый старт: сразу тёмный shell, данные догружаются без белой вспышки.'],
  ['Predictive JIT next art', 'B', 'Обложка следующего трека подгружается заранее.'],
  ['Adaptive poll presence', 'A', 'В фоне presence опрашивается реже, в фокусе — чаще.'],
  ['Battery saver mode', 'B', 'На батарее реже анимации и тяжелее эффекты режутся.'],
  ['Thermal soft degrade', 'C', 'При перегреве (если доступно) снизить нагрузку UI/декода.'],
  ['Auto EQ learn', 'D', 'ML сам учит EQ под твои правки со временем.'],
  ['Always-on mic BPM', 'F', 'Постоянно слушать микрофон и подстраивать BPM/светомузыку.'],
  ['Keylogger-like global hooks', 'F', 'Глобальный хук всех клавиш системы вне окна aoi.'],
  ['Hidden analytics phone-home', 'F', 'Скрытая аналитика на сервер без явного согласия.'],
  ['DRM local files', 'F', 'DRM/ограничения на свои локальные файлы.'],
  ['Force cloud library only', 'F', 'Запретить локальную папку, только облако.'],
  ['Random delete surprise clean', 'F', 'Случайное удаление треков «для чистоты».'],
  ['Aggressive auto-update no consent', 'F', 'Обновление без спроса и без кнопки.'],
  ['Busy-wait audio thread', 'F', 'Крутить CPU busy-wait в аудиопотоке.'],
  ['Sync settings to public gist', 'D', 'Синк настроек в публичный GitHub Gist.'],
  ['Heuristic genre ML local', 'C', 'Локально угадывать жанр по аудио без облака.'],
  ['Dead-letter queue for SC writes', 'A', 'Неудачные like/unlike копятся в очередь и повторяются позже.'],
];

const ritual = [
  ['Dawn queue', 'S', 'Ночью можно собрать очередь, которая сама стартует утром в заданное время.'],
  ['One-track tea ceremony', 'A', 'Режим: один трек целиком; скип заблокирован на N минут.'],
  ['Rain window listen', 'A', 'Под трек опционально кладётся локальный слой дождя/окна.'],
  ['Letter to future self track', 'S', '«Письмо себе»: трек откроется тебе через неделю/месяц.'],
  ['Commuter mask mode', 'B', 'Профиль «дорога»: выше громкость по умолчанию, меньше анимаций.'],
  ['Funeral for unliked', 'C', 'Короткая анимация/экран прощания при снятии лайка.'],
  ['First snow playlist lock', 'B', 'Плейлист открывается только в «зимние» даты по календарю.'],
  ['Candle timer', 'A', 'Таймер сна гасит яркость UI как свечу — к нулю к концу.'],
  ['Shared silence minute', 'A', 'В комнате все синхронно минуту тишины (mute), потом продолжают.'],
  ['Vow: finish album', 'A', 'Обет: этот альбом до конца без шаффла и без скипа альбома.'],
  ['Secret knock to open mini', 'B', 'Секретный «стук» по titlebar (ритм кликов) открывает mini.'],
  ['Listening wedding playlist', 'D', 'Шутливый режим «свадьбы плейлистов».'],
  ['Horoscope DJ', 'F', 'Подбор музыки по гороскопу.'],
  ['Astrology BPM', 'F', 'BPM/очередь от знака зодиака.'],
  ['Ouija queue', 'F', '«Мистическая» очередь якобы из ответов спиритической доски.'],
  ['Step-counter unlock tracks', 'D', 'Треки открываются после N шагов с фитнес-трекера.'],
  ['Paywall mood', 'F', 'Настроения/режимы за paywall.'],
  ['Public shame skip counter', 'F', 'Публичный счётчик скипов «для стыда».'],
  ['Location stalk playlist', 'F', 'Плейлист строго по GPS-локации пользователя.'],
  ['Always listen together forced', 'F', 'Нельзя слушать в одиночку — только с кем-то онлайн.'],
  ['Analog Sunday', 'A', 'По воскресеньям по умолчанию только local files, SC спрятан.'],
  ['Blackout listening', 'S', 'Почти чёрный экран: звук + пробел play/pause, UI скрыт.'],
  ['Postcard export', 'B', 'Сгенерить «открытку» что слушал сегодня (картинка/PDF).'],
  ['Habit chain without streaks UI', 'A', 'Цепочка дней слушания без огней/душного streak-гейминга.'],
  ['Library spring cleaning', 'B', 'Режим уборки: давно не слушавшиеся треки предложить архивировать.'],
  ['Apology replay', 'B', 'Если скипнул трек в комнате — жест «извини», ставит его снова.'],
  ['Window seat mode', 'B', 'Медленный параллакс «вида из окна» как фон режима слушания.'],
  ['Intermission bell', 'C', 'Между альбомами короткий звук/колокол антракта.'],
  ['Monastery mode', 'A', 'Выкл presence, инвайты и уведомления — только музыка.'],
  ['Shipwatch night', 'B', 'Ночная «вахта»: лог что играло часами, как судовой журнал.'],
  ['Train timetable playlists', 'C', 'Плейлисты как расписание поездов (время → набор).'],
  ['Museum headphone etiquette', 'B', 'В комнате короткие подсказки этикета (не спамить скипом и т.д.).'],
  ['Pen pal track exchange', 'A', 'Раз в день обмен одним треком с выбранным человеком.'],
  ['Burn after listening link', 'B', 'Ссылка на комнату/трек сгорает после одного прослушивания.'],
  ['Seasonal hardware theme', 'D', 'Праздничные скины UI под Новый год и т.п.'],
  ['Fortune cookie toast', 'D', 'Случайные цитаты в тостах.'],
  ['Dice queue', 'C', 'Кнопка-кубик выбирает следующий трек случайно из пула.'],
  ['Pomodoro sidekick', 'B', 'Рядом таймер помодоро, не встроенный в плеер как игра.'],
  ['Journal prompt after album', 'A', 'После альбома один короткий вопрос в дневник («как было?»).'],
  ['Lights out hardware hook', 'C', 'Интеграция с умными лампами: свет под обложку.'],
  ['Vinyl crackle Sunday', 'B', 'По воскресеньям опциональный слой треска винила под музыку.'],
  ['No-skip church hour', 'A', 'Добровольный час без скипа (можно выйти из режима).'],
  ['Memory lane year ago', 'A', '«Год назад в этот день ты слушал…» — только тебе, без соцшеринга.'],
  ['Farewell mix on uninstall', 'C', 'При удалении собрать прощальный микс из любимого.'],
  ['Deep work berm', 'A', 'На N часов глушит presence/инвайты «валом» для глубокой работы.'],
  ['Tide chart energy', 'C', 'График «приливов» твоей активности слушания по часам.'],
  ['Campfire room voice later', 'B', 'Задел: тихий voice у «костра» комнаты — не сейчас, архитектура позже.'],
  ['Typewriter search', 'B', 'Опциональный звук печатной машинки при наборе в поиске.'],
  ['Ink diary export PDF', 'B', 'Экспорт дневника прослушиваний в аккуратный PDF.'],
  ['Quiet hours OS sync', 'A', 'Связка с Focus Assist Windows: quiet hours = mute presence.'],
  ['Neighbor volume warn', 'C', 'Поздно вечером предупреждение, что громкость высокая.'],
  ['Bookmark ritual slots', 'B', 'Слоты «утро / дорога / ночь» с разными очередями.'],
  ['Handshake listen', 'A', 'Двое должны нажать play почти одновременно, чтобы комната стартовала.'],
  ['Sealed envelope playlist', 'A', 'Плейлист-конверт открывается только в выбранную дату.'],
  ['Last light battery rite', 'B', 'При низком заряде уже есть dim — оформить как явный «обряд last light».'],
  ['Echo of room after leave', 'B', 'После выхода из комнаты короткий отголосок последнего трека и тишина.'],
  ['Solo residency week', 'A', 'Неделя одного артиста: UI подсвечивает только его.'],
  ['Cartography walk mode', 'C', 'Связка прогулки (шаги) с продвижением по карте вкуса.'],
  ['Paper plane invite', 'B', 'Анимация инвайта бумажным самолётиком вместо обычного тоста.'],
  ['Midnight door', 'S', 'После полуночи открывается скрытый набор треков/настроения на эту ночь.'],
];

function pack(list, cat) {
  if (list.length !== 60) throw new Error(cat + ' len ' + list.length);
  return list.map(([name, tier, what], i) => ({
    id: cat[0].toUpperCase() + String(i + 1).padStart(2, '0'),
    name,
    cat,
    catLabel: cats[cat],
    tier,
    what,
  }));
}

const all = [
  ...pack(visual, 'visual'),
  ...pack(tabs, 'tabs'),
  ...pack(extend, 'extend'),
  ...pack(logic, 'logic'),
  ...pack(ritual, 'ritual'),
];
if (all.length !== 300) throw new Error('total ' + all.length);

const src = `import { useMemo, useState, Stack, Row, H1, H2, Text, Card, CardHeader, CardBody, Table, Select, Pill, Stat, Grid, Divider, Callout, Spacer } from 'cursor/canvas';

const FEATURES = ${JSON.stringify(all)} as Array<{
  id: string;
  name: string;
  cat: string;
  catLabel: string;
  tier: string;
  what: string;
}>;

const TIER_ORDER = ['S', 'A', 'B', 'C', 'D', 'F'] as const;

const CAT_OPTS = [
  { value: 'all', label: 'Все разделы' },
  { value: 'visual', label: 'Визуал (60)' },
  { value: 'tabs', label: 'Новые вкладки (60)' },
  { value: 'extend', label: 'Дополнения (60)' },
  { value: 'logic', label: 'Логика (60)' },
  { value: 'ritual', label: 'Ритуалы и контекст (60)' },
];

const TIER_WHY: Record<string, string> = {
  S: 'Уникально под aoi, сильно меняет ощущение — делать в первую очередь.',
  A: 'Отличный fit, делать рано.',
  B: 'Хорошо, но после ядра.',
  C: 'Мелкий polish.',
  D: 'Слабо стыкуется с минимализмом.',
  F: 'Ломает продукт / шум / creepy.',
};

export default function Aoi300Tierlist() {
  const [cat, setCat] = useState('all');
  const [tier, setTier] = useState('all');

  const filtered = useMemo(() => {
    return FEATURES.filter((f) => (cat === 'all' || f.cat === cat) && (tier === 'all' || f.tier === tier)).sort((a, b) => {
      const td = TIER_ORDER.indexOf(a.tier as (typeof TIER_ORDER)[number]) - TIER_ORDER.indexOf(b.tier as (typeof TIER_ORDER)[number]);
      if (td !== 0) return td;
      return a.id.localeCompare(b.id);
    });
  }, [cat, tier]);

  const counts = useMemo(() => {
    const c: Record<string, number> = { S: 0, A: 0, B: 0, C: 0, D: 0, F: 0 };
    for (const f of FEATURES) c[f.tier]++;
    return c;
  }, []);

  return (
    <Stack gap={24}>
      <Stack gap={8}>
        <H1>aoi — 300 идей вместо пыльной полки</H1>
        <Text tone="secondary">
          Бэклог идей. Колонка «Что делает» — только суть функции. Почему тир S/A/… — в блоке критериев сверху, не в каждой строке.
        </Text>
      </Stack>

      <Callout tone="info" title="Версии">
        Значимое (фича/визуал add/remove) → патч 1.0.n. Мелочи → буква: 1.0.6a / в Cargo как 1.0.6-a.
      </Callout>

      <Grid columns={6} gap={12}>
        <Stat label="Tier S" value={String(counts.S)} tone="success" />
        <Stat label="Tier A" value={String(counts.A)} />
        <Stat label="Tier B" value={String(counts.B)} />
        <Stat label="Tier C" value={String(counts.C)} />
        <Stat label="Tier D" value={String(counts.D)} tone="warning" />
        <Stat label="Tier F" value={String(counts.F)} tone="danger" />
      </Grid>

      <Card>
        <CardHeader>Критерии тиров</CardHeader>
        <CardBody>
          <Stack gap={6}>
            <Row gap={10} align="center">
              <Pill tone="success" size="sm">S</Pill>
              <Text size="small">{TIER_WHY.S}</Text>
            </Row>
            <Row gap={10} align="center">
              <Pill tone="neutral" size="sm">A</Pill>
              <Text size="small">{TIER_WHY.A}</Text>
            </Row>
            <Row gap={10} align="center">
              <Pill tone="neutral" size="sm">B</Pill>
              <Text size="small">{TIER_WHY.B}</Text>
            </Row>
            <Row gap={10} align="center">
              <Pill tone="neutral" size="sm">C</Pill>
              <Text size="small">{TIER_WHY.C}</Text>
            </Row>
            <Row gap={10} align="center">
              <Pill tone="warning" size="sm">D</Pill>
              <Text size="small">{TIER_WHY.D}</Text>
            </Row>
            <Row gap={10} align="center">
              <Pill tone="deleted" size="sm">F</Pill>
              <Text size="small">{TIER_WHY.F}</Text>
            </Row>
          </Stack>
        </CardBody>
      </Card>

      <H2>Пять разделов</H2>
      <Grid columns={2} gap={12}>
        <Card>
          <CardHeader trailing="60">Визуал</CardHeader>
          <CardBody>
            <Text size="small">Атмосфера, типографика, motion, обложки.</Text>
          </CardBody>
        </Card>
        <Card>
          <CardHeader trailing="60">Новые вкладки</CardHeader>
          <CardBody>
            <Text size="small">Отдельные экраны, которых ещё нет в навигации.</Text>
          </CardBody>
        </Card>
        <Card>
          <CardHeader trailing="60">Дополнения</CardHeader>
          <CardBody>
            <Text size="small">Надстройки над SC, local, rooms, EQ, mini, Discord.</Text>
          </CardBody>
        </Card>
        <Card>
          <CardHeader trailing="60">Логика</CardHeader>
          <CardBody>
            <Text size="small">Очередь, синхрон, кеш, гонки, надёжность.</Text>
          </CardBody>
        </Card>
      </Grid>
      <Card>
        <CardHeader trailing="60">Ритуалы и контекст</CardHeader>
        <CardBody>
          <Text size="small">Ритуалы слушания, время суток, жесты комнат, дневник.</Text>
        </CardBody>
      </Card>

      <Divider />

      <H2>Тирлист</H2>
      <Row gap={12} align="center">
        <Select value={cat} onChange={setCat} options={CAT_OPTS} />
        <Select
          value={tier}
          onChange={setTier}
          options={[{ value: 'all', label: 'Все тиры' }, ...TIER_ORDER.map((t) => ({ value: t, label: \`Tier \${t} (\${counts[t]})\` }))]}
        />
        <Text tone="secondary" size="small">{\`\${filtered.length} функций\`}</Text>
      </Row>

      <Table
        headers={['Tier', 'ID', 'Функция', 'Раздел', 'Что делает']}
        rows={filtered.map((f) => [f.tier, f.id, f.name, f.catLabel, f.what])}
        rowTone={filtered.map((f) => (f.tier === 'S' ? 'success' : f.tier === 'F' ? 'danger' : f.tier === 'D' ? 'warning' : undefined))}
        striped
        stickyHeader
      />

      <Spacer height={12} />
      <Text tone="secondary" size="small">
        Замена пыльной полке (S): Skip regret undo, Almost-finished resume, Ghost queue, Listening diary, Midnight door.
      </Text>
    </Stack>
  );
}
`;

fs.writeFileSync(out, src, 'utf8');
console.log('ok', all.length);
