// RustyHand API Client — Fetch wrapper, WebSocket manager, auth injection, toast notifications
'use strict';

// ── Toast Notification System ──
var RustyHandToast = (function() {
  var _container = null;
  var _toastId = 0;

  function getContainer() {
    if (!_container) {
      _container = document.getElementById('toast-container');
      if (!_container) {
        _container = document.createElement('div');
        _container.id = 'toast-container';
        _container.className = 'toast-container';
        document.body.appendChild(_container);
      }
    }
    return _container;
  }

  function toast(message, type, duration) {
    type = type || 'info';
    duration = duration || 4000;
    var id = ++_toastId;
    var el = document.createElement('div');
    el.className = 'toast toast-' + type;
    el.setAttribute('data-toast-id', id);

    // Type icon
    var icons = { success: '\u2713', error: '\u2717', warn: '\u26A0', info: '\u2139' };
    var iconEl = document.createElement('span');
    iconEl.className = 'toast-icon';
    iconEl.textContent = icons[type] || icons.info;
    el.appendChild(iconEl);

    var msgSpan = document.createElement('span');
    msgSpan.className = 'toast-msg';
    msgSpan.textContent = message;
    el.appendChild(msgSpan);

    var closeBtn = document.createElement('button');
    closeBtn.className = 'toast-close';
    closeBtn.textContent = '\u00D7';
    closeBtn.onclick = function() { dismissToast(el); };
    el.appendChild(closeBtn);

    // Progress bar
    if (duration > 0) {
      var prog = document.createElement('div');
      prog.className = 'toast-progress';
      prog.style.animationDuration = duration + 'ms';
      el.appendChild(prog);
    }

    el.onclick = function(e) { if (e.target === el) dismissToast(el); };

    // Hover-pause: pause progress + clear timer on mouseenter
    el.addEventListener('mouseenter', function() {
      if (el._dismissTimer) { clearTimeout(el._dismissTimer); el._dismissTimer = null; }
      var p = el.querySelector('.toast-progress');
      if (p) p.style.animationPlayState = 'paused';
    });
    el.addEventListener('mouseleave', function() {
      var p = el.querySelector('.toast-progress');
      if (p) p.style.animationPlayState = 'running';
      el._dismissTimer = setTimeout(function() { dismissToast(el); }, 1000);
    });

    getContainer().appendChild(el);

    // Auto-dismiss
    if (duration > 0) {
      el._dismissTimer = setTimeout(function() { dismissToast(el); }, duration);
    }
    return id;
  }

  function dismissToast(el) {
    if (!el || el.classList.contains('toast-dismiss')) return;
    el.classList.add('toast-dismiss');
    setTimeout(function() { if (el.parentNode) el.parentNode.removeChild(el); }, 400);
  }

  function success(msg, duration) { return toast(msg, 'success', duration); }
  function error(msg, duration) { return toast(msg, 'error', duration || 6000); }
  function warn(msg, duration) { return toast(msg, 'warn', duration || 5000); }
  function info(msg, duration) { return toast(msg, 'info', duration); }

  // Styled confirmation modal — replaces native confirm()
  function confirm(title, message, onConfirm) {
    var overlay = document.createElement('div');
    overlay.className = 'confirm-overlay';

    var modal = document.createElement('div');
    modal.className = 'confirm-modal';

    var titleEl = document.createElement('div');
    titleEl.className = 'confirm-title';
    titleEl.textContent = title;
    modal.appendChild(titleEl);

    var msgEl = document.createElement('div');
    msgEl.className = 'confirm-message';
    msgEl.textContent = message;
    modal.appendChild(msgEl);

    var actions = document.createElement('div');
    actions.className = 'confirm-actions';

    var cancelBtn = document.createElement('button');
    cancelBtn.className = 'btn btn-ghost confirm-cancel';
    cancelBtn.textContent = 'Cancel';
    actions.appendChild(cancelBtn);

    var okBtn = document.createElement('button');
    okBtn.className = 'btn btn-danger confirm-ok';
    okBtn.textContent = 'Confirm';
    actions.appendChild(okBtn);

    modal.appendChild(actions);
    overlay.appendChild(modal);

    function close() { if (overlay.parentNode) overlay.parentNode.removeChild(overlay); document.removeEventListener('keydown', onKey); }
    cancelBtn.onclick = close;
    okBtn.onclick = function() { close(); if (onConfirm) onConfirm(); };
    overlay.addEventListener('click', function(e) { if (e.target === overlay) close(); });

    function onKey(e) { if (e.key === 'Escape') close(); }
    document.addEventListener('keydown', onKey);

    document.body.appendChild(overlay);
    okBtn.focus();
  }

  // Styled prompt modal — replaces native prompt()
  function prompt(title, message, placeholder, onSubmit) {
    var overlay = document.createElement('div');
    overlay.className = 'confirm-overlay';

    var modal = document.createElement('div');
    modal.className = 'confirm-modal';

    var titleEl = document.createElement('div');
    titleEl.className = 'confirm-title';
    titleEl.textContent = title;
    modal.appendChild(titleEl);

    var msgEl = document.createElement('div');
    msgEl.className = 'confirm-message';
    msgEl.textContent = message;
    modal.appendChild(msgEl);

    var input = document.createElement('input');
    input.className = 'form-input';
    input.type = 'text';
    input.placeholder = placeholder || '';
    input.style.marginBottom = '16px';
    modal.appendChild(input);

    var actions = document.createElement('div');
    actions.className = 'confirm-actions';

    var cancelBtn = document.createElement('button');
    cancelBtn.className = 'btn btn-ghost confirm-cancel';
    cancelBtn.textContent = 'Cancel';
    actions.appendChild(cancelBtn);

    var okBtn = document.createElement('button');
    okBtn.className = 'btn btn-primary confirm-ok';
    okBtn.textContent = 'OK';
    actions.appendChild(okBtn);

    modal.appendChild(actions);
    overlay.appendChild(modal);

    function close() { if (overlay.parentNode) overlay.parentNode.removeChild(overlay); document.removeEventListener('keydown', onKey); }
    cancelBtn.onclick = close;
    okBtn.onclick = function() { var val = input.value; close(); if (onSubmit) onSubmit(val); };
    input.addEventListener('keydown', function(e) { if (e.key === 'Enter') { e.preventDefault(); okBtn.click(); } });
    overlay.addEventListener('click', function(e) { if (e.target === overlay) close(); });

    function onKey(e) { if (e.key === 'Escape') close(); }
    document.addEventListener('keydown', onKey);

    document.body.appendChild(overlay);
    input.focus();
  }

  return {
    toast: toast,
    success: success,
    error: error,
    warn: warn,
    info: info,
    confirm: confirm,
    prompt: prompt
  };
})();

// ── Friendly Error Messages ──
function friendlyError(status, serverMsg) {
  if (status === 0 || !status) return 'Cannot reach daemon — is rustyhand running?';
  if (status === 401) return 'Not authorized — check your API key';
  if (status === 403) return 'Permission denied';
  if (status === 404) return serverMsg || 'Resource not found';
  if (status === 429) return 'Rate limited — slow down and try again';
  if (status === 413) return 'Request too large';
  if (status === 500) return 'Server error — check daemon logs';
  if (status === 502 || status === 503) return 'Daemon unavailable — is it running?';
  return serverMsg || 'Unexpected error (' + status + ')';
}

// ── API Client ──
var RustyHandAPI = (function() {
  var BASE = window.location.origin;
  var WS_BASE = BASE.replace(/^http/, 'ws');
  var _authToken = '';

  // Connection state tracking
  var _connectionState = 'connected';
  var _connectionListeners = [];

  function headers() {
    var h = { 'Content-Type': 'application/json' };
    if (_authToken) h['Authorization'] = 'Bearer ' + _authToken;
    return h;
  }

  function setConnectionState(state) {
    if (_connectionState === state) return;
    _connectionState = state;
    _connectionListeners.forEach(function(fn) { fn(state); });
  }

  function onConnectionChange(fn) { _connectionListeners.push(fn); }

  function request(method, path, body) {
    var opts = { method: method, headers: headers() };
    if (body !== undefined) opts.body = JSON.stringify(body);
    return fetch(BASE + path, opts).then(function(r) {
      if (_connectionState !== 'connected') setConnectionState('connected');
      if (!r.ok) {
        return r.text().then(function(text) {
          var msg = '';
          try {
            var json = JSON.parse(text);
            msg = json.error || r.statusText;
          } catch(e) {
            msg = r.statusText;
          }
          throw new Error(friendlyError(r.status, msg));
        });
      }
      var ct = r.headers.get('content-type') || '';
      if (ct.indexOf('application/json') >= 0) return r.json();
      return r.text().then(function(t) {
        try { return JSON.parse(t); } catch(e) { return { text: t }; }
      });
    }).catch(function(e) {
      if (e.name === 'TypeError' && e.message.includes('Failed to fetch')) {
        setConnectionState('disconnected');
        throw new Error('Cannot connect to daemon — is rustyhand running?');
      }
      throw e;
    });
  }

  function get(path) { return request('GET', path); }
  function post(path, body) { return request('POST', path, body); }
  function put(path, body) { return request('PUT', path, body); }
  function patch(path, body) { return request('PATCH', path, body); }
  function del(path) { return request('DELETE', path); }

  // WebSocket manager with auto-reconnect
  var _ws = null;
  var _wsCallbacks = {};
  var _wsConnected = false;
  var _wsAgentId = null;
  var _reconnectTimer = null;
  var _reconnectAttempts = 0;
  var MAX_RECONNECT = 20;

  function wsConnect(agentId, callbacks) {
    wsDisconnect();
    _wsCallbacks = callbacks || {};
    _wsAgentId = agentId;
    _reconnectAttempts = 0;
    _doConnect(agentId);
  }

  function _doConnect(agentId) {
    try {
      var url = WS_BASE + '/api/agents/' + agentId + '/ws';
      if (_authToken) url += '?token=' + encodeURIComponent(_authToken);
      var ws = new WebSocket(url);
      _ws = ws;

      ws.onopen = function() {
        if (_ws !== ws) return; // Stale socket — a newer connect replaced us
        _wsConnected = true;
        setConnectionState('connected');
        if (_reconnectAttempts > 0) {
          RustyHandToast.success('Reconnected');
          if (typeof Alpine !== 'undefined' && Alpine.store('app')) {
            Alpine.store('app').refreshAgents();
          }
        }
        _reconnectAttempts = 0;
        if (_wsCallbacks.onOpen) _wsCallbacks.onOpen();
      };

      ws.onmessage = function(e) {
        if (_ws !== ws) return;
        try {
          var data = JSON.parse(e.data);
          if (_wsCallbacks.onMessage) _wsCallbacks.onMessage(data);
        } catch(err) { /* ignore parse errors */ }
      };

      ws.onclose = function(e) {
        if (_ws !== ws) return; // Stale socket — ignore old close events
        _wsConnected = false;
        _ws = null;
        if (_wsAgentId && _reconnectAttempts < MAX_RECONNECT && e.code !== 1000) {
          _reconnectAttempts++;
          setConnectionState('reconnecting');
          // Only show toast after 2+ failures (skip transient blips)
          if (_reconnectAttempts === 2) {
            RustyHandToast.warn('Connection lost, reconnecting...');
          }
          var delay = Math.min(1000 * Math.pow(1.5, _reconnectAttempts - 1), 30000);
          _reconnectTimer = setTimeout(function() { _doConnect(_wsAgentId); }, delay);
          return;
        }
        if (_wsAgentId && _reconnectAttempts >= MAX_RECONNECT) {
          setConnectionState('disconnected');
          RustyHandToast.error('Connection lost — switched to HTTP mode', 0);
        }
        if (_wsCallbacks.onClose) _wsCallbacks.onClose();
      };

      ws.onerror = function() {
        if (_ws !== ws) return;
        _wsConnected = false;
        if (_wsCallbacks.onError) _wsCallbacks.onError();
      };
    } catch(e) {
      _wsConnected = false;
    }
  }

  function wsDisconnect() {
    _wsAgentId = null;
    _reconnectAttempts = MAX_RECONNECT;
    if (_reconnectTimer) { clearTimeout(_reconnectTimer); _reconnectTimer = null; }
    if (_ws) { var old = _ws; _ws = null; old.close(1000); }
    _wsConnected = false;
  }

  function wsSend(data) {
    if (_ws && _ws.readyState === WebSocket.OPEN) {
      _ws.send(JSON.stringify(data));
      return true;
    }
    return false;
  }

  function isWsConnected() { return _wsConnected; }

  function getToken() { return _authToken; }

  function setToken(token) {
    _authToken = token || '';
    if (token) {
      sessionStorage.setItem('rh-token', token);
    } else {
      sessionStorage.removeItem('rh-token');
    }
  }

  function initAuth() {
    var saved = sessionStorage.getItem('rh-token');
    if (saved) _authToken = saved;
    return !!_authToken;
  }

  function logout() {
    _authToken = '';
    sessionStorage.removeItem('rh-token');
  }

  // Check if auth is required (uses public endpoint — no 401 in console)
  async function checkAuthRequired() {
    try {
      var resp = await fetch(BASE + '/api/onboarding');
      var data = await resp.json();
      return !!data.api_key_set;
    } catch(e) {
      return false;
    }
  }

  // Validate token by calling /api/auth/me
  async function validateToken(token) {
    try {
      var resp = await fetch(BASE + '/api/auth/me', {
        headers: { 'Authorization': 'Bearer ' + token }
      });
      return resp.ok;
    } catch(e) {
      return false;
    }
  }

  function upload(agentId, file) {
    var hdrs = {
      'Content-Type': file.type || 'application/octet-stream',
      'X-Filename': file.name
    };
    if (_authToken) hdrs['Authorization'] = 'Bearer ' + _authToken;
    return fetch(BASE + '/api/agents/' + agentId + '/upload', {
      method: 'POST',
      headers: hdrs,
      body: file
    }).then(function(r) {
      if (!r.ok) throw new Error('Upload failed');
      return r.json();
    });
  }

  return {
    getToken: getToken,
    setToken: setToken,
    initAuth: initAuth,
    logout: logout,
    checkAuthRequired: checkAuthRequired,
    validateToken: validateToken,
    get: get,
    post: post,
    put: put,
    patch: patch,
    del: del,
    delete: del,
    upload: upload,
    wsConnect: wsConnect,
    wsDisconnect: wsDisconnect,
    wsSend: wsSend,
    isWsConnected: isWsConnected,
    onConnectionChange: onConnectionChange
  };
})();
