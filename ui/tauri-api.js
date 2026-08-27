(function () {
  var invoke = function (cmd, args) {
    if (!window.__TAURI__?.core?.invoke) {
      return Promise.reject(new Error('Tauri API unavailable'));
    }
    return window.__TAURI__.core.invoke(cmd, args || {});
  };

  var listen = function (event, cb) {
    if (!window.__TAURI__?.event?.listen) return function () {};
    return window.__TAURI__.event.listen(event, function () { cb(); });
  };

  var toAssetUrl = function (p) {
    if (!p) return p;
    var s = String(p);
    if (/^(data:|https?:|asset:|tauri:)/i.test(s)) return s;
    var path = s.replace(/^file:\/\//i, '');
    try {
      if (window.__TAURI__?.core?.convertFileSrc) {
        return window.__TAURI__.core.convertFileSrc(path, 'media');
      }
    } catch (e) {}
    return s;
  };

  var mapCoverMap = function (obj) {
    if (!obj || typeof obj !== 'object') return obj || {};
    var out = {};
    Object.keys(obj).forEach(function (k) { out[k] = toAssetUrl(obj[k]); });
    return out;
  };

  window.electronAPI = {
    minimize: function () { return invoke('win_minimize'); },
    maximize: function () { return invoke('win_maximize'); },
    close: function () { return invoke('win_close'); },
    quit: function () { return invoke('win_quit'); },
    batteryPct: function () { return invoke('battery_pct'); },
    selectMusicFolder: function () { return invoke('select_music_folder'); },
    fileSrc: function (path) { return toAssetUrl(path); },
    scanMusicFolder: function (folder, minDuration) {
      return invoke('scan_music_folder', { folderPath: folder, minDuration: minDuration });
    },
    getCoverArt: function (filePath) {
      return invoke('get_cover_art', { filePath: filePath });
    },
    loadSettings: function () { return invoke('load_settings'); },
    saveSettings: function (data) { return invoke('save_settings', { data: data }); },
    backupSettings: function () { return invoke('backup_settings'); },
    restoreSettingsBackup: function (path) { return invoke('restore_settings_backup', { path: path }); },
    onMediaPlayPause: function (cb) { listen('media-play-pause', cb); },
    onMediaNext: function (cb) { listen('media-next', cb); },
    onMediaPrev: function (cb) { listen('media-prev', cb); },
    scLogin: function () { return invoke('sc_login'); },
    scFetch: function (url, token, clientId, method) {
      var m = method || 'GET';
      return invoke('sc_fetch', {
        url: url,
        token: token,
        clientId: clientId,
        method: m,
        httpMethod: m,
      });
    },
    scCheckCovers: function (ids) {
      return invoke('sc_check_covers', { ids: (ids || []).map(function (id) { return String(id); }) })
        .then(mapCoverMap);
    },
    scCacheCover: function (id, url) {
      return invoke('sc_cache_cover', { id: String(id), url: url }).then(function (p) {
        return p ? toAssetUrl(p) : p;
      });
    },
    scClearCoversCache: function () { return invoke('sc_clear_covers_cache'); },
    scClearLikesCache: function () { return invoke('sc_clear_likes_cache'); },
    scLoadLikesCache: function () { return invoke('sc_load_likes_cache'); },
    scSaveLikesCache: function (data) { return invoke('sc_save_likes_cache', { data: data }); },
    discordUpdate: function (data) { return invoke('discord_update', { data: data }); },
    discordClear: function () { return invoke('discord_clear'); },
    discordConnect: function (clientId) {
      return invoke('discord_connect', { clientId: clientId });
    },
    discordDisconnect: function () { return invoke('discord_disconnect'); },
    setLoginItem: function (enable) {
      return invoke('set_login_item', { enable: enable });
    },
    openMini: function () { return invoke('open_mini'); },
    miniExpand: function () { return invoke('mini_expand'); },
    miniCmd: function (action, value) {
      return invoke('mini_cmd', { action: action, value: value });
    },
    miniState: function (state) {
      return invoke('mini_state', { state: state });
    },
    miniMenu: function (open) {
      return invoke('mini_menu', { open: !!open });
    },
    miniAlwaysOnTop: function (on) {
      return invoke('mini_always_on_top', { on: !!on });
    },
    openUrl: function (url) {
      if (!url || !/^https?:\/\//i.test(String(url))) return Promise.resolve();
      try {
        if (window.__TAURI__?.core?.invoke) {
          return window.__TAURI__.core.invoke('plugin:shell|open', { path: String(url) });
        }
        if (window.__TAURI__?.shell?.open) return window.__TAURI__.shell.open(String(url));
      } catch (e) {}
      try { window.open(String(url), '_blank', 'noopener'); } catch (e2) {}
      return Promise.resolve();
    },
    appVersion: function () { return invoke('app_version'); },
    checkForUpdate: function () { return invoke('check_for_update'); },
    installUpdate: function (url, sha256) {
      return invoke('install_update', { url: url, sha256: sha256 || null });
    },
  };
})();
