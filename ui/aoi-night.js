/* aoi-night.js — helpers for 1.1.0 overnight features (no React) */
(function () {
  'use strict';

  var duckTimer = null;
  var duckOrig = null;
  var lastNotifAt = 0;

  function clamp01(n) {
    n = Number(n);
    if (!isFinite(n)) return 0;
    return Math.max(0, Math.min(1, n));
  }

  /** Pleasant soft chime via WebAudio (no asset file). */
  function playNotifChime() {
    try {
      var Ctx = window.AudioContext || window.webkitAudioContext;
      if (!Ctx) return;
      if (!window.__aoiAudioCtx) window.__aoiAudioCtx = new Ctx();
      var ctx = window.__aoiAudioCtx;
      if (ctx.state === 'suspended') ctx.resume();
      var now = ctx.currentTime;
      var freqs = [523.25, 659.25, 783.99];
      freqs.forEach(function (f, i) {
        var o = ctx.createOscillator();
        var g = ctx.createGain();
        o.type = 'sine';
        o.frequency.value = f;
        g.gain.setValueAtTime(0.0001, now);
        g.gain.exponentialRampToValueAtTime(0.045, now + 0.02 + i * 0.04);
        g.gain.exponentialRampToValueAtTime(0.0001, now + 0.55 + i * 0.08);
        o.connect(g);
        g.connect(ctx.destination);
        o.start(now + i * 0.05);
        o.stop(now + 0.7 + i * 0.08);
      });
    } catch (e) {}
  }

  /**
   * Duck main HTMLAudioElements briefly, then restore.
   * getVolume/setVolume optional; else mutates audio.volume.
   */
  function duckMusic(opts) {
    opts = opts || {};
    var factor = opts.factor != null ? opts.factor : 0.28;
    var ms = opts.ms != null ? opts.ms : 1600;
    if (duckTimer) {
      clearTimeout(duckTimer);
      duckTimer = null;
    }
    if (typeof window.__aoiSetDuck === 'function') {
      window.__aoiSetDuck(clamp01(factor));
      duckTimer = setTimeout(function () {
        window.__aoiSetDuck(1);
        duckTimer = null;
      }, ms);
      return;
    }
    var audios = [];
    try {
      document.querySelectorAll('audio').forEach(function (a) { audios.push(a); });
    } catch (e) {}
    if (!audios.length) return;
    if (duckOrig == null) {
      duckOrig = audios.map(function (a) { return a.volume; });
    }
    audios.forEach(function (a, i) {
      var base = duckOrig[i] != null ? duckOrig[i] : a.volume;
      a.volume = clamp01(base * factor);
    });
    duckTimer = setTimeout(function () {
      audios.forEach(function (a, i) {
        if (duckOrig && duckOrig[i] != null) a.volume = duckOrig[i];
      });
      duckOrig = null;
      duckTimer = null;
    }, ms);
  }

  /** Fire on any new notification (invite/update/like result…). */
  function onNotificationEvent(settings) {
    settings = settings || {};
    if (settings.notifSound === false) return;
    var now = Date.now();
    if (now - lastNotifAt < 400) return;
    lastNotifAt = now;
    playNotifChime();
    duckMusic({ factor: 0.32, ms: 1400 });
  }

  function encodeMoment(track, sec) {
    var id = track && track.id != null ? String(track.id) : '';
    var title = encodeURIComponent((track && track.title) || '').slice(0, 80);
    var artist = encodeURIComponent((track && track.artist) || '').slice(0, 60);
    var s = Math.max(0, Math.floor(Number(sec) || 0));
    return 'aoi1:' + id + ':' + s + ':' + title + ':' + artist;
  }

  function decodeMoment(code) {
    var s = String(code || '').trim();
    if (s.indexOf('aoi1:') !== 0) return null;
    var parts = s.split(':');
    if (parts.length < 3) return null;
    return {
      id: parts[1],
      sec: parseInt(parts[2], 10) || 0,
      title: decodeURIComponent(parts[3] || ''),
      artist: decodeURIComponent(parts[4] || ''),
    };
  }

  function artistKey(track) {
    return String((track && (track.artist || track.uploaderUsername)) || '')
      .toLowerCase()
      .trim();
  }

  /** E08 — longer crossfade when same artist / similar title tokens. */
  function crossfadeSeconds(base, prev, next) {
    var b = Math.max(0, Number(base) || 0);
    if (!prev || !next || b <= 0) return b;
    var sameArtist = artistKey(prev) && artistKey(prev) === artistKey(next);
    if (sameArtist) return Math.min(12, b * 1.55 + 0.4);
    return b;
  }

  function peerKey(p) {
    if (!p) return '';
    return String(p.uid || p.token || p.profile || p.name || '');
  }

  function isFriend(friends, peer) {
    var k = peerKey(peer);
    if (!k) return false;
    return (friends || []).some(function (f) { return peerKey(f) === k; });
  }

  function toggleFriend(friends, peer) {
    friends = Array.isArray(friends) ? friends.slice() : [];
    var k = peerKey(peer);
    if (!k) return friends;
    var i = friends.findIndex(function (f) { return peerKey(f) === k; });
    if (i >= 0) {
      friends.splice(i, 1);
      return friends;
    }
    friends.unshift({
      uid: peer.uid || '',
      token: peer.token || '',
      name: peer.name || '',
      avatar: peer.avatar || '',
      profile: peer.profile || '',
    });
    return friends.slice(0, 80);
  }

  /** L60 — quiet like queue: no spinner loop; notify when done. */
  var likeQueue = [];
  var likeBusy = false;

  function enqueueLikeJob(job) {
    likeQueue.push(job);
    pumpLikeQueue();
  }

  function pumpLikeQueue() {
    if (likeBusy) return;
    var job = likeQueue.shift();
    if (!job) return;
    likeBusy = true;
    Promise.resolve()
      .then(job.run)
      .then(function (ok) {
        if (typeof job.done === 'function') job.done(!!ok);
      })
      .catch(function () {
        if (typeof job.done === 'function') job.done(false);
      })
      .then(function () {
        likeBusy = false;
        setTimeout(pumpLikeQueue, 120);
      });
  }

  function prefersReducedMotion() {
    try {
      return !!(window.matchMedia && window.matchMedia('(prefers-reduced-motion: reduce)').matches);
    } catch (e) {
      return false;
    }
  }

  function detectSilentBuffer(audio, threshold) {
    try {
      if (!audio || !isFinite(audio.duration) || audio.duration < 0.5) return false;
      // heuristic: near-zero currentTime movement while "playing" handled by caller;
      // volume/energy: use WebAudio analyser if wired — fallback duration/readyState
      if (audio.readyState < 2 && audio.networkState === 3) return true;
      return false;
    } catch (e) {
      return false;
    }
  }

  window.AoiNight = {
    playNotifChime: playNotifChime,
    duckMusic: duckMusic,
    onNotificationEvent: onNotificationEvent,
    encodeMoment: encodeMoment,
    decodeMoment: decodeMoment,
    crossfadeSeconds: crossfadeSeconds,
    peerKey: peerKey,
    isFriend: isFriend,
    toggleFriend: toggleFriend,
    enqueueLikeJob: enqueueLikeJob,
    prefersReducedMotion: prefersReducedMotion,
    detectSilentBuffer: detectSilentBuffer,
    artistKey: artistKey,
  };
})();
