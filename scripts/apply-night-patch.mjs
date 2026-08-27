/**
 * Apply overnight 1.1.0 feature hooks into ui/index.html
 * Run: node scripts/apply-night-patch.mjs
 */
import fs from 'fs';

const path = new URL('../ui/index.html', import.meta.url);
let html = fs.readFileSync(path, 'utf8');
const had = (s) => html.includes(s);

function rep(oldStr, newStr, label) {
  if (!html.includes(oldStr)) {
    console.warn('MISS', label);
    return false;
  }
  html = html.replace(oldStr, newStr);
  console.log('OK', label);
  return true;
}

// CSS extras
if (!had('aoi-night-css')) {
  rep(
    `  #app-shell.last-light::after {
    background:
      radial-gradient(ellipse 115% 80% at 50% -8%, rgba(255,150,80,0.14) 0%, transparent 52%),
      radial-gradient(ellipse at center, transparent 45%, rgba(40,16,6,0.5) 100%);
  }`,
    `  #app-shell.last-light::after {
    background:
      radial-gradient(ellipse 115% 80% at 50% -8%, rgba(255,150,80,0.14) 0%, transparent 52%),
      radial-gradient(ellipse at center, transparent 45%, rgba(40,16,6,0.5) 100%);
  }
  /* aoi-night-css */
  :root { --black-lift: 0; }
  #app-shell { filter: brightness(calc(1 + var(--black-lift) * 0.35)); }
  #app-shell.chrome-quiet #titlebar,
  #app-shell.chrome-quiet .sidebar-chrome { opacity:0; pointer-events:none; transition:opacity 0.55s ease; }
  #app-shell.lib-desat .home-grid, #app-shell.lib-desat .track-list-wrap { filter:grayscale(1) saturate(0); transition:filter 0.7s ease; }
  #app-shell.has-played .home-grid, #app-shell.has-played .track-list-wrap { filter:none; }
  #app-shell.buffering { box-shadow: inset 0 0 0 1px rgba(var(--accent-rgb),0.35), inset 0 0 40px rgba(var(--accent-rgb),0.08); }
  #app-shell.pause-rain::before {
    background-image: radial-gradient(1px 8px at 20% 30%, rgba(255,255,255,0.18), transparent),
      radial-gradient(1px 10px at 60% 10%, rgba(255,255,255,0.12), transparent),
      radial-gradient(1px 6px at 80% 50%, rgba(255,255,255,0.1), transparent);
    background-size: 120px 180px; animation: aoiRain 1.2s linear infinite; opacity:0.35;
  }
  @keyframes aoiRain { to { background-position: 0 180px; } }
  @keyframes aoiBreathe { 0%,100%{ transform:scale(1);} 50%{ transform:scale(1.018);} }
  .aoi-breathe { animation: aoiBreathe 1.6s ease-in-out infinite; transform-origin:center; }
  .aoi-dual-tone { position:relative; overflow:hidden; }
  .aoi-dual-tone::after {
    content:''; position:absolute; inset:0; pointer-events:none;
    background: linear-gradient(105deg, rgba(var(--accent-rgb),0.35) 0 48%, transparent 52% 100%);
    mix-blend-mode: soft-light;
  }
  html.aoi-cinema *, html.aoi-cinema *::before, html.aoi-cinema *::after {
    animation:none !important; transition:none !important;
  }`,
    'css'
  );
}

// i18n RU bits
rep(
  `    dusty_shelf:'пыльная полка', dusty_blow:'дослушать',`,
  `    notif_sound:'звук уведомлений', notif_sound_sub:'тихий сигнал и приглушение музыки',
    backup_settings:'бэкап настроек', backup_ok:'бэкап сохранён', backup_err:'не удалось сохранить бэкап',
    friends_section:'друзья', online_section:'онлайн', add_friend:'в друзья', remove_friend:'из друзей',
    crate_title:'общий ящик', crate_add:'в ящик', crate_empty:'ящик пуст — добавьте трек с друга',
    accounts_title:'аккаунты SoundCloud', accounts_add:'добавить аккаунт', accounts_switch:'переключить',
    accounts_active:'активный', sleep_tracks:'сон по трекам', sleep_tracks_sub:'сколько треков до паузы',
    deep_work:'глубокая работа', deep_work_sub:'заглушить presence на N часов',
    quiet_hours:'тихие часы Windows', quiet_hours_sub:'синхрон с Focus Assist если доступен',
    black_lift:'подъём чёрного', black_lift_sub:'OLED: чуть поднять тени',
    hls_quality:'качество потока', hls_auto:'авто',
    moment_copy:'код момента скопирован', moment_bad:'неверный код момента',
    like_queued_ok:'лайк сохранён', like_queued_fail:'лайк не прошёл — в уведомлениях',
    veto_btn:'veto', veto_sent:'предложение отклонено',
    dawn_queue:'утренняя очередь', dawn_queue_sub:'старт очереди в выбранный час',
    candle_timer:'свеча',`,
  'i18n-ru-dusty-miss-ok'
);

// Fix: dusty already removed - insert after notif strings
if (!had("notif_sound:'звук уведомлений'")) {
  rep(
    `    notif_title:'уведомления',`,
    `    notif_title:'уведомления',
    notif_sound:'звук уведомлений', notif_sound_sub:'тихий сигнал и приглушение музыки',
    notif_like_ok:'лайк сохранён', notif_like_fail:'не удалось сохранить лайк',
    notif_unlike_ok:'лайк убран', notif_unlike_fail:'не удалось убрать лайк',
    backup_settings:'бэкап настроек', backup_ok:'бэкап сохранён', backup_err:'не удалось сохранить бэкап',
    friends_section:'друзья', online_section:'онлайн', add_friend:'в друзья', remove_friend:'из друзей',
    crate_title:'общий ящик', crate_add:'в ящик', crate_empty:'ящик пуст',
    accounts_title:'аккаунты SoundCloud', accounts_add:'добавить аккаунт', accounts_switch:'переключить',
    accounts_active:'активный', sleep_tracks:'сон по трекам', sleep_tracks_sub:'сколько треков до паузы (0 = выкл)',
    deep_work:'глубокая работа', deep_work_sub:'заглушить presence на часы',
    quiet_hours:'тихие часы', quiet_hours_sub:'меньше опроса presence ночью',
    black_lift:'подъём чёрного', black_lift_sub:'OLED: чуть поднять тени',
    hls_quality:'качество потока', hls_auto:'авто',
    moment_copy:'код момента скопирован', moment_bad:'неверный код момента',
    veto_btn:'veto', veto_sent:'предложение отклонено',
    dawn_queue:'утренняя очередь', dawn_queue_sub:'час автостарта (0–23, −1 выкл)',
    candle_timer:'свеча (яркость к нулю)', chapter_add:'метка',`,
    'i18n-ru'
  );
}

if (!had("notif_sound:'notification sound'")) {
  rep(
    `    notif_title:'notifications',`,
    `    notif_title:'notifications',
    notif_sound:'notification sound', notif_sound_sub:'soft chime + brief duck',
    notif_like_ok:'like saved', notif_like_fail:'could not save like',
    notif_unlike_ok:'like removed', notif_unlike_fail:'could not remove like',
    backup_settings:'backup settings', backup_ok:'backup saved', backup_err:'backup failed',
    friends_section:'friends', online_section:'online', add_friend:'add friend', remove_friend:'remove friend',
    crate_title:'shared crate', crate_add:'to crate', crate_empty:'crate is empty',
    accounts_title:'SoundCloud accounts', accounts_add:'add account', accounts_switch:'switch',
    accounts_active:'active', sleep_tracks:'sleep by tracks', sleep_tracks_sub:'tracks until pause (0 = off)',
    deep_work:'deep work', deep_work_sub:'mute presence for hours',
    quiet_hours:'quiet hours', quiet_hours_sub:'slower presence polling at night',
    black_lift:'black lift', black_lift_sub:'OLED: lift crushed blacks',
    hls_quality:'stream quality', hls_auto:'auto',
    moment_copy:'moment code copied', moment_bad:'bad moment code',
    veto_btn:'veto', veto_sent:'suggestion vetoed',
    dawn_queue:'dawn queue', dawn_queue_sub:'auto-start hour (0–23, −1 off)',
    candle_timer:'candle (fade UI)', chapter_add:'mark',`,
    'i18n-en'
  );
}

// default settings
rep(
  `    presenceEnabled: true,
    blockedPresence: [],
    notifications: [],
  });`,
  `    presenceEnabled: true,
    blockedPresence: [],
    notifications: [],
    notifSound: true,
    friends: [],
    friendCrate: [],
    scAccounts: [],
    sleepTracksLeft: 0,
    deepWorkUntil: 0,
    quietHours: true,
    blackLift: 0,
    hlsQuality: 'auto',
    dawnHour: -1,
    dawnQueue: [],
    chapters: {},
    playbackQueue: null,
    undoStack: [],
    errorQuarantine: [],
    candleSleep: false,
  });`,
  'defaults'
);

// PresenceUserMenu + friends
rep(
  `function PresenceUserMenu({ menu, onClose, onOpenProfile, onInvite, onBlock }) {`,
  `function PresenceUserMenu({ menu, onClose, onOpenProfile, onInvite, onBlock, onToggleFriend, isFriend }) {`,
  'menu-sig'
);

rep(
  `      <Item label={t('presence_open_profile')} disabled={!profile} onClick={onOpenProfile}/>
      <Item label={t('presence_invite')} onClick={onInvite}/>
      <Item label={t('presence_block')} onClick={onBlock}/>`,
  `      <Item label={t('presence_open_profile')} disabled={!profile} onClick={onOpenProfile}/>
      <Item label={isFriend ? t('remove_friend') : t('add_friend')} onClick={onToggleFriend}/>
      <Item label={t('presence_invite')} onClick={onInvite}/>
      <Item label={t('crate_add')} onClick={(p) => { try { window.__aoiAddCrate?.(p); } catch(_){} }}/>
      <Item label={t('presence_block')} onClick={onBlock}/>`,
  'menu-items'
);

// OnlinePanel friends split — replace visible list building
rep(
  `function OnlinePanel({ open, onClose, peers, selfToken, blocked, onUserMenu, presenceEnabled, onToggleVisible }) {
  const t = useLang();
  useEffect(() => {
    if (!open) return;
    const onKey = (e) => { if (e.key === 'Escape') onClose(); };
    document.addEventListener('keydown', onKey);
    return () => document.removeEventListener('keydown', onKey);
  }, [open, onClose]);
  if (!open) return null;
  const visible = (peers || []).filter(p => !isPresenceBlocked(p, blocked));`,
  `function OnlinePanel({ open, onClose, peers, selfToken, blocked, onUserMenu, presenceEnabled, onToggleVisible, friends, friendCrate, onPlayCrate }) {
  const t = useLang();
  useEffect(() => {
    if (!open) return;
    const onKey = (e) => { if (e.key === 'Escape') onClose(); };
    document.addEventListener('keydown', onKey);
    return () => document.removeEventListener('keydown', onKey);
  }, [open, onClose]);
  if (!open) return null;
  const visible = (peers || []).filter(p => !isPresenceBlocked(p, blocked));
  const friendKeys = new Set((friends || []).map(f => window.AoiNight?.peerKey?.(f) || ''));
  const friendPeers = visible.filter(p => friendKeys.has(window.AoiNight?.peerKey?.(p) || ''));
  const otherPeers = visible.filter(p => !friendKeys.has(window.AoiNight?.peerKey?.(p) || ''));`,
  'online-panel-sig'
);

// Inject friend sections before peer map - find the map of visible
if (had('{(visible || []).map') || had('{visible.map')) {
  // try common patterns
  const m1 = `{visible.map(peer =>`;
  const m2 = `{(visible || []).map(peer =>`;
  if (html.includes(m1) || html.includes('{visible.map(')) {
    // read a chunk - use a safer approach: replace first occurrence of rendering peers label
  }
}

fs.writeFileSync(path, html);
console.log('wrote', path.pathname, 'len', html.length);
