(function () {
  var DEFAULT_URL = 'https://aoi-rooms.elvishedcc.workers.dev';

  function roomsUrl() {
    return String(window.AOI_ROOMS_URL || DEFAULT_URL).replace(/\/+$/, '');
  }

  function wsUrl(httpUrl) {
    return httpUrl.replace(/^http/i, 'ws');
  }

  function RoomClient() {
    this.ws = null;
    this.closed = false;
    this.handlers = [];
    this.pingTimer = null;
    this.retry = 0;
    this.opts = null;
  }

  RoomClient.prototype.on = function (fn) {
    this.handlers.push(fn);
    return function () {
      this.handlers = this.handlers.filter(function (h) { return h !== fn; });
    }.bind(this);
  };

  RoomClient.prototype.emit = function (msg) {
    this.handlers.forEach(function (fn) {
      try { fn(msg); } catch (e) {}
    });
  };

  RoomClient.prototype.send = function (msg) {
    if (!this.ws || this.ws.readyState !== 1) {
      if (msg && msg.type === 'state') this._pendingState = msg;
      if (msg && (msg.type === 'profile' || msg.type === 'name')) this._pendingProfile = msg;
      if (msg && (msg.type === 'lock' || msg.type === 'locks')) this._pendingLock = msg;
      return false;
    }
    try { this.ws.send(JSON.stringify(msg)); return true; }
    catch (e) { return false; }
  };

  RoomClient.prototype.create = function (opts) {
    opts = opts || {};
    return fetch(roomsUrl() + '/create', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        name: opts.name || 'host',
        avatar: opts.avatar || '',
        uid: opts.uid || '',
        lockedSlots: opts.lockedSlots || [],
      }),
    }).then(function (res) { return res.json(); });
  };

  RoomClient.prototype.connect = function (opts) {
    this.leave();
    this.closed = false;
    this.opts = opts || {};
    this.retry = 0;
    this._open();
  };

  RoomClient.prototype._dropSocket = function (ws) {
    if (!ws) return;
    try {
      ws.onopen = null;
      ws.onmessage = null;
      ws.onerror = null;
      ws.onclose = null;
      ws.close();
    } catch (e) {}
  };

  RoomClient.prototype._open = function () {
    var self = this;
    if (this.closed || !this.opts || !this.opts.code) return;
    this._gen = (this._gen || 0) + 1;
    var gen = this._gen;
    if (this.ws) {
      this._dropSocket(this.ws);
      this.ws = null;
    }
    var code = String(this.opts.code).toUpperCase().replace(/[^A-Z0-9]/g, '').slice(0, 6);
    var q = new URLSearchParams();
    if (this.opts.name) q.set('name', this.opts.name);
    if (this.opts.token) q.set('token', this.opts.token);
    if (this.opts.avatar) q.set('avatar', this.opts.avatar);
    if (this.opts.uid) q.set('uid', this.opts.uid);
    var url = wsUrl(roomsUrl()) + '/room/' + code + '?' + q.toString();
    var ws;
    try { ws = new WebSocket(url); }
    catch (e) {
      this.emit({ type: 'error', error: 'connect_failed' });
      return;
    }
    this.ws = ws;
    var opened = false;
    ws.onopen = function () {
      if (gen !== self._gen || self.ws !== ws) return;
      opened = true;
      self.retry = 0;
      self._armPing();
      if (self.opts && self.opts.name && self.opts.name !== 'friend') {
        var profile = { type: 'profile', name: self.opts.name, uid: self.opts.uid || '' };
        if (self.opts.avatar) profile.avatar = self.opts.avatar;
        self.send(profile);
      }
      if (self._pendingProfile) {
        var pendingProfile = self._pendingProfile;
        self._pendingProfile = null;
        self.send(pendingProfile);
      }
      if (self._pendingState) {
        var pending = self._pendingState;
        self._pendingState = null;
        self.send(pending);
      }
      if (self._pendingLock) {
        var pendingLock = self._pendingLock;
        self._pendingLock = null;
        self.send(pendingLock);
      }
      self.emit({ type: 'open' });
    };
    ws.onmessage = function (ev) {
      if (gen !== self._gen || self.ws !== ws) return;
      var msg;
      try { msg = JSON.parse(ev.data); } catch (e) { return; }
      self.emit(msg);
    };
    ws.onclose = function (ev) {
      if (gen !== self._gen || self.ws !== ws) return;
      self._clearPing();
      if (ev && ev.code === 4000) {
        self.closed = true;
        self.emit({ type: 'replaced' });
        return;
      }
      if (ev && ev.code === 4001) {
        self.closed = true;
        self.emit({ type: 'kicked' });
        return;
      }
      if (ev && ev.code === 4002) {
        self.closed = true;
        self.emit({ type: 'banned' });
        return;
      }
      if (self.closed) {
        self.emit({ type: 'closed' });
        return;
      }
      if (!opened && self.retry >= 3) {
        self.closed = true;
        self.emit({ type: 'error', error: 'no_room' });
        return;
      }
      var wait = Math.min(8000, 600 * Math.pow(2, self.retry++));
      setTimeout(function () {
        if (gen !== self._gen || self.closed) return;
        self._open();
      }, wait);
      self.emit({ type: 'reconnecting' });
    };
    ws.onerror = function () {
      if (gen !== self._gen || self.ws !== ws) return;
      try { ws.close(); } catch (e) {}
    };
  };

  RoomClient.prototype._armPing = function () {
    var self = this;
    this._clearPing();
    this.pingTimer = setInterval(function () { self.send({ type: 'ping' }); }, 25000);
  };

  RoomClient.prototype._clearPing = function () {
    if (this.pingTimer) { clearInterval(this.pingTimer); this.pingTimer = null; }
  };

  RoomClient.prototype.sendState = function (state) {
    var msg = {
      type: 'state',
      track: state.track,
      isPlaying: state.isPlaying,
      progress: state.progress,
      duration: state.duration,
    };
    var playChanged = this._lastPlay !== state.isPlaying;
    this._lastPlay = state.isPlaying;
    this._wantState = msg;
    if (playChanged) {
      this._flushState();
      return true;
    }
    if (this._stateFlush) return true;
    var self = this;
    this._stateFlush = setTimeout(function () { self._flushState(); }, 400);
    return true;
  };

  RoomClient.prototype._flushState = function () {
    if (this._stateFlush) {
      clearTimeout(this._stateFlush);
      this._stateFlush = null;
    }
    var msg = this._wantState;
    this._wantState = null;
    if (msg) this.send(msg);
  };

  RoomClient.prototype.leave = function () {
    this.closed = true;
    this._gen = (this._gen || 0) + 1;
    this._pendingState = null;
    this._pendingProfile = null;
    this._pendingLock = null;
    this._wantState = null;
    if (this._stateFlush) {
      clearTimeout(this._stateFlush);
      this._stateFlush = null;
    }
    this._clearPing();
    this._dropSocket(this.ws);
    this.ws = null;
  };

  window.AoiRooms = new RoomClient();

  function presenceToken() {
    var key = 'aoiPresenceTok';
    try {
      var tok = localStorage.getItem(key);
      if (tok) return tok;
      tok = Math.random().toString(36).slice(2, 10) + Date.now().toString(36);
      localStorage.setItem(key, tok);
      return tok;
    } catch (e) {
      return Math.random().toString(36).slice(2, 14);
    }
  }

  window.AoiRooms.presenceToken = presenceToken;

  window.AoiRooms.presenceBeat = function (payload) {
    var tok = presenceToken();
    var body = Object.assign({ token: tok }, payload || {});
    return fetch(roomsUrl() + '/presence/beat', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(body),
    }).then(function (r) {
      if (!r.ok) return { ok: false, n: 0 };
      return r.json();
    }).catch(function () { return { ok: false, n: 0 }; });
  };

  window.AoiRooms.presenceLeave = function () {
    var tok = presenceToken();
    return fetch(roomsUrl() + '/presence/leave', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ token: tok }),
    }).then(function (r) {
      if (!r.ok) return { ok: false };
      return r.json();
    }).catch(function () { return { ok: false }; });
  };

  window.AoiRooms.presenceInvite = function (opts) {
    opts = opts || {};
    return fetch(roomsUrl() + '/presence/invite', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        fromToken: presenceToken(),
        toToken: opts.toToken || '',
        toUid: opts.toUid || '',
        roomCode: opts.roomCode || '',
        fromName: opts.fromName || '',
        fromAvatar: opts.fromAvatar || '',
      }),
    }).then(function (r) {
      if (!r.ok) return { ok: false };
      return r.json();
    }).catch(function () { return { ok: false }; });
  };

  window.AoiRooms.presenceList = function () {
    return fetch(roomsUrl() + '/presence/list')
      .then(function (r) {
        if (!r.ok) return { ok: false, n: 0, peers: [] };
        return r.json();
      })
      .catch(function () { return { ok: false, n: 0, peers: [] }; });
  };

  window.AoiRooms.pulse = function (id, token) {
    var trackId = String(id || '').replace(/[^\w-]/g, '').slice(0, 64);
    var tok = String(token || '').replace(/[^\w-]/g, '').slice(0, 32);
    if (!trackId || !tok) return Promise.resolve(0);
    return fetch(roomsUrl() + '/pulse?id=' + encodeURIComponent(trackId), {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ token: tok }),
    }).then(function (r) {
      if (!r.ok) return 0;
      return r.json().then(function (j) { return Number(j && j.n) || 0; });
    }).catch(function () { return 0; });
  };
})();
