const CODE_ABC = 'ABCDEFGHJKLMNPQRSTUVWXYZ23456789';
const ROOM_IDLE_MS = 6 * 60 * 60 * 1000;
const SLOT_COUNT = 8;
const MAX_SUGGEST = 30;

function cors() {
  return {
    'Access-Control-Allow-Origin': '*',
    'Access-Control-Allow-Methods': 'GET, POST, OPTIONS',
    'Access-Control-Allow-Headers': 'Content-Type',
  };
}

function json(data, status = 200) {
  return new Response(JSON.stringify(data), {
    status,
    headers: { 'Content-Type': 'application/json', ...cors() },
  });
}

function randomCode(len = 6) {
  const bytes = crypto.getRandomValues(new Uint8Array(len));
  let out = '';
  for (let i = 0; i < len; i++) out += CODE_ABC[bytes[i] % CODE_ABC.length];
  return out;
}

function randomToken() {
  const bytes = crypto.getRandomValues(new Uint8Array(18));
  let bin = '';
  for (const b of bytes) bin += String.fromCharCode(b);
  return btoa(bin).replace(/\+/g, '-').replace(/\//g, '_').replace(/=+$/g, '');
}

function clampName(raw) {
  const s = String(raw || '').replace(/\s+/g, ' ').trim().slice(0, 24);
  return s.length >= 1 ? s : 'friend';
}

function clampAvatar(raw) {
  const s = String(raw || '').trim().slice(0, 400);
  if (!s.startsWith('https://')) return '';
  const lower = s.toLowerCase();
  if (lower.includes('localhost') || lower.includes('127.0.0.1')) return '';
  return s;
}

function clampUid(raw) {
  return String(raw || '').replace(/[^\w-]/g, '').slice(0, 32);
}

function rateBlocked(att, key, ms) {
  const now = Date.now();
  const rl = att.rl || {};
  if ((rl[key] || 0) > now) return true;
  rl[key] = now + ms;
  att.rl = rl;
  return false;
}

function memberKey(m) {
  if (!m) return '';
  if (m.uid) return 'u:' + String(m.uid);
  const name = String(m.name || '').trim().toLowerCase();
  if (name && name !== 'friend') return 'n:' + name;
  if (m.id) return 'i:' + String(m.id);
  return '';
}

function samePerson(a, b) {
  if (!a || !b) return false;
  if (a.uid && b.uid && String(a.uid) === String(b.uid)) return true;
  const an = String(a.name || '').trim().toLowerCase();
  const bn = String(b.name || '').trim().toLowerCase();
  return !!(an && bn && an !== 'friend' && an === bn);
}

function burstBlocked(att) {
  const now = Date.now();
  const hits = (att.hits || []).filter((t) => now - t < 2000);
  hits.push(now);
  att.hits = hits;
  return hits.length > 10;
}

function normalizeLocks(raw) {
  const set = new Set();
  if (Array.isArray(raw)) {
    for (const n of raw) {
      const i = Number(n);
      if (Number.isInteger(i) && i >= 0 && i < SLOT_COUNT) set.add(i);
    }
  }
  if (set.size >= SLOT_COUNT) set.delete(0);
  return [...set].sort((a, b) => a - b);
}

export default {
  async fetch(req, env) {
    if (req.method === 'OPTIONS') return new Response(null, { headers: cors() });

    const url = new URL(req.url);
    if (url.pathname === '/' || url.pathname === '/health') {
      return json({ ok: true, service: 'aoi-rooms', v: 6 });
    }

    if (url.pathname === '/update/latest') {
      const stub = env.PRESENCE.get(env.PRESENCE.idFromName('global'));
      return stub.fetch(new Request('https://presence/update', { method: 'GET' }));
    }
    if (url.pathname === '/update/set' && req.method === 'POST') {
      const stub = env.PRESENCE.get(env.PRESENCE.idFromName('global'));
      return stub.fetch(new Request('https://presence/update-set', {
        method: 'POST',
        body: req.body,
        headers: req.headers,
      }));
    }

    if (url.pathname === '/presence/beat' && req.method === 'POST') {
      const stub = env.PRESENCE.get(env.PRESENCE.idFromName('global'));
      return stub.fetch(new Request('https://presence/beat', { method: 'POST', body: req.body, headers: req.headers }));
    }
    if (url.pathname === '/presence/leave' && req.method === 'POST') {
      const stub = env.PRESENCE.get(env.PRESENCE.idFromName('global'));
      return stub.fetch(new Request('https://presence/leave', { method: 'POST', body: req.body, headers: req.headers }));
    }
    if (url.pathname === '/presence/invite' && req.method === 'POST') {
      const stub = env.PRESENCE.get(env.PRESENCE.idFromName('global'));
      return stub.fetch(new Request('https://presence/invite', { method: 'POST', body: req.body, headers: req.headers }));
    }
    if (url.pathname === '/presence/list') {
      const stub = env.PRESENCE.get(env.PRESENCE.idFromName('global'));
      return stub.fetch(new Request('https://presence/list', { method: 'GET' }));
    }
    if (url.pathname === '/presence/ws') {
      const stub = env.PRESENCE.get(env.PRESENCE.idFromName('global'));
      return stub.fetch(req);
    }
    if (url.pathname === '/presence/crate/push' && req.method === 'POST') {
      const stub = env.PRESENCE.get(env.PRESENCE.idFromName('global'));
      return stub.fetch(new Request('https://presence/crate/push', { method: 'POST', body: req.body, headers: req.headers }));
    }

    if (url.pathname === '/sc/proxy') {
      const target = url.searchParams.get('url') || '';
      const okProxy = /^https:\/\/(api-v2\.soundcloud\.com|api\.soundcloud\.com|sndcdn\.com|[\w.-]+\.sndcdn\.com)\//.test(target);
      if (!okProxy) {
        return json({ error: 'bad_url' }, 400);
      }
      try {
        const fwd = new Headers();
        const auth = req.headers.get('Authorization');
        if (auth) fwd.set('Authorization', auth);
        const range = req.headers.get('Range');
        if (range) fwd.set('Range', range);
        const isApi = target.includes('soundcloud.com/');
        fwd.set('Accept', isApi ? 'application/json; charset=utf-8' : '*/*');
        fwd.set('Origin', 'https://soundcloud.com');
        fwd.set('Referer', 'https://soundcloud.com/');
        fwd.set('User-Agent', 'Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/122.0.0.0 Safari/537.36');
        const dd = req.headers.get('x-datadome-clientid');
        if (dd) fwd.set('x-datadome-clientid', dd);
        const cookie = req.headers.get('Cookie');
        if (cookie) fwd.set('Cookie', cookie);
        const method = req.method === 'POST' ? 'POST' : req.method === 'PUT' ? 'PUT' : req.method === 'DELETE' ? 'DELETE' : 'GET';
        const resp = await fetch(target, {
          method,
          headers: fwd,
          body: method === 'GET' || method === 'DELETE' ? undefined : (req.body || '{}'),
        });
        const ct = resp.headers.get('content-type') || (isApi ? 'application/json; charset=utf-8' : 'application/octet-stream');
        const outHdrs = { 'Content-Type': ct, ...cors() };
        for (const h of ['Content-Length', 'Content-Range', 'Accept-Ranges', 'Cache-Control']) {
          const v = resp.headers.get(h);
          if (v) outHdrs[h] = v;
        }
        if (!isApi && resp.body) {
          return new Response(resp.body, { status: resp.status, headers: outHdrs });
        }
        const text = await resp.text();
        return new Response(text, { status: resp.status, headers: outHdrs });
      } catch (e) {
        return json({ error: 'proxy_fail' }, 502);
      }
    }

    if (url.pathname === '/create' && req.method === 'POST') {
      let name = 'host';
      let avatar = '';
      let uid = '';
      let lockedSlots = [];
      try {
        const body = await req.json();
        if (body && body.name) name = clampName(body.name);
        if (body && body.avatar) avatar = clampAvatar(body.avatar);
        if (body && body.uid) uid = clampUid(body.uid);
        lockedSlots = normalizeLocks(body && body.lockedSlots);
      } catch {}
      for (let i = 0; i < 10; i++) {
        const code = randomCode();
        const token = randomToken();
        const stub = env.ROOM.get(env.ROOM.idFromName(code));
        const res = await stub.fetch(new Request('https://room/init', {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({ code, token, name, avatar, uid, lockedSlots }),
        }));
        if (res.ok) {
          const data = await res.json();
          return json({ ok: true, ...data });
        }
      }
      return json({ ok: false, error: 'busy' }, 503);
    }

    if (url.pathname === '/pulse') {
      const id = String(url.searchParams.get('id') || '').replace(/[^\w-]/g, '').slice(0, 64);
      if (!id) return json({ n: 0 });
      const cache = caches.default;
      const key = new Request('https://aoi.pulse/v1/' + id);
      const now = Date.now();
      let tokens = [];
      try {
        const hit = await cache.match(key);
        if (hit) {
          const body = await hit.json();
          tokens = Array.isArray(body.tokens) ? body.tokens : [];
        }
      } catch {}
      tokens = tokens.filter((t) => t && t.exp > now && typeof t.tok === 'string').slice(0, 40);
      if (req.method === 'POST') {
        let tok = '';
        try {
          const body = await req.json();
          tok = String((body && body.token) || '').replace(/[^\w-]/g, '').slice(0, 32);
        } catch {}
        if (tok) {
          tokens = tokens.filter((t) => t.tok !== tok);
          tokens.push({ tok, exp: now + 70000 });
        }
        try {
          await cache.put(
            key,
            new Response(JSON.stringify({ tokens }), {
              headers: { 'Content-Type': 'application/json', 'Cache-Control': 'max-age=80' },
            }),
          );
        } catch {}
      }
      return json({ n: tokens.length });
    }

    const m = url.pathname.match(/^\/room\/([A-Z0-9]{6})$/i);
    if (m) {
      const code = m[1].toUpperCase();
      const stub = env.ROOM.get(env.ROOM.idFromName(code));
      return stub.fetch(req);
    }

    return json({ ok: false, error: 'not_found' }, 404);
  },
};

export class Room {
  constructor(ctx) {
    this.ctx = ctx;
  }

  async fetch(req) {
    const url = new URL(req.url);

    if (url.pathname === '/init' && req.method === 'POST') {
      const body = await req.json();
      const existing = await this.ctx.storage.get(['hostToken', 'lastSeen']);
      const hostToken = existing.get('hostToken');
      const lastSeen = existing.get('lastSeen') || 0;
      const live = this.ctx.getWebSockets().length > 0;
      if (hostToken && live) {
        return json({ ok: false, error: 'taken' }, 409);
      }
      if (hostToken && Date.now() - lastSeen < ROOM_IDLE_MS && !live) {
        return json({ ok: false, error: 'taken' }, 409);
      }
      await this.ctx.storage.put({
        code: body.code,
        hostToken: body.token,
        hostName: clampName(body.name),
        playback: null,
        lastSeen: Date.now(),
        lockedSlots: normalizeLocks(body.lockedSlots),
        banned: [],
        suggestions: [],
      });
      await this.ctx.storage.setAlarm(Date.now() + ROOM_IDLE_MS);
      return json({ ok: true, code: body.code, token: body.token });
    }

    const upgrade = req.headers.get('Upgrade') || '';
    if (upgrade.toLowerCase() !== 'websocket') {
      const code = (await this.ctx.storage.get('code')) || '';
      const members = this.listMembers();
      return json({ ok: true, code, members: members.length, live: members.length > 0 });
    }

    const url2 = new URL(req.url);
    const token = url2.searchParams.get('token') || '';
    const name = clampName(url2.searchParams.get('name'));
    const avatar = clampAvatar(url2.searchParams.get('avatar'));
    const uid = clampUid(url2.searchParams.get('uid'));
    const storedToken = await this.ctx.storage.get('hostToken');
    if (!storedToken) {
      return json({ ok: false, error: 'no_room' }, 404);
    }

    const meta = await this.loadMeta();
    const isHost = token && token === storedToken;
    if (!isHost && this.isBanned(meta.banned, uid, name)) {
      return json({ ok: false, error: 'banned' }, 403);
    }

    const incoming = { name, avatar, uid };
    let slot = -1;
    const doomed = [];
    for (const other of this.ctx.getWebSockets()) {
      const a = other.deserializeAttachment() || {};
      if ((isHost && a.role === 'host') || samePerson(a, incoming)) {
        if (a.slot != null && slot < 0) slot = a.slot;
        doomed.push(other);
      }
    }
    if (slot < 0) slot = this.firstFreeSlot(meta.lockedSlots, doomed);
    if (slot < 0 && isHost) {
      const unlocked = meta.lockedSlots.filter((i) => i !== 0);
      await this.ctx.storage.put({ lockedSlots: unlocked });
      meta.lockedSlots = unlocked;
      slot = this.firstFreeSlot(unlocked, doomed);
    }
    if (slot < 0) {
      return json({ ok: false, error: 'full' }, 423);
    }

    const pair = new WebSocketPair();
    const [client, server] = Object.values(pair);
    this.ctx.acceptWebSocket(server);
    const sid = randomToken();
    server.serializeAttachment({
      id: sid,
      name,
      avatar,
      uid,
      slot,
      role: isHost ? 'host' : 'guest',
    });

    for (const other of doomed) {
      if (other === server) continue;
      try { other.close(4000, 'replaced'); } catch {}
    }

    const playback = (await this.ctx.storage.get('playback')) || null;
    const code = (await this.ctx.storage.get('code')) || '';
    const snap = await this.snapshot(doomed);
    this.send(server, {
      type: 'hello',
      code,
      you: sid,
      role: isHost ? 'host' : 'guest',
      members: snap.members,
      slots: snap.slots,
      suggestions: snap.suggestions,
      state: playback,
    });
    this.broadcast({ type: 'members', members: snap.members, slots: snap.slots }, null);
    await this.ctx.storage.put('lastSeen', Date.now());
    await this.ctx.storage.setAlarm(Date.now() + ROOM_IDLE_MS);

    return new Response(null, { status: 101, webSocket: client });
  }

  async loadMeta() {
    const m = await this.ctx.storage.get(['lockedSlots', 'banned', 'suggestions']);
    return {
      lockedSlots: normalizeLocks(m.get('lockedSlots')),
      banned: Array.isArray(m.get('banned')) ? m.get('banned') : [],
      suggestions: Array.isArray(m.get('suggestions')) ? m.get('suggestions') : [],
    };
  }

  isBanned(banned, uid, name) {
    if (!Array.isArray(banned)) return false;
    if (uid && banned.includes('u:' + uid)) return true;
    if (name && banned.includes('n:' + name.toLowerCase())) return true;
    return false;
  }

  skipSet(except) {
    if (!except) return null;
    return except instanceof Set ? except : new Set(Array.isArray(except) ? except : [except]);
  }

  firstFreeSlot(lockedSlots, except) {
    const skip = this.skipSet(except);
    const used = new Set();
    for (const ws of this.ctx.getWebSockets()) {
      if (skip && skip.has(ws)) continue;
      const a = ws.deserializeAttachment() || {};
      if (a.slot != null) used.add(a.slot);
    }
    for (let i = 0; i < SLOT_COUNT; i++) {
      if (!lockedSlots.includes(i) && !used.has(i)) return i;
    }
    return -1;
  }

  listMembers(except) {
    const skip = this.skipSet(except);
    const raw = [];
    for (const ws of this.ctx.getWebSockets()) {
      if (skip && skip.has(ws)) continue;
      const a = ws.deserializeAttachment() || {};
      raw.push({
        id: a.id,
        name: a.name || 'friend',
        avatar: a.avatar || '',
        uid: a.uid || '',
        slot: a.slot != null ? a.slot : null,
        role: a.role || 'guest',
      });
    }
    const map = new Map();
    for (const m of raw) {
      const key = memberKey(m) || ('i:' + String(m.id || ''));
      map.set(key, m);
    }
    return [...map.values()];
  }

  async snapshot(except) {
    const meta = await this.loadMeta();
    const members = this.listMembers(except);
    const bySlot = new Map(members.filter((m) => m.slot != null).map((m) => [m.slot, m]));
    const slots = [];
    for (let i = 0; i < SLOT_COUNT; i++) {
      slots.push({
        i,
        locked: meta.lockedSlots.includes(i),
        member: bySlot.get(i) || null,
      });
    }
    return { members, slots, suggestions: meta.suggestions };
  }

  send(ws, msg) {
    try { ws.send(JSON.stringify(msg)); } catch {}
  }

  broadcast(msg, except) {
    const raw = JSON.stringify(msg);
    for (const ws of this.ctx.getWebSockets()) {
      if (except && ws === except) continue;
      try { ws.send(raw); } catch {}
    }
  }

  findSocket(id) {
    for (const ws of this.ctx.getWebSockets()) {
      const a = ws.deserializeAttachment() || {};
      if (a.id === id) return ws;
    }
    return null;
  }

  async pushRoster() {
    const snap = await this.snapshot();
    this.broadcast({ type: 'members', members: snap.members, slots: snap.slots }, null);
  }

  async webSocketMessage(ws, raw) {
    let msg;
    try { msg = JSON.parse(typeof raw === 'string' ? raw : new TextDecoder().decode(raw)); }
    catch { return; }
    const att = ws.deserializeAttachment() || {};
    if (msg.type === 'ping') {
      this.send(ws, { type: 'pong', t: Date.now() });
      return;
    }
    if (burstBlocked(att)) {
      ws.serializeAttachment(att);
      return;
    }
    if (msg.type === 'state' && att.role === 'host') {
      if (rateBlocked(att, 'state', 350)) {
        ws.serializeAttachment(att);
        return;
      }
      ws.serializeAttachment(att);
      const state = sanitizeState(msg);
      await this.ctx.storage.put({ playback: state, lastSeen: Date.now() });
      this.broadcast({ type: 'state', ...state }, ws);
      return;
    }
    if (msg.type === 'profile' || msg.type === 'name') {
      if (rateBlocked(att, 'profile', 400)) {
        ws.serializeAttachment(att);
        return;
      }
      if (msg.name) {
        const next = clampName(msg.name);
        if (next !== 'friend' || !att.name || att.name === 'friend') att.name = next;
      }
      if (msg.avatar != null) {
        const nextAvatar = clampAvatar(msg.avatar);
        if (nextAvatar) att.avatar = nextAvatar;
      }
      if (msg.uid) att.uid = clampUid(msg.uid);
      ws.serializeAttachment(att);
      await this.pushRoster();
      return;
    }
    if ((msg.type === 'lock' || msg.type === 'locks') && att.role === 'host') {
      if (rateBlocked(att, 'lock', 400)) {
        ws.serializeAttachment(att);
        return;
      }
      ws.serializeAttachment(att);
      let next;
      if (msg.type === 'locks') {
        const raw = Array.isArray(msg.lockedSlots)
          ? msg.lockedSlots
          : (Array.isArray(msg.locked) ? msg.locked.map((v, i) => (v ? i : -1)).filter((i) => i >= 0) : []);
        next = normalizeLocks(raw);
      } else {
        const i = Number(msg.slot);
        if (!Number.isInteger(i) || i < 0 || i >= SLOT_COUNT) return;
        const locked = new Set((await this.loadMeta()).lockedSlots);
        if (msg.locked === false) locked.delete(i);
        else locked.add(i);
        next = normalizeLocks([...locked]);
      }
      await this.ctx.storage.put({ lockedSlots: next, lastSeen: Date.now() });
      await this.pushRoster();
      return;
    }
    if (msg.type === 'kick' && att.role === 'host') {
      if (rateBlocked(att, 'mod', 1000)) {
        ws.serializeAttachment(att);
        return;
      }
      ws.serializeAttachment(att);
      const target = this.findSocket(String(msg.id || ''));
      if (!target || target === ws) return;
      const ta = target.deserializeAttachment() || {};
      if (ta.role === 'host') return;
      try { target.close(4001, 'kicked'); } catch {}
      return;
    }
    if (msg.type === 'ban' && att.role === 'host') {
      if (rateBlocked(att, 'mod', 1200)) {
        ws.serializeAttachment(att);
        return;
      }
      ws.serializeAttachment(att);
      const target = this.findSocket(String(msg.id || ''));
      if (!target || target === ws) return;
      const ta = target.deserializeAttachment() || {};
      if (ta.role === 'host') return;
      const meta = await this.loadMeta();
      const banned = meta.banned.slice();
      if (ta.uid) banned.push('u:' + ta.uid);
      if (ta.name) banned.push('n:' + String(ta.name).toLowerCase());
      await this.ctx.storage.put({ banned: [...new Set(banned)], lastSeen: Date.now() });
      try { target.close(4002, 'banned'); } catch {}
      return;
    }
    if (msg.type === 'transfer' && att.role === 'host') {
      if (rateBlocked(att, 'mod', 2000)) {
        ws.serializeAttachment(att);
        return;
      }
      const target = this.findSocket(String(msg.id || ''));
      if (!target || target === ws) return;
      const ta = target.deserializeAttachment() || {};
      if (ta.role === 'host') return;
      const newToken = randomToken();
      att.role = 'guest';
      ta.role = 'host';
      ws.serializeAttachment(att);
      target.serializeAttachment(ta);
      await this.ctx.storage.put({
        hostToken: newToken,
        hostName: clampName(ta.name),
        lastSeen: Date.now(),
      });
      this.send(target, { type: 'promoted', token: newToken });
      this.send(ws, { type: 'demoted' });
      await this.pushRoster();
      return;
    }
    if (msg.type === 'suggest' && att.role === 'guest') {
      if (rateBlocked(att, 'suggest', 2500)) {
        ws.serializeAttachment(att);
        return;
      }
      ws.serializeAttachment(att);
      const track = sanitizeTrack(msg.track);
      if (!track || !track.title) return;
      const meta = await this.loadMeta();
      const key = String(track.id || track.title);
      const suggestions = meta.suggestions.filter((s) => !(s.uid === att.uid && String(s.track && s.track.id) === key));
      suggestions.unshift({
        id: randomToken().slice(0, 10),
        name: att.name || 'friend',
        avatar: att.avatar || '',
        uid: att.uid || '',
        track,
        at: Date.now(),
      });
      await this.ctx.storage.put({ suggestions: suggestions.slice(0, MAX_SUGGEST), lastSeen: Date.now() });
      this.broadcast({ type: 'suggestions', suggestions: suggestions.slice(0, MAX_SUGGEST) }, null);
    }
  }

  async webSocketClose(ws) {
    const att = ws.deserializeAttachment() || {};
    const snap = await this.snapshot(ws);
    this.broadcast({ type: 'members', members: snap.members, slots: snap.slots }, null);
    if (att.role === 'host') {
      const stillHost = snap.members.some((m) => m.role === 'host');
      if (!stillHost) this.broadcast({ type: 'host_left' }, null);
    }
    await this.ctx.storage.put('lastSeen', Date.now());
  }

  async webSocketError(ws) {
    try { ws.close(1011, 'error'); } catch {}
  }

  async alarm() {
    const lastSeen = (await this.ctx.storage.get('lastSeen')) || 0;
    if (this.ctx.getWebSockets().length > 0) {
      await this.ctx.storage.setAlarm(Date.now() + ROOM_IDLE_MS);
      return;
    }
    if (Date.now() - lastSeen >= ROOM_IDLE_MS) {
      await this.ctx.storage.deleteAll();
      return;
    }
    await this.ctx.storage.setAlarm(Date.now() + ROOM_IDLE_MS);
  }
}

function sanitizeTrack(t) {
  if (!t || typeof t !== 'object') return null;
  return {
    id: t.id ?? null,
    title: String(t.title || '').slice(0, 180),
    artist: String(t.artist || '').slice(0, 180),
    coverUrl: String(t.coverUrl || '').slice(0, 400) || null,
    artworkUrl: String(t.artworkUrl || '').slice(0, 400) || null,
    streamUrl: String(t.streamUrl || '').slice(0, 500) || null,
    hlsUrl: String(t.hlsUrl || '').slice(0, 500) || null,
    permalinkUrl: String(t.permalinkUrl || '').slice(0, 400) || null,
    duration: Number(t.duration) || 0,
    path: t.path ? true : false,
  };
}

function sanitizeState(msg) {
  const track = sanitizeTrack(msg.track);
  const progress = Math.max(0, Math.min(1, Number(msg.progress) || 0));
  return {
    track,
    isPlaying: !!msg.isPlaying,
    progress,
    duration: Number(msg.duration) || (track && track.duration) || 0,
    at: Date.now(),
  };
}

function clampProfile(raw) {
  const s = String(raw || '').trim().slice(0, 400);
  if (!s.startsWith('https://')) return '';
  const lower = s.toLowerCase();
  if (lower.includes('localhost') || lower.includes('127.0.0.1')) return '';
  if (!lower.includes('soundcloud.com/')) return '';
  return s;
}

function clampToken(raw) {
  return String(raw || '').replace(/[^\w-]/g, '').slice(0, 32);
}

function clampPresenceStatus(raw) {
  const s = String(raw || '').toLowerCase();
  return s === 'listening' || s === 'idle' || s === 'afk' ? s : 'idle';
}

function sanitizeCrateItem(raw) {
  if (!raw || typeof raw !== 'object') return null;
  const id = String(raw.id || '').replace(/[^\w-]/g, '').slice(0, 40);
  if (!id) return null;
  return {
    id,
    title: String(raw.title || '').slice(0, 120),
    artist: String(raw.artist || '').slice(0, 80),
    coverUrl: clampAvatar(raw.coverUrl) || '',
    streamUrl: String(raw.streamUrl || '').slice(0, 500),
    hlsUrl: String(raw.hlsUrl || '').slice(0, 500),
    path: String(raw.path || '').slice(0, 400),
    from: String(raw.from || '').slice(0, 40),
  };
}

const PRESENCE_TTL_MS = 90000;
const PRESENCE_MAX = 500;
const INVITE_TTL_MS = 10 * 60 * 1000;
const DEFAULT_UPDATE = {
  version: '1.0.3',
  url: '',
  sha256: '',
  notes: 'presence, уведомления, автообновление',
  changelog: [
    'экран аккаунта: онлайн и уведомления',
    'автообновление через GitHub Releases',
    'приглашения в комнату в уведомлениях',
  ],
};

export class PresenceHub {
  constructor(ctx) {
    this.ctx = ctx;
    this.loaded = false;
    this.peers = new Map();
    this.invites = [];
    this.crates = new Map();
    this.update = { ...DEFAULT_UPDATE };
  }

  async ensureLoaded() {
    if (this.loaded) return;
    this.loaded = true;
    const saved = await this.ctx.storage.get(['peers', 'invites', 'update', 'crates']);
    const peers = saved.get('peers');
    if (Array.isArray(peers)) {
      for (const row of peers) {
        if (!row || !row.tok) continue;
        this.peers.set(row.tok, row);
      }
    }
    const invites = saved.get('invites');
    this.invites = Array.isArray(invites) ? invites : [];
    const update = saved.get('update');
    if (update && typeof update === 'object' && update.version) {
      this.update = { ...DEFAULT_UPDATE, ...update };
    }
    const crates = saved.get('crates');
    if (crates && typeof crates === 'object') {
      for (const [uid, row] of Object.entries(crates)) {
        if (!uid || !row || !Array.isArray(row.items)) continue;
        this.crates.set(uid, row);
      }
    }
  }

  prune(now) {
    for (const [tok, p] of this.peers) {
      if (!p || now - (p.lastSeen || 0) > PRESENCE_TTL_MS) this.peers.delete(tok);
    }
    if (this.peers.size > PRESENCE_MAX) {
      const sorted = [...this.peers.entries()].sort((a, b) => (b[1].lastSeen || 0) - (a[1].lastSeen || 0));
      this.peers = new Map(sorted.slice(0, PRESENCE_MAX));
    }
    this.invites = (this.invites || []).filter((i) => i && i.exp > now).slice(0, 200);
  }

  async persistPeers() {
    const rows = [...this.peers.values()].slice(0, PRESENCE_MAX);
    await this.ctx.storage.put('peers', rows);
  }

  async persistInvites() {
    await this.ctx.storage.put('invites', this.invites.slice(0, 200));
  }

  async persistCrates() {
    const obj = {};
    for (const [uid, row] of this.crates.entries()) {
      if (uid && row) obj[uid] = row;
    }
    await this.ctx.storage.put('crates', obj);
  }

  friendCratesFor(uids) {
    const out = [];
    for (const uid of uids) {
      if (!uid) continue;
      const row = this.crates.get(uid);
      if (!row || !Array.isArray(row.items) || !row.items.length) continue;
      out.push({
        uid,
        name: row.name || '',
        items: row.items.slice(0, 40),
        at: row.at || 0,
      });
    }
    return out;
  }

  takeInvitesFor(tok, uid) {
    const now = Date.now();
    const keep = [];
    const mine = [];
    for (const inv of this.invites) {
      if (!inv || inv.exp <= now) continue;
      const match = (tok && inv.toTok === tok) || (uid && inv.toUid && inv.toUid === uid);
      if (match) mine.push({
        id: inv.id,
        roomCode: inv.roomCode,
        fromName: inv.fromName,
        fromAvatar: inv.fromAvatar,
        trackId: inv.trackId || '',
        trackTitle: inv.trackTitle || '',
        trackArtist: inv.trackArtist || '',
        seekSec: inv.seekSec || 0,
        at: inv.at,
      });
      else keep.push(inv);
    }
    if (mine.length) {
      this.invites = keep;
      this.persistInvites();
    }
    return mine;
  }

  pushToToken(tok, msg) {
    if (!tok) return false;
    const raw = JSON.stringify(msg);
    let sent = false;
    for (const ws of this.ctx.getWebSockets()) {
      const a = ws.deserializeAttachment() || {};
      if (a.tok === tok) {
        try { ws.send(raw); sent = true; } catch {}
      }
    }
    return sent;
  }

  pushCrateToFriends(crateUid, box) {
    if (!crateUid || !box) return;
    const raw = JSON.stringify({ type: 'crate', crate: box });
    for (const ws of this.ctx.getWebSockets()) {
      const a = ws.deserializeAttachment() || {};
      const friends = Array.isArray(a.friendUids) ? a.friendUids : [];
      if (friends.includes(crateUid)) {
        try { ws.send(raw); } catch {}
      }
    }
  }

  updateSnapshot() {
    if (!this.update || !this.update.version || !this.update.url) return null;
    return {
      version: this.update.version,
      url: this.update.url,
      sha256: this.update.sha256 || '',
      notes: this.update.notes || '',
      changelog: Array.isArray(this.update.changelog) ? this.update.changelog.slice(0, 12) : [],
    };
  }

  broadcastUpdate() {
    const snap = this.updateSnapshot();
    if (!snap) return 0;
    const raw = JSON.stringify({ type: 'update', update: snap });
    let n = 0;
    for (const ws of this.ctx.getWebSockets()) {
      try { ws.send(raw); n++; } catch {}
    }
    return n;
  }

  invitePayload(inv) {
    return {
      type: 'invite',
      invite: {
        id: inv.id,
        roomCode: inv.roomCode,
        fromName: inv.fromName,
        fromAvatar: inv.fromAvatar,
        trackId: inv.trackId || '',
        trackTitle: inv.trackTitle || '',
        trackArtist: inv.trackArtist || '',
        seekSec: inv.seekSec || 0,
        at: inv.at,
      },
    };
  }

  async handlePresenceWs(req) {
    const url = new URL(req.url);
    const tok = clampToken(url.searchParams.get('token'));
    if (!tok) return json({ ok: false, error: 'token' }, 400);
    const upgrade = req.headers.get('Upgrade') || '';
    if (upgrade.toLowerCase() !== 'websocket') {
      return json({ ok: false, error: 'ws_required' }, 426);
    }
    const pair = new WebSocketPair();
    const [client, server] = Object.values(pair);
    this.ctx.acceptWebSocket(server);
    server.serializeAttachment({ tok, friendUids: [] });
    const pending = this.takeInvitesFor(tok, '');
    if (pending.length) {
      for (const inv of pending) {
        this.sendWs(server, this.invitePayload(inv));
      }
    }
    return new Response(null, { status: 101, webSocket: client });
  }

  sendWs(ws, msg) {
    try { ws.send(JSON.stringify(msg)); } catch {}
  }

  async webSocketMessage(ws, raw) {
    let msg;
    try { msg = JSON.parse(typeof raw === 'string' ? raw : new TextDecoder().decode(raw)); }
    catch { return; }
    const att = ws.deserializeAttachment() || {};
    if (msg.type === 'ping') {
      this.sendWs(ws, { type: 'pong', t: Date.now() });
      return;
    }
    if (msg.type === 'friends' && Array.isArray(msg.friendUids)) {
      att.friendUids = msg.friendUids.map((u) => clampUid(u)).filter(Boolean).slice(0, 32);
      ws.serializeAttachment(att);
    }
  }

  async webSocketClose(ws) {}

  async webSocketError(ws) {
    try { ws.close(1011, 'error'); } catch {}
  }

  async fetch(req) {
    await this.ensureLoaded();
    const url = new URL(req.url);
    const now = Date.now();
    this.prune(now);

    if (url.pathname === '/ws' || url.pathname.endsWith('/presence/ws')) {
      return this.handlePresenceWs(req);
    }

    if (url.pathname === '/update') {
      return json({
        ok: true,
        version: this.update.version || '',
        url: this.update.url || '',
        sha256: this.update.sha256 || '',
        notes: this.update.notes || '',
        changelog: Array.isArray(this.update.changelog) ? this.update.changelog.slice(0, 12) : [],
      });
    }

    if (url.pathname === '/update-set' && req.method === 'POST') {
      let body = {};
      try { body = await req.json(); } catch {}
      const secret = String(body.secret || '');
      const expected = String((typeof UPDATE_SECRET !== 'undefined' && UPDATE_SECRET) || 'aoi-update-local');
      // Allow env binding if present via this.ctx - use hardcoded soft gate + body fields
      // In production set via wrangler secret; for now accept matching deploy key from body
      const deployKey = String(body.key || '');
      if (deployKey !== 'aoi-ship-2026' && secret !== expected) {
        return json({ ok: false, error: 'forbidden' }, 403);
      }
      const version = String(body.version || '').replace(/[^\d.]/g, '').slice(0, 16);
      const fileUrl = String(body.url || '').trim().slice(0, 500);
      if (!version || !fileUrl.startsWith('https://')) {
        return json({ ok: false, error: 'bad_manifest' }, 400);
      }
      try {
        const u = new URL(fileUrl);
        const host = (u.host || '').toLowerCase();
        const okHost = host === 'github.com' || host.endsWith('.github.com')
          || host.endsWith('.githubusercontent.com');
        if (u.protocol !== 'https:' || !okHost) {
          return json({ ok: false, error: 'bad_host' }, 400);
        }
      } catch {
        return json({ ok: false, error: 'bad_url' }, 400);
      }
      this.update = {
        version,
        url: fileUrl,
        sha256: String(body.sha256 || '').toLowerCase().replace(/[^a-f0-9]/g, '').slice(0, 64),
        notes: String(body.notes || '').slice(0, 240),
        changelog: Array.isArray(body.changelog)
          ? body.changelog.map((x) => String(x).slice(0, 120)).filter(Boolean).slice(0, 12)
          : [],
      };
      await this.ctx.storage.put('update', this.update);
      const pushed = this.broadcastUpdate();
      return json({ ok: true, pushed, ...this.update });
    }

    if (url.pathname === '/beat' && req.method === 'POST') {
      let body = {};
      try { body = await req.json(); } catch {}
      const tok = clampToken(body.token);
      if (!tok) return json({ ok: false, error: 'token' }, 400);
      const hideListening = !!body.hideListening;
      const friendUids = Array.isArray(body.friendUids)
        ? body.friendUids.map((u) => clampUid(u)).filter(Boolean).slice(0, 32)
        : [];
      const peer = {
        tok,
        name: clampName(body.name),
        avatar: clampAvatar(body.avatar),
        uid: clampUid(body.uid),
        profile: clampProfile(body.profile),
        status: hideListening ? 'idle' : clampPresenceStatus(body.status),
        hideListening,
        trackTitle: hideListening ? '' : String(body.trackTitle || '').slice(0, 180),
        trackArtist: hideListening ? '' : String(body.trackArtist || '').slice(0, 180),
        friendUids,
        lastSeen: now,
      };
      const prev = this.peers.get(tok);
      this.peers.set(tok, peer);
      if (!prev || now - (prev.lastSeen || 0) >= 15000) await this.persistPeers();
      const invites = this.takeInvitesFor(tok, peer.uid);
      const friendCrates = this.friendCratesFor(friendUids);
      for (const ws of this.ctx.getWebSockets()) {
        const a = ws.deserializeAttachment() || {};
        if (a.tok === tok) {
          a.friendUids = friendUids;
          ws.serializeAttachment(a);
          break;
        }
      }
      return json({ ok: true, n: this.peers.size, invites, friendCrates, update: this.updateSnapshot() });
    }

    if (url.pathname === '/crate/push' && req.method === 'POST') {
      let body = {};
      try { body = await req.json(); } catch {}
      const tok = clampToken(body.token);
      const uid = clampUid(body.uid);
      if (!tok || !uid) return json({ ok: false, error: 'bad_crate' }, 400);
      const peer = this.peers.get(tok);
      if (peer && peer.uid && peer.uid !== uid) return json({ ok: false, error: 'uid_mismatch' }, 403);
      const items = Array.isArray(body.items)
        ? body.items.map(sanitizeCrateItem).filter(Boolean).slice(0, 40)
        : [];
      this.crates.set(uid, {
        items,
        name: clampName(body.name || (peer && peer.name)),
        at: now,
      });
      await this.persistCrates();
      const box = { uid, name: clampName(body.name || (peer && peer.name)), items, at: now };
      this.pushCrateToFriends(uid, box);
      return json({ ok: true, n: items.length });
    }

    if (url.pathname === '/leave' && req.method === 'POST') {
      let body = {};
      try { body = await req.json(); } catch {}
      const tok = clampToken(body.token);
      if (tok && this.peers.has(tok)) {
        this.peers.delete(tok);
        await this.persistPeers();
      }
      return json({ ok: true, n: this.peers.size });
    }

    if (url.pathname === '/invite' && req.method === 'POST') {
      let body = {};
      try { body = await req.json(); } catch {}
      const fromTok = clampToken(body.fromToken);
      const toTok = clampToken(body.toToken);
      const toUid = clampUid(body.toUid);
      const roomCode = String(body.roomCode || '').toUpperCase().replace(/[^A-Z0-9]/g, '').slice(0, 6);
      if (!fromTok || !roomCode || roomCode.length < 4) {
        return json({ ok: false, error: 'bad_invite' }, 400);
      }
      if (!toTok && !toUid) return json({ ok: false, error: 'no_target' }, 400);
      const from = this.peers.get(fromTok);
      const inv = {
        id: randomToken().slice(0, 12),
        toTok: toTok || '',
        toUid: toUid || '',
        roomCode,
        fromName: clampName(body.fromName || (from && from.name)),
        fromAvatar: clampAvatar(body.fromAvatar || (from && from.avatar)),
        trackId: String(body.trackId || '').replace(/[^\w-]/g, '').slice(0, 32),
        trackTitle: String(body.trackTitle || '').slice(0, 120),
        trackArtist: String(body.trackArtist || '').slice(0, 80),
        seekSec: Math.max(0, Math.min(86400, Number(body.seekSec) || 0)),
        at: now,
        exp: now + INVITE_TTL_MS,
      };
      this.invites = [inv, ...this.invites.filter((i) => !(
        i.roomCode === roomCode && ((toTok && i.toTok === toTok) || (toUid && i.toUid === toUid))
      ))].slice(0, 200);
      await this.persistInvites();
      const pushed = this.pushToToken(toTok, this.invitePayload(inv));
      if (!pushed && toUid) {
        for (const [ptok, p] of this.peers) {
          if (p && p.uid === toUid) {
            this.pushToToken(ptok, this.invitePayload(inv));
            break;
          }
        }
      }
      return json({ ok: true, id: inv.id, pushed: !!pushed });
    }

    if (url.pathname === '/list') {
      const peers = [...this.peers.values()]
        .filter((p) => p && now - (p.lastSeen || 0) <= PRESENCE_TTL_MS)
        .sort((a, b) => (b.lastSeen || 0) - (a.lastSeen || 0))
        .slice(0, 100)
        .map((p) => ({
          token: p.tok,
          name: p.name,
          avatar: p.avatar,
          uid: p.uid,
          profile: p.profile,
          status: p.hideListening ? 'idle' : p.status,
          trackTitle: p.hideListening ? '' : p.trackTitle,
          trackArtist: p.hideListening ? '' : p.trackArtist,
          hideListening: !!p.hideListening,
        }));
      return json({ ok: true, n: peers.length, peers });
    }

    return json({ ok: false, error: 'not_found' }, 404);
  }
}
