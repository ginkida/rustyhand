// RustyHand Settings Page — providers, models, config, budget, security, networking
'use strict';

function settingsPage() {
  return {
    tab: 'models-keys',
    sysInfo: {},
    tools: [],
    config: {},
    providers: [],
    models: [],
    toolSearch: '',
    modelSearch: '',
    modelProviderFilter: '',
    modelTierFilter: '',
    providerKeyInputs: {},
    providerTesting: {},
    providerTestResults: {},
    toolsExpanded: false,
    securityExpanded: true,
    networkExpanded: false,

    loading: true,
    loadError: '',
    loadWarnings: [],

    // -- Dynamic config state --
    configSchema: null,
    configSchemaError: '',
    configValues: {},
    configDirty: {},
    configSaving: {},

    // -- Security state --
    securityData: null,
    securityLoadError: '',
    secLoading: false,
    verifyingChain: false,
    chainResult: null,

    coreFeatures: [
      {
        name: 'Path Traversal Prevention', key: 'path_traversal',
        description: 'Blocks directory escape attacks (../) in all file operations. Two-phase validation: syntactic rejection of path components, then canonicalization to normalize symlinks.',
        threat: 'Directory escape, privilege escalation via symlinks',
        impl: 'host_functions.rs — safe_resolve_path() + safe_resolve_parent()'
      },
      {
        name: 'SSRF Protection', key: 'ssrf_protection',
        description: 'Blocks outbound requests to private IPs, localhost, and cloud metadata endpoints (AWS/GCP/Azure). Validates DNS resolution results to defeat rebinding attacks.',
        threat: 'Internal network reconnaissance, cloud credential theft',
        impl: 'host_functions.rs — is_ssrf_target() + is_private_ip()'
      },
      {
        name: 'Capability-Based Access Control', key: 'capability_system',
        description: 'Deny-by-default permission system. Every agent operation (file I/O, network, shell, memory, spawn) requires an explicit capability grant in the manifest.',
        threat: 'Unauthorized resource access, sandbox escape',
        impl: 'host_functions.rs — check_capability() on every host function'
      },
      {
        name: 'Privilege Escalation Prevention', key: 'privilege_escalation_prevention',
        description: 'When a parent agent spawns a child, the kernel enforces child capabilities are a subset of parent capabilities. No agent can grant rights it does not have.',
        threat: 'Capability escalation through agent spawning chains',
        impl: 'kernel_handle.rs — spawn_agent_checked()'
      },
      {
        name: 'Subprocess Environment Isolation', key: 'subprocess_isolation',
        description: 'Child processes (shell tools) inherit only a safe allow-list of environment variables. API keys, database passwords, and secrets are never leaked to subprocesses.',
        threat: 'Secret exfiltration via child process environment',
        impl: 'subprocess_sandbox.rs — env_clear() + SAFE_ENV_VARS'
      },
      {
        name: 'Security Headers', key: 'security_headers',
        description: 'Every HTTP response includes CSP, X-Frame-Options: DENY, X-Content-Type-Options: nosniff, Referrer-Policy, and X-XSS-Protection headers.',
        threat: 'XSS, clickjacking, MIME sniffing, content injection',
        impl: 'middleware.rs — security_headers()'
      },
      {
        name: 'Wire Protocol Authentication', key: 'wire_hmac_auth',
        description: 'Agent-to-agent RHP connections use HMAC-SHA256 mutual authentication with nonce-based handshake and constant-time signature comparison (subtle crate).',
        threat: 'Man-in-the-middle attacks on mesh network',
        impl: 'peer.rs — hmac_sign() + hmac_verify()'
      },
      {
        name: 'Request ID Tracking', key: 'request_id_tracking',
        description: 'Every API request receives a unique UUID (x-request-id header) and is logged with method, path, status code, and latency for full traceability.',
        threat: 'Untraceable actions, forensic blind spots',
        impl: 'middleware.rs — request_logging()'
      }
    ],

    configurableFeatures: [
      {
        name: 'API Rate Limiting', key: 'rate_limiter',
        description: 'GCRA (Generic Cell Rate Algorithm) with cost-aware tokens. Different endpoints cost different amounts — spawning an agent costs 50 tokens, health check costs 1.',
        configHint: 'Hard-coded: 500 tokens/minute per IP. Edit rate_limiter.rs to tune.',
        valueKey: 'rate_limiter'
      },
      {
        name: 'WebSocket Connection Limits', key: 'websocket_limits',
        description: 'Per-IP connection cap prevents connection exhaustion. Idle timeout closes abandoned connections. Message rate limiting prevents flooding.',
        configHint: 'Hard-coded: 5 connections/IP, 30min idle timeout, 64KB max message. Edit ws.rs to tune.',
        valueKey: 'websocket_limits'
      },
      {
        name: 'WASM Dual Metering', key: 'wasm_sandbox',
        description: 'WASM modules run with two independent resource limits: fuel metering (CPU instruction count) and epoch interruption (wall-clock timeout with watchdog thread).',
        configHint: 'Default: 1M fuel units, 30s timeout. Configurable per-agent via SandboxConfig.',
        valueKey: 'wasm_sandbox'
      },
      {
        name: 'Bearer Token Authentication', key: 'auth',
        description: 'All non-health endpoints require Authorization: Bearer header. When no API key is configured, all requests are restricted to localhost only.',
        configHint: 'Set api_key in ~/.rustyhand/config.toml for remote access. Empty = localhost only.',
        valueKey: 'auth'
      }
    ],

    monitoringFeatures: [
      {
        name: 'Merkle Audit Trail', key: 'audit_trail',
        description: 'Every security-critical action is appended to an immutable, tamper-evident log. Each entry is cryptographically linked to the previous via SHA-256 hash chain.',
        configHint: 'Always active. Verify chain integrity from the Audit Log page.',
        valueKey: 'audit_trail'
      },
      {
        name: 'Information Flow Taint Tracking', key: 'taint_tracking',
        description: 'Labels data by provenance (ExternalNetwork, UserInput, PII, Secret, UntrustedAgent) and blocks unsafe flows: external data cannot reach shell_exec, secrets cannot reach network.',
        configHint: 'Always active. Prevents data flow attacks automatically.',
        valueKey: 'taint_tracking'
      },
      {
        name: 'Ed25519 Manifest Signing', key: 'manifest_signing',
        description: 'Agent manifests can be cryptographically signed with Ed25519. Verify manifest integrity before loading to prevent supply chain tampering.',
        configHint: 'Available for use. Sign manifests with ed25519-dalek for verification.',
        valueKey: 'manifest_signing'
      }
    ],

    // -- Peers state --
    peers: [],
    peersLoading: false,
    peersLoadError: '',
    _peerPollTimer: null,

    normalizeError(err) {
      if (!err) return 'Unknown error';
      return err.message || String(err);
    },

    collectWarnings(results) {
      var warnings = [];
      for (var i = 0; i < results.length; i++) {
        if (results[i].status === 'rejected') {
          warnings.push(this.normalizeError(results[i].reason));
        }
      }
      return warnings;
    },

    // -- Settings load --
    async loadSettings() {
      this.loading = true;
      this.loadError = '';
      this.loadWarnings = [];
      var results = await Promise.allSettled([
        this.loadSysInfo(),
        this.loadTools(),
        this.loadConfig(),
        this.loadProviders(),
        this.loadModels()
      ]);
      var warnings = this.collectWarnings(results);
      if (results[0].status === 'rejected') {
        this.loadError = this.normalizeError(results[0].reason) || 'Could not load settings.';
      } else {
        this.loadWarnings = warnings;
      }
      this.loading = false;
    },

    async loadData() { return this.loadSettings(); },

    async loadSysInfo() {
      try {
        var ver = await RustyHandAPI.get('/api/version');
        var status = await RustyHandAPI.get('/api/status');
        this.sysInfo = {
          version: ver.version || '-',
          platform: ver.platform || '-',
          arch: ver.arch || '-',
          uptime_seconds: status.uptime_seconds || 0,
          agent_count: status.agent_count || 0,
          default_provider: status.default_provider || '-',
          default_model: status.default_model || '-'
        };
      } catch(e) {
        throw new Error('/api/version or /api/status: ' + this.normalizeError(e));
      }
    },

    async loadTools() {
      try {
        var data = await RustyHandAPI.get('/api/tools');
        this.tools = data.tools || [];
      } catch(e) {
        this.tools = [];
        throw new Error('/api/tools: ' + this.normalizeError(e));
      }
    },

    async loadConfig() {
      try {
        this.config = await RustyHandAPI.get('/api/config');
      } catch(e) {
        this.config = {};
        throw new Error('/api/config: ' + this.normalizeError(e));
      }
    },

    async loadProviders() {
      try {
        var data = await RustyHandAPI.get('/api/providers');
        this.providers = data.providers || [];
      } catch(e) {
        this.providers = [];
        throw new Error('/api/providers: ' + this.normalizeError(e));
      }
    },

    async loadModels() {
      try {
        var data = await RustyHandAPI.get('/api/models');
        this.models = data.models || [];
      } catch(e) {
        this.models = [];
        throw new Error('/api/models: ' + this.normalizeError(e));
      }
    },

    async loadConfigSchema() {
      this.configSchemaError = '';
      var results = await Promise.allSettled([
        RustyHandAPI.get('/api/config/schema'),
        RustyHandAPI.get('/api/config')
      ]);

      if (results[0].status === 'fulfilled') {
        this.configSchema = results[0].value.sections || null;
        if (!this.configSchema) {
          this.configSchemaError = 'Configuration schema is unavailable. Showing raw JSON only.';
        }
      } else {
        this.configSchema = null;
        this.configSchemaError = '/api/config/schema: ' + this.normalizeError(results[0].reason);
      }

      if (results[1].status === 'fulfilled') {
        this.configValues = results[1].value || {};
      } else {
        this.configValues = this.config || {};
        var configError = '/api/config: ' + this.normalizeError(results[1].reason);
        this.configSchemaError = this.configSchemaError
          ? this.configSchemaError + ' | ' + configError
          : configError;
      }
    },

    isConfigDirty(section, field) {
      return this.configDirty[section + '.' + field] === true;
    },

    markConfigDirty(section, field) {
      this.configDirty[section + '.' + field] = true;
    },

    async saveConfigField(section, field, value) {
      var key = section + '.' + field;
      this.configSaving[key] = true;
      try {
        await RustyHandAPI.post('/api/config/set', { path: key, value: value });
        this.configDirty[key] = false;
        RustyHandToast.success('Saved ' + key);
      } catch(e) {
        RustyHandToast.error('Failed to save: ' + e.message);
      }
      this.configSaving[key] = false;
    },

    get filteredTools() {
      var q = this.toolSearch.toLowerCase().trim();
      if (!q) return this.tools;
      return this.tools.filter(function(t) {
        return t.name.toLowerCase().indexOf(q) !== -1 ||
               (t.description || '').toLowerCase().indexOf(q) !== -1;
      });
    },

    get filteredModels() {
      var self = this;
      return this.models.filter(function(m) {
        if (self.modelProviderFilter && m.provider !== self.modelProviderFilter) return false;
        if (self.modelTierFilter && m.tier !== self.modelTierFilter) return false;
        if (self.modelSearch) {
          var q = self.modelSearch.toLowerCase();
          if (m.id.toLowerCase().indexOf(q) === -1 &&
              (m.display_name || '').toLowerCase().indexOf(q) === -1) return false;
        }
        return true;
      });
    },

    get uniqueProviderNames() {
      var seen = {};
      this.models.forEach(function(m) { seen[m.provider] = true; });
      return Object.keys(seen).sort();
    },

    get uniqueTiers() {
      var seen = {};
      this.models.forEach(function(m) { if (m.tier) seen[m.tier] = true; });
      return Object.keys(seen).sort();
    },

    get configuredProvidersCount() {
      return this.providers.filter(function(p) { return p.auth_status === 'configured'; }).length;
    },

    get keylessProvidersCount() {
      return this.providers.filter(function(p) { return !p.api_key_env || p.key_required === false; }).length;
    },

    get availableModelsCount() {
      return this.models.filter(function(m) { return !!m.available; }).length;
    },

    get dirtyConfigCount() {
      return Object.keys(this.configDirty).filter((key) => this.configDirty[key]).length;
    },

    get providerSetupQueue() {
      return this.providers
        .slice()
        .sort(function(a, b) {
          var aConfigured = a.auth_status === 'configured' ? 1 : 0;
          var bConfigured = b.auth_status === 'configured' ? 1 : 0;
          return aConfigured - bConfigured;
        })
        .slice(0, 4);
    },

    get settingsAttentionItems() {
      var items = [];
      if (this.providers.length && this.configuredProvidersCount === 0) {
        items.push({
          tone: 'warning',
          title: 'No provider is configured',
          detail: 'Set at least one API key to unlock model-backed agents.',
          actionLabel: 'Fix Providers',
          tab: 'models-keys'
        });
      }
      if (this.dirtyConfigCount > 0) {
        items.push({
          tone: 'info',
          title: this.dirtyConfigCount + ' config change(s) not saved',
          detail: 'Apply pending runtime configuration edits before leaving this page.',
          actionLabel: 'Review Config',
          tab: 'config'
        });
      }
      if (this.configSchemaError) {
        items.push({
          tone: 'warning',
          title: 'Config schema fallback is active',
          detail: this.configSchemaError,
          actionLabel: 'Open Config',
          tab: 'config'
        });
      }
      if (this.securityLoadError) {
        items.push({
          tone: 'warning',
          title: 'Live security status unavailable',
          detail: this.securityLoadError,
          actionLabel: 'Open Advanced',
          tab: 'advanced'
        });
      }
      return items.slice(0, 4);
    },

    openTab(tabName) {
      this.tab = tabName;
      if (tabName === 'config' && !this.configSchema) this.loadConfigSchema();
      if (tabName === 'advanced' && !this.securityData && !this.secLoading) this.loadSecurity();
    },

    providerAuthClass(p) {
      if (p.auth_status === 'configured') return 'auth-configured';
      if (p.auth_status === 'not_set' || p.auth_status === 'missing') return 'auth-not-set';
      return 'auth-no-key';
    },

    providerAuthText(p) {
      if (p.auth_status === 'configured') return 'Configured';
      if (p.auth_status === 'not_set' || p.auth_status === 'missing') return 'Not Set';
      return 'No Key Needed';
    },

    providerCardClass(p) {
      if (p.auth_status === 'configured') return 'configured';
      if (p.auth_status === 'not_set' || p.auth_status === 'missing') return 'not-configured';
      return 'no-key';
    },

    tierBadgeClass(tier) {
      if (!tier) return '';
      var t = tier.toLowerCase();
      if (t === 'frontier') return 'tier-frontier';
      if (t === 'smart') return 'tier-smart';
      if (t === 'balanced') return 'tier-balanced';
      if (t === 'fast') return 'tier-fast';
      return '';
    },

    formatCost(cost) {
      if (!cost && cost !== 0) return '-';
      return '$' + cost.toFixed(4);
    },

    formatContext(ctx) {
      if (!ctx) return '-';
      if (ctx >= 1000000) return (ctx / 1000000).toFixed(1) + 'M';
      if (ctx >= 1000) return Math.round(ctx / 1000) + 'K';
      return String(ctx);
    },

    formatUptime(secs) {
      if (!secs) return '-';
      var h = Math.floor(secs / 3600);
      var m = Math.floor((secs % 3600) / 60);
      var s = secs % 60;
      if (h > 0) return h + 'h ' + m + 'm';
      if (m > 0) return m + 'm ' + s + 's';
      return s + 's';
    },

    async saveProviderKey(provider) {
      var key = this.providerKeyInputs[provider.id];
      if (!key || !key.trim()) { RustyHandToast.error('Please enter an API key'); return; }
      try {
        await RustyHandAPI.post('/api/providers/' + encodeURIComponent(provider.id) + '/key', { key: key.trim() });
        RustyHandToast.success('API key saved for ' + provider.display_name);
        this.providerKeyInputs[provider.id] = '';
        await this.loadProviders();
        await this.loadModels();
      } catch(e) {
        RustyHandToast.error('Failed to save key: ' + e.message);
      }
    },

    async removeProviderKey(provider) {
      try {
        await RustyHandAPI.del('/api/providers/' + encodeURIComponent(provider.id) + '/key');
        RustyHandToast.success('API key removed for ' + provider.display_name);
        await this.loadProviders();
        await this.loadModels();
      } catch(e) {
        RustyHandToast.error('Failed to remove key: ' + e.message);
      }
    },

    async testProvider(provider) {
      this.providerTesting[provider.id] = true;
      this.providerTestResults[provider.id] = null;
      try {
        var result = await RustyHandAPI.post('/api/providers/' + encodeURIComponent(provider.id) + '/test', {});
        this.providerTestResults[provider.id] = result;
        if (result.status === 'ok') {
          RustyHandToast.success(provider.display_name + ' connected (' + (result.latency_ms || '?') + 'ms)');
        } else {
          RustyHandToast.error(provider.display_name + ': ' + (result.error || 'Connection failed'));
        }
      } catch(e) {
        this.providerTestResults[provider.id] = { status: 'error', error: e.message };
        RustyHandToast.error('Test failed: ' + e.message);
      }
      this.providerTesting[provider.id] = false;
    },

    // -- Security methods --
    async loadSecurity() {
      this.secLoading = true;
      this.securityLoadError = '';
      try {
        this.securityData = await RustyHandAPI.get('/api/security');
      } catch(e) {
        this.securityData = null;
        this.securityLoadError = e.message || 'Could not load security status.';
      }
      this.secLoading = false;
    },

    isActive(key) {
      if (!this.securityData) return true;
      var core = this.securityData.core_protections || {};
      if (core[key] !== undefined) return core[key];
      return true;
    },

    getConfigValue(key) {
      if (!this.securityData) return null;
      var cfg = this.securityData.configurable || {};
      return cfg[key] || null;
    },

    getMonitoringValue(key) {
      if (!this.securityData) return null;
      var mon = this.securityData.monitoring || {};
      return mon[key] || null;
    },

    formatConfigValue(feature) {
      if (this.securityLoadError && !this.securityData) {
        return 'Live status unavailable — ' + this.securityLoadError;
      }
      var val = this.getConfigValue(feature.valueKey);
      if (!val) return feature.configHint;
      switch (feature.valueKey) {
        case 'rate_limiter':
          return 'Algorithm: ' + (val.algorithm || 'GCRA') + ' | ' + (val.tokens_per_minute || 500) + ' tokens/min per IP';
        case 'websocket_limits':
          return 'Max ' + (val.max_per_ip || 5) + ' conn/IP | ' + Math.round((val.idle_timeout_secs || 1800) / 60) + 'min idle timeout | ' + Math.round((val.max_message_size || 65536) / 1024) + 'KB max msg';
        case 'wasm_sandbox':
          return 'Fuel: ' + (val.fuel_metering ? 'ON' : 'OFF') + ' | Epoch: ' + (val.epoch_interruption ? 'ON' : 'OFF') + ' | Timeout: ' + (val.default_timeout_secs || 30) + 's';
        case 'auth':
          return 'Mode: ' + (val.mode || 'unknown') + (val.api_key_set ? ' (key configured)' : ' (no key set)');
        default:
          return feature.configHint;
      }
    },

    formatMonitoringValue(feature) {
      if (this.securityLoadError && !this.securityData) {
        return 'Live status unavailable — ' + this.securityLoadError;
      }
      var val = this.getMonitoringValue(feature.valueKey);
      if (!val) return feature.configHint;
      switch (feature.valueKey) {
        case 'audit_trail':
          return (val.enabled ? 'Active' : 'Disabled') + ' | ' + (val.algorithm || 'SHA-256') + ' | ' + (val.entry_count || 0) + ' entries logged';
        case 'taint_tracking':
          var labels = val.tracked_labels || [];
          return (val.enabled ? 'Active' : 'Disabled') + ' | Tracking: ' + labels.join(', ');
        case 'manifest_signing':
          return 'Algorithm: ' + (val.algorithm || 'Ed25519') + ' | ' + (val.available ? 'Available' : 'Not available');
        default:
          return feature.configHint;
      }
    },

    async verifyAuditChain() {
      this.verifyingChain = true;
      this.chainResult = null;
      try {
        var res = await RustyHandAPI.get('/api/audit/verify');
        this.chainResult = res;
      } catch(e) {
        this.chainResult = { valid: false, error: e.message };
      }
      this.verifyingChain = false;
    },

    // -- Peers methods --
    async loadPeers() {
      this.peersLoading = true;
      this.peersLoadError = '';
      try {
        var data = await RustyHandAPI.get('/api/peers');
        this.peers = (data.peers || []).map(function(p) {
          return {
            node_id: p.node_id,
            node_name: p.node_name,
            address: p.address,
            state: p.state,
            agent_count: (p.agents || []).length,
            protocol_version: p.protocol_version || 1
          };
        });
      } catch(e) {
        this.peers = [];
        this.peersLoadError = e.message || 'Could not load peers.';
      }
      this.peersLoading = false;
    },

    startPeerPolling() {
      var self = this;
      this.stopPeerPolling();
      this._peerPollTimer = setInterval(async function() {
        if (self.tab !== 'advanced' || !self.networkExpanded) { self.stopPeerPolling(); return; }
        try {
          var data = await RustyHandAPI.get('/api/peers');
          self.peers = (data.peers || []).map(function(p) {
            return {
              node_id: p.node_id,
              node_name: p.node_name,
              address: p.address,
              state: p.state,
              agent_count: (p.agents || []).length,
              protocol_version: p.protocol_version || 1
            };
          });
          self.peersLoadError = '';
        } catch(e) {
          self.peersLoadError = e.message || 'Could not refresh peers.';
        }
      }, 15000);
    },

    stopPeerPolling() {
      if (this._peerPollTimer) { clearInterval(this._peerPollTimer); this._peerPollTimer = null; }
    },

    destroy() {
      this.stopPeerPolling();
    }
  };
}
