(function () {
  var DEFAULT_UI_SHOW = {
    navHome: true,
    navPlaylists: true,
    navSearch: true,
    navPlayer: true,
    navRooms: true,
    navLocal: true,
    navSettings: true,
    sidebarAvatar: true,
    queuePanel: true,
    playerGlow: true,
    playerProgress: true,
    playerVolume: true,
    playerEq: true,
    playerShuffle: true,
    playerRepeat: true,
  };

  var SC_MOODS = [
    { id: 'popular', labelRu: 'популярное', labelEn: 'popular', q: 'trending music' },
    { id: 'sad', labelRu: 'грустное', labelEn: 'sad', q: 'sad ambient' },
    { id: 'happy', labelRu: 'весёлое', labelEn: 'happy', q: 'happy upbeat' },
    { id: 'energy', labelRu: 'энергичное', labelEn: 'energetic', q: 'energetic electronic' },
    { id: 'new', labelRu: 'новое', labelEn: 'new', q: 'new releases' },
    { id: 'chill', labelRu: 'чилл', labelEn: 'chill', q: 'chill lofi' },
    { id: 'night', labelRu: 'ночное', labelEn: 'night', q: 'night drive' },
    { id: 'focus', labelRu: 'фокус', labelEn: 'focus', q: 'focus instrumental' },
  ];

  function defaultTheme() {
    return {
      navDock: 'left',
      libWidth: 260,
      libPinned: true,
      playerBg: '',
      playerBgType: 'none',
      playerBgImage: '',
      playerBgBlur: 24,
      playerBgDim: 45,
      playerBgScope: 'window',
      playerBgLibAlpha: 35,
      playerBgLibDim: 15,
      navDockAlpha: 100,
      navDockDim: 0,
      accentMode: 'default',
      uiShow: Object.assign({}, DEFAULT_UI_SHOW),
    };
  }

  function mergeUiShow(raw) {
    var out = Object.assign({}, DEFAULT_UI_SHOW);
    if (!raw || typeof raw !== 'object') return out;
    Object.keys(DEFAULT_UI_SHOW).forEach(function (k) {
      if (typeof raw[k] === 'boolean') out[k] = raw[k];
    });
    return out;
  }

  function clampPct(n, fallback) {
    return Math.max(0, Math.min(100, Number(n) || fallback));
  }

  function themeFromSettings(s) {
    s = s || {};
    var base = defaultTheme();
    var t = s.theme && typeof s.theme === 'object' ? s.theme : {};
    return {
      navDock: ['left', 'right', 'top', 'bottom'].indexOf(t.navDock || s.navDock) >= 0
        ? ((t.navDock || s.navDock) === 'bottom' ? 'top' : (t.navDock || s.navDock)) : base.navDock,
      libWidth: Math.max(180, Math.min(520, Number(t.libWidth ?? s.libWidth) || base.libWidth)),
      libPinned: t.libPinned != null ? !!t.libPinned : (s.libPinned != null ? !!s.libPinned : base.libPinned),
      playerBg: String(t.playerBg || s.playerBg || '').slice(0, 80),
      playerBgType: ['none', 'image'].indexOf(t.playerBgType || s.playerBgType) >= 0
        ? (t.playerBgType || s.playerBgType) : base.playerBgType,
      playerBgImage: String(t.playerBgImage || s.playerBgImage || '').slice(0, 512),
      playerBgBlur: clampPct(t.playerBgBlur ?? s.playerBgBlur, base.playerBgBlur),
      playerBgDim: clampPct(t.playerBgDim ?? s.playerBgDim, base.playerBgDim),
      playerBgScope: ['window', 'player'].indexOf(t.playerBgScope || s.playerBgScope) >= 0
        ? (t.playerBgScope || s.playerBgScope) : base.playerBgScope,
      playerBgLibAlpha: clampPct(t.playerBgLibAlpha ?? s.playerBgLibAlpha, base.playerBgLibAlpha),
      playerBgLibDim: clampPct(t.playerBgLibDim ?? s.playerBgLibDim, base.playerBgLibDim),
      navDockAlpha: clampPct(t.navDockAlpha ?? s.navDockAlpha, base.navDockAlpha),
      navDockDim: clampPct(t.navDockDim ?? s.navDockDim, base.navDockDim),
      accentMode: String(t.accentMode || s.accentMode || base.accentMode),
      uiShow: mergeUiShow(t.uiShow || s.uiShow),
    };
  }

  function exportThemeCode(settings) {
    var pack = themeFromSettings(settings || {});
    try {
      var json = JSON.stringify(pack);
      var b64 = btoa(unescape(encodeURIComponent(json)));
      return 'AOI1-' + b64.replace(/\+/g, '-').replace(/\//g, '_').replace(/=+$/g, '');
    } catch (e) {
      return '';
    }
  }

  function importThemeCode(code) {
    var raw = String(code || '').trim();
    if (!raw) return null;
    if (raw.indexOf('AOI1-') === 0) raw = raw.slice(5);
    raw = raw.replace(/-/g, '+').replace(/_/g, '/');
    while (raw.length % 4) raw += '=';
    try {
      var json = decodeURIComponent(escape(atob(raw)));
      var data = JSON.parse(json);
      return themeFromSettings(data);
    } catch (e) {
      return null;
    }
  }

  window.AoiCustom = {
    DEFAULT_UI_SHOW: DEFAULT_UI_SHOW,
    SC_MOODS: SC_MOODS,
    defaultTheme: defaultTheme,
    mergeUiShow: mergeUiShow,
    themeFromSettings: themeFromSettings,
    exportThemeCode: exportThemeCode,
    importThemeCode: importThemeCode,
  };
})();
