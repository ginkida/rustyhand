// RustyHand Chat Page — Agent chat with markdown + streaming
'use strict';

function chatPage() {
  var msgId = 0;
  return {
    currentAgent: null,
    messages: [],
    inputText: '',
    sending: false,
    messageQueue: [],    // Queue for messages sent while streaming
    thinkingMode: 'off', // 'off' | 'on' | 'stream'
    _wsAgent: null,
    _httpFallbackNotified: false,
    showSlashMenu: false,
    slashFilter: '',
    slashIdx: 0,
    attachments: [],
    dragOver: false,
    contextPressure: 'low', // green/yellow/orange/red indicator
    sessionCostUsd: 0,      // running total cost for the current session
    _typingTimeout: null,
    // Multi-session state
    sessions: [],
    sessionsLoadError: '',
    sessionsOpen: false,
    searchOpen: false,
    searchQuery: '',
    // Activity log state
    actionLog: [],
    logOpen: false,
    // Model switcher in chat header
    modelSwitchOpen: false,
    modelSwitchProviders: [],
    modelSwitchModels: [],
    modelSwitchLoaded: false,
    modelSwitchProvider: '',
    modelSwitchSaving: false,
    // Group editor in chat header
    groupEditOpen: false,
    groupDraft: '',
    groupSaving: false,
    // Voice recording state
    recording: false,
    _mediaRecorder: null,
    _audioChunks: [],
    recordingTime: 0,
    _recordingTimer: null,
    slashCommands: [
      { cmd: '/help', desc: 'Show available commands' },
      { cmd: '/agents', desc: 'Switch to Agents page' },
      { cmd: '/new', desc: 'Reset session (clear history)' },
      { cmd: '/compact', desc: 'Trigger LLM session compaction' },
      { cmd: '/model', desc: 'Show or switch model', args: 'name' },
      { cmd: '/retry', desc: 'Regenerate the last response' },
      { cmd: '/stop', desc: 'Cancel current agent run' },
      { cmd: '/usage', desc: 'Show session token usage & cost' },
      { cmd: '/think', desc: 'Toggle extended thinking', args: 'on | off | stream' },
      { cmd: '/temp', desc: 'Get or set sampling temperature', args: '0.0–2.0' },
      { cmd: '/system', desc: 'View or replace the system prompt', args: 'new prompt text' },
      { cmd: '/context', desc: 'Show context window usage & pressure' },
      { cmd: '/verbose', desc: 'Cycle tool detail level', args: 'off | on | full' },
      { cmd: '/queue', desc: 'Check if agent is processing' },
      { cmd: '/status', desc: 'Show system status' },
      { cmd: '/clear', desc: 'Clear chat display' },
      { cmd: '/exit', desc: 'Disconnect from agent' },
      { cmd: '/budget', desc: 'Show spending limits and current costs' },
      { cmd: '/peers', desc: 'Show RHP peer network status' },
      { cmd: '/a2a', desc: 'List discovered external A2A agents' },
      { cmd: '/label', desc: 'Name the current session', args: 'title (empty to clear)' },
      { cmd: '/remember', desc: 'Store a key-value note in agent memory', args: '<key> <value>' },
      { cmd: '/recall', desc: 'Retrieve a memory note', args: '[key]' },
      { cmd: '/workflows', desc: 'List available workflows' },
      { cmd: '/workflow', desc: 'Run a workflow', args: 'run <name> [input]' },
      { cmd: '/export', desc: 'Export chat as Markdown file', args: '' }
    ],
    isScrolledUp: false,
    isDragging: false,
    tokenCount: 0,
    streamStartTime: 0,
    tokensPerSec: 0,
    starredOpen: false,
    starredMessages: [],

    // ── Tip Bar ──
    tipIndex: 0,
    tips: ['Type / for commands', '/think on for reasoning', 'Drag files to attach', '/model to switch models', '/context to check usage', '/verbose off to hide tool details'],
    tipTimer: null,
    get currentTip() {
      if (localStorage.getItem('rh-tips-off') === 'true') return '';
      return this.tips[this.tipIndex % this.tips.length];
    },
    dismissTips: function() { localStorage.setItem('rh-tips-off', 'true'); },
    startTipCycle: function() {
      var self = this;
      if (this.tipTimer) clearInterval(this.tipTimer);
      this.tipTimer = setInterval(function() {
        self.tipIndex = (self.tipIndex + 1) % self.tips.length;
      }, 30000);
    },

    // ── Model Switcher (chat header dropdown) ──
    async openModelSwitch() {
      this.modelSwitchOpen = !this.modelSwitchOpen;
      if (!this.modelSwitchOpen) return;
      if (this.modelSwitchLoaded) {
        this.modelSwitchProvider = (this.currentAgent && this.currentAgent.model_provider) || '';
        return;
      }
      try {
        var results = await Promise.all([
          RustyHandAPI.get('/api/models?available=true'),
          RustyHandAPI.get('/api/providers')
        ]);
        var allModels = results[0].models || [];
        var providers = (results[1].providers || []).filter(function(p) {
          if (p.auth_status === 'configured') return true;
          if ((p.auth_status === 'not_required') && p.is_local && p.reachable) return true;
          return false;
        });
        this.modelSwitchProviders = providers;
        this.modelSwitchModels = allModels;
        this.modelSwitchProvider = (this.currentAgent && this.currentAgent.model_provider) || '';
        this.modelSwitchLoaded = true;
      } catch(e) {
        console.warn('Model switch load failed:', e.message);
      }
    },

    get modelSwitchFiltered() {
      var provider = this.modelSwitchProvider;
      var filtered = this.modelSwitchModels.filter(function(m) { return m.provider === provider; });
      var tierOrder = { frontier: 0, smart: 1, balanced: 2, fast: 3, local: 4 };
      filtered.sort(function(a, b) {
        var aO = tierOrder[a.tier] !== undefined ? tierOrder[a.tier] : 5;
        var bO = tierOrder[b.tier] !== undefined ? tierOrder[b.tier] : 5;
        if (aO !== bO) return aO - bO;
        return (a.display_name || a.id).localeCompare(b.display_name || b.id);
      });
      return filtered;
    },

    async switchModel(modelId) {
      if (!this.currentAgent) return;
      this.modelSwitchSaving = true;
      try {
        await RustyHandAPI.put('/api/agents/' + this.currentAgent.id + '/model', { model: modelId });
        var entry = this.modelSwitchModels.find(function(m) { return m.id === modelId; });
        this.currentAgent.model_name = modelId;
        if (entry) this.currentAgent.model_provider = entry.provider;
        RustyHandToast.success('Model: ' + (entry ? entry.display_name : modelId));
        this.modelSwitchOpen = false;
        await Alpine.store('app').refreshAgents();
      } catch(e) {
        RustyHandToast.error('Failed: ' + e.message);
      }
      this.modelSwitchSaving = false;
    },

    modelTierColor(tier) {
      var colors = { frontier: '#a855f7', smart: '#3b82f6', balanced: '#22c55e', fast: '#eab308', local: '#6b7280' };
      return colors[tier] || '#6b7280';
    },

    openGroupEditor() {
      if (!this.currentAgent) return;
      this.groupEditOpen = true;
      this.groupDraft = (this.currentAgent.group || '');
      this.modelSwitchOpen = false;
      this.$nextTick(function() {
        if (this.$refs.groupInput) this.$refs.groupInput.focus();
      }.bind(this));
    },

    cancelGroupEditor() {
      this.groupEditOpen = false;
      this.groupDraft = '';
    },

    async saveGroup() {
      if (!this.currentAgent) return;
      this.groupSaving = true;
      try {
        await RustyHandAPI.patch('/api/agents/' + this.currentAgent.id + '/config', {
          group: this.groupDraft
        });
        await Alpine.store('app').refreshAgents();
        var updated = Alpine.store('app').agents.find(function(agent) {
          return agent.id === this.currentAgent.id;
        }.bind(this));
        if (updated) {
          this.currentAgent = updated;
          Alpine.store('app').activeChatAgent = updated;
        } else {
          this.currentAgent.group = (this.groupDraft || '').trim();
        }
        this.groupEditOpen = false;
        RustyHandToast.success((this.currentAgent.group ? 'Group: ' + this.currentAgent.group : 'Group cleared'));
      } catch(e) {
        RustyHandToast.error('Failed to update group: ' + e.message);
      }
      this.groupSaving = false;
    },

    // Backward compat helper
    get thinkingEnabled() { return this.thinkingMode !== 'off'; },

    normalizeError(err) {
      if (!err) return 'Unknown error';
      return err.message || String(err);
    },

    appendSystemMessage(text) {
      this.messages.push({ id: ++msgId, role: 'system', text: text, meta: '', tools: [], ts: Date.now() });
      this.scrollToBottom();
    },

    // Context pressure dot color
    get contextDotColor() {
      switch (this.contextPressure) {
        case 'critical': return '#ef4444';
        case 'high': return '#f97316';
        case 'medium': return '#eab308';
        default: return '#22c55e';
      }
    },

    init() {
      var self = this;

      // Start tip cycle
      this.startTipCycle();

      // Restore last active agent from session
      var savedAgentId = sessionStorage.getItem('rh-active-agent');
      if (savedAgentId && !this.currentAgent) {
        var agents = Alpine.store('app').agents || [];
        var saved = agents.find(function(a) { return a.id === savedAgentId; });
        if (saved) {
          this.$nextTick(function() { self.selectAgent(saved); });
        }
      }

      // Fetch dynamic commands from server
      this.fetchCommands();

      // Ctrl+/ keyboard shortcut
      document.addEventListener('keydown', function(e) {
        if ((e.ctrlKey || e.metaKey) && e.key === '/') {
          e.preventDefault();
          var input = document.getElementById('msg-input');
          if (input) { input.focus(); self.inputText = '/'; }
        }
        // Ctrl+F for chat search
        if ((e.ctrlKey || e.metaKey) && e.key === 'f' && self.currentAgent) {
          e.preventDefault();
          self.toggleSearch();
        }
      });

      // Load session + session list when agent changes
      this.$watch('currentAgent', function(agent) {
        self.groupEditOpen = false;
        self.groupDraft = '';
        if (agent) {
          self.loadSession(agent.id);
          self.loadSessions(agent.id);
          self.loadStars();
        }
      });

      // Check for pending agent from Agents page (set before chat mounted)
      var store = Alpine.store('app');
      if (store.pendingAgent) {
        self.selectAgent(store.pendingAgent);
        store.pendingAgent = null;
      }

      // Watch for future pending agent selections (e.g., user clicks agent while on chat)
      this.$watch('$store.app.pendingAgent', function(agent) {
        if (agent) {
          self.selectAgent(agent);
          Alpine.store('app').pendingAgent = null;
        }
      });

      // Watch for slash commands and auto-save draft
      this.$watch('inputText', function(val) {
        if (val.startsWith('/')) {
          self.slashFilter = val.slice(1).toLowerCase();
          self.showSlashMenu = true;
          self.slashIdx = 0;
        } else {
          self.showSlashMenu = false;
        }
        // Persist draft per agent so it survives navigation
        if (self.currentAgent) {
          try {
            if (val.trim()) { localStorage.setItem('rh-draft-' + self.currentAgent.id, val); }
            else { localStorage.removeItem('rh-draft-' + self.currentAgent.id); }
          } catch(_) {}
        }
      });
    },

    // Fetch dynamic slash commands from server
    fetchCommands: function() {
      var self = this;
      RustyHandAPI.get('/api/commands').then(function(data) {
        if (data.commands && data.commands.length) {
          // Build a set of known cmds to avoid duplicates
          var existing = {};
          self.slashCommands.forEach(function(c) { existing[c.cmd] = true; });
          data.commands.forEach(function(c) {
            if (!existing[c.cmd]) {
              self.slashCommands.push({ cmd: c.cmd, desc: c.desc || '', source: c.source || 'server' });
              existing[c.cmd] = true;
            }
          });
        }
      }).catch(function(e) {
        console.warn('[Chat] Failed to load dynamic slash commands:', self.normalizeError(e));
      });
    },

    get filteredSlashCommands() {
      if (!this.slashFilter) return this.slashCommands;
      var f = this.slashFilter;
      return this.slashCommands.filter(function(c) {
        return c.cmd.toLowerCase().indexOf(f) !== -1 || c.desc.toLowerCase().indexOf(f) !== -1;
      });
    },

    // Clear any stuck typing indicator after 120s
    _resetTypingTimeout: function() {
      var self = this;
      if (self._typingTimeout) clearTimeout(self._typingTimeout);
      self._typingTimeout = setTimeout(function() {
        // Auto-clear stuck typing indicators
        self.messages = self.messages.filter(function(m) { return !m.thinking; });
        self.sending = false;
      }, 45000);
    },

    _clearTypingTimeout: function() {
      if (this._typingTimeout) {
        clearTimeout(this._typingTimeout);
        this._typingTimeout = null;
      }
    },

    executeSlashCommand(cmd, cmdArgs) {
      this.showSlashMenu = false;
      this.inputText = '';
      var self = this;
      cmdArgs = cmdArgs || '';
      switch (cmd) {
        case '/help':
          self.messages.push({ id: ++msgId, role: 'system', text: self.slashCommands.map(function(c) { return '`' + c.cmd + '` — ' + c.desc; }).join('\n'), meta: '', tools: [] });
          self.scrollToBottom();
          break;
        case '/agents':
          location.hash = 'agents';
          break;
        case '/new':
          if (self.currentAgent) {
            RustyHandAPI.post('/api/agents/' + self.currentAgent.id + '/session/reset', {}).then(function() {
              self.messages = [];
              RustyHandToast.success('Session reset');
            }).catch(function(e) { RustyHandToast.error('Reset failed: ' + e.message); });
          }
          break;
        case '/compact':
          if (self.currentAgent) {
            self.messages.push({ id: ++msgId, role: 'system', text: 'Compacting session...', meta: '', tools: [] });
            RustyHandAPI.post('/api/agents/' + self.currentAgent.id + '/session/compact', {}).then(function(res) {
              self.messages.push({ id: ++msgId, role: 'system', text: res.message || 'Compaction complete', meta: '', tools: [] });
              self.scrollToBottom();
            }).catch(function(e) { RustyHandToast.error('Compaction failed: ' + e.message); });
          }
          break;
        case '/retry':
          if (self.currentAgent) self.sendWsCommand('retry', '');
          break;
        case '/stop':
          if (self.currentAgent) {
            RustyHandAPI.post('/api/agents/' + self.currentAgent.id + '/stop', {}).then(function(res) {
              self.messages.push({ id: ++msgId, role: 'system', text: res.message || 'Run cancelled', meta: '', tools: [] });
              self.sending = false;
              self.scrollToBottom();
            }).catch(function(e) { RustyHandToast.error('Stop failed: ' + e.message); });
          }
          break;
        case '/usage':
          if (self.currentAgent) {
            var approxTokens = self.messages.reduce(function(sum, m) { return sum + Math.round((m.text || '').length / 4); }, 0);
            self.messages.push({ id: ++msgId, role: 'system', text: '**Session Usage**\n- Messages: ' + self.messages.length + '\n- Approx tokens: ~' + approxTokens, meta: '', tools: [] });
            self.scrollToBottom();
          }
          break;
        case '/think':
          if (cmdArgs === 'on' || cmdArgs === 'stream') {
            self.thinkingMode = cmdArgs === 'stream' ? 'stream' : 'on';
          } else if (cmdArgs === 'off') {
            self.thinkingMode = 'off';
          } else {
            // Cycle: off -> stream -> off
            self.thinkingMode = self.thinkingMode === 'off' ? 'stream' : 'off';
          }
          // Sync the server-side thinking flag for the agent
          if (self.currentAgent && RustyHandAPI.isWsConnected()) {
            RustyHandAPI.wsSend({ type: 'command', command: 'think', args: self.thinkingMode === 'off' ? 'off' : 'on' });
          }
          var modeLabel2 = self.thinkingMode !== 'off' ? 'enabled — reasoning tokens will appear in a collapsible panel' : 'disabled';
          self.messages.push({ id: ++msgId, role: 'system', text: 'Extended thinking **' + modeLabel2 + '**.',
            meta: '', tools: [] });
          self.scrollToBottom();
          break;
        case '/temp':
          if (self.currentAgent && RustyHandAPI.isWsConnected()) {
            RustyHandAPI.wsSend({ type: 'command', command: 'temp', args: cmdArgs });
          } else {
            self.messages.push({ id: ++msgId, role: 'system', text: 'Not connected. Connect to an agent first.', meta: '', tools: [] });
            self.scrollToBottom();
          }
          break;
        case '/system':
          if (self.currentAgent && RustyHandAPI.isWsConnected()) {
            RustyHandAPI.wsSend({ type: 'command', command: 'system', args: cmdArgs });
          } else {
            self.messages.push({ id: ++msgId, role: 'system', text: 'Not connected. Connect to an agent first.', meta: '', tools: [] });
            self.scrollToBottom();
          }
          break;
        case '/context':
          // Send via WS command
          if (self.currentAgent && RustyHandAPI.isWsConnected()) {
            RustyHandAPI.wsSend({ type: 'command', command: 'context', args: '' });
          } else {
            self.messages.push({ id: ++msgId, role: 'system', text: 'Not connected. Connect to an agent first.', meta: '', tools: [] });
            self.scrollToBottom();
          }
          break;
        case '/verbose':
          if (self.currentAgent && RustyHandAPI.isWsConnected()) {
            RustyHandAPI.wsSend({ type: 'command', command: 'verbose', args: cmdArgs });
          } else {
            self.messages.push({ id: ++msgId, role: 'system', text: 'Not connected. Connect to an agent first.', meta: '', tools: [] });
            self.scrollToBottom();
          }
          break;
        case '/queue':
          if (self.currentAgent && RustyHandAPI.isWsConnected()) {
            RustyHandAPI.wsSend({ type: 'command', command: 'queue', args: '' });
          } else {
            self.messages.push({ id: ++msgId, role: 'system', text: 'Not connected.', meta: '', tools: [] });
            self.scrollToBottom();
          }
          break;
        case '/status':
          RustyHandAPI.get('/api/status').then(function(s) {
            self.messages.push({ id: ++msgId, role: 'system', text: '**System Status**\n- Agents: ' + (s.agent_count || 0) + '\n- Uptime: ' + (s.uptime_seconds || 0) + 's\n- Version: ' + (s.version || '?'), meta: '', tools: [] });
            self.scrollToBottom();
          }).catch(function(e) {
            self.appendSystemMessage('Status request failed: ' + self.normalizeError(e));
          });
          break;
        case '/model':
          if (self.currentAgent) {
            if (cmdArgs) {
              RustyHandAPI.put('/api/agents/' + self.currentAgent.id + '/model', { model: cmdArgs }).then(function() {
                self.currentAgent.model_name = cmdArgs;
                self.messages.push({ id: ++msgId, role: 'system', text: 'Model switched to: `' + cmdArgs + '`', meta: '', tools: [] });
                self.scrollToBottom();
              }).catch(function(e) { RustyHandToast.error('Model switch failed: ' + e.message); });
            } else if (RustyHandAPI.isWsConnected()) {
              // WS handler returns current model + full catalog list with tier icons
              RustyHandAPI.wsSend({ type: 'command', command: 'model', args: '' });
            } else {
              self.messages.push({ id: ++msgId, role: 'system', text: '**Current Model**\n- Provider: `' + (self.currentAgent.model_provider || '?') + '`\n- Model: `' + (self.currentAgent.model_name || '?') + '`', meta: '', tools: [] });
              self.scrollToBottom();
            }
          } else {
            self.messages.push({ id: ++msgId, role: 'system', text: 'No agent selected.', meta: '', tools: [] });
            self.scrollToBottom();
          }
          break;
        case '/clear':
          self.messages = [];
          break;
        case '/export':
          self.exportChat();
          break;
        case '/exit':
          RustyHandAPI.wsDisconnect();
          self._wsAgent = null;
          self.currentAgent = null;
          self.messages = [];
          window.dispatchEvent(new Event('close-chat'));
          break;
        case '/budget':
          RustyHandAPI.get('/api/budget').then(function(b) {
            var fmt = function(v) { return v > 0 ? '$' + v.toFixed(2) : 'unlimited'; };
            self.messages.push({ id: ++msgId, role: 'system', text: '**Budget Status**\n' +
              '- Hourly: $' + (b.hourly_spend||0).toFixed(4) + ' / ' + fmt(b.hourly_limit) + '\n' +
              '- Daily: $' + (b.daily_spend||0).toFixed(4) + ' / ' + fmt(b.daily_limit) + '\n' +
              '- Monthly: $' + (b.monthly_spend||0).toFixed(4) + ' / ' + fmt(b.monthly_limit), meta: '', tools: [] });
            self.scrollToBottom();
          }).catch(function(e) {
            self.appendSystemMessage('Budget request failed: ' + self.normalizeError(e));
          });
          break;
        case '/peers':
          RustyHandAPI.get('/api/network/status').then(function(ns) {
            self.messages.push({ id: ++msgId, role: 'system', text: '**RHP Network**\n' +
              '- Status: ' + (ns.enabled ? 'Enabled' : 'Disabled') + '\n' +
              '- Connected peers: ' + (ns.connected_peers||0) + ' / ' + (ns.total_peers||0), meta: '', tools: [] });
            self.scrollToBottom();
          }).catch(function(e) {
            self.appendSystemMessage('Peer status request failed: ' + self.normalizeError(e));
          });
          break;
        case '/label':
          if (self.currentAgent) self.sendWsCommand('label', (cmdArgs || '').trim());
          break;
        case '/remember':
          if (self.currentAgent) self.sendWsCommand('remember', (cmdArgs || '').trim());
          break;
        case '/recall':
          if (self.currentAgent) self.sendWsCommand('recall', (cmdArgs || '').trim());
          break;
        case '/workflows':
          if (self.currentAgent) self.sendWsCommand('workflows', '');
          break;
        case '/workflow':
          if (self.currentAgent) self.sendWsCommand('workflow', (cmdArgs || '').trim());
          break;
        case '/a2a':
          RustyHandAPI.get('/api/a2a/agents').then(function(res) {
            var agents = res.agents || [];
            if (!agents.length) {
              self.messages.push({ id: ++msgId, role: 'system', text: 'No external A2A agents discovered.', meta: '', tools: [] });
            } else {
              var lines = agents.map(function(a) { return '- **' + a.name + '** — ' + a.url; });
              self.messages.push({ id: ++msgId, role: 'system', text: '**A2A Agents (' + agents.length + ')**\n' + lines.join('\n'), meta: '', tools: [] });
            }
            self.scrollToBottom();
          }).catch(function(e) {
            self.appendSystemMessage('A2A discovery request failed: ' + self.normalizeError(e));
          });
          break;
      }
    },

    selectAgent(agent) {
      this.currentAgent = agent;
      this.groupEditOpen = false;
      this.groupDraft = '';
      this.messages = [];
      this.actionLog = [];
      this.sessionCostUsd = 0;
      this.contextPressure = 'low';
      sessionStorage.setItem('rh-active-agent', agent.id);
      this.connectWs(agent.id);
      // Show welcome tips on first use
      if (!localStorage.getItem('rh-chat-tips-seen')) {
        var localMsgId = 0;
        this.messages.push({
          id: ++localMsgId,
          role: 'system',
          text: '**Welcome to RustyHand Chat!**\n\n' +
            '- Type `/` to see available commands\n' +
            '- `/help` shows all commands\n' +
            '- `/think on` enables extended reasoning\n' +
            '- `/context` shows context window usage\n' +
            '- `/verbose off` hides tool details\n' +
            '- Drag & drop files to attach them\n' +
            '- `Ctrl+/` opens the command palette',
          meta: '',
          tools: []
        });
        localStorage.setItem('rh-chat-tips-seen', 'true');
      }
      // Restore draft for this agent
      try {
        var draft = localStorage.getItem('rh-draft-' + agent.id) || '';
        if (draft && !this.inputText) this.inputText = draft;
      } catch(_) {}
      // Focus input after agent selection
      var self = this;
      this.$nextTick(function() {
        var el = document.getElementById('msg-input');
        if (el) {
          el.focus();
          el.style.height = 'auto';
          el.style.height = Math.min(el.scrollHeight, 150) + 'px';
        }
      });
    },

    async loadSession(agentId) {
      var self = this;
      try {
        var data = await RustyHandAPI.get('/api/agents/' + agentId + '/session');
        if (data.messages && data.messages.length) {
          self.messages = data.messages.map(function(m) {
            var role = m.role === 'User' ? 'user' : (m.role === 'System' ? 'system' : 'agent');
            var text = typeof m.content === 'string' ? m.content : JSON.stringify(m.content);
            // Sanitize any raw function-call text from history
            text = self.sanitizeToolText(text);
            // Build tool cards from historical tool data
            var tools = (m.tools || []).map(function(t, idx) {
              return {
                id: (t.name || 'tool') + '-hist-' + idx,
                name: t.name || 'unknown',
                running: false,
                expanded: false,
                input: t.input || '',
                result: t.result || '',
                is_error: !!t.is_error
              };
            });
            return { id: ++msgId, role: role, text: text, meta: '', tools: tools };
          });
          // Initialise context pressure from session data (survives page reload)
          if (data.context_pressure) self.contextPressure = data.context_pressure;
          // Restore star state from localStorage
          var starredIds = new Set(self.starredMessages.map(function(m) { return m.id; }));
          if (starredIds.size) {
            self.messages.forEach(function(m) { if (starredIds.has(m.id)) m._starred = true; });
          }
          self._needsCopyButtonInject = true;
          self.$nextTick(function() { self.scrollToBottom(); });
        }
      } catch(e) {
        self.messages = [];
        self.appendSystemMessage('Failed to load session history: ' + self.normalizeError(e));
      }
    },

    // Multi-session: load session list for current agent
    async loadSessions(agentId) {
      try {
        var data = await RustyHandAPI.get('/api/agents/' + agentId + '/sessions');
        this.sessions = data.sessions || [];
        this.sessionsLoadError = '';
      } catch(e) {
        this.sessions = [];
        this.sessionsLoadError = this.normalizeError(e);
      }
    },

    // Multi-session: create a new session
    createSession() {
      if (!this.currentAgent) return;
      var self = this;
      RustyHandToast.prompt('New Session', 'Enter a name for the new session:', 'Session name (optional)', async function(label) {
        try {
          await RustyHandAPI.post('/api/agents/' + self.currentAgent.id + '/sessions', {
            label: (label || '').trim() || undefined
          });
          await self.loadSessions(self.currentAgent.id);
          await self.loadSession(self.currentAgent.id);
          self.messages = [];
          self.scrollToBottom();
          RustyHandToast.success('New session created');
        } catch(e) {
          RustyHandToast.error('Failed to create session: ' + self.normalizeError(e));
        }
      });
    },

    // Multi-session: switch to an existing session
    async switchSession(sessionId) {
      if (!this.currentAgent) return;
      try {
        await RustyHandAPI.post('/api/agents/' + this.currentAgent.id + '/sessions/' + sessionId + '/switch', {});
        this.messages = [];
        await this.loadSession(this.currentAgent.id);
        await this.loadSessions(this.currentAgent.id);
        // Reconnect WebSocket for new session
        this._wsAgent = null;
        this.connectWs(this.currentAgent.id);
      } catch(e) {
        if (typeof RustyHandToast !== 'undefined') RustyHandToast.error('Failed to switch session: ' + this.normalizeError(e));
      }
    },

    connectWs(agentId) {
      if (this._wsAgent === agentId) return;
      this._wsAgent = agentId;
      this._httpFallbackNotified = false;
      var self = this;

      RustyHandAPI.wsConnect(agentId, {
        onOpen: function() {
          Alpine.store('app').wsConnected = true;
          Alpine.store('app')._wasConnected = true;
          self._httpFallbackNotified = false;
        },
        onMessage: function(data) { self.handleWsMessage(data); },
        onClose: function() {
          // Only called after final reconnect failure
          Alpine.store('app').wsConnected = false;
          self._wsAgent = null;
          // Clear any stuck sending state
          if (self.sending) {
            self.sending = false;
            self.messages = self.messages.filter(function(m) { return !m.thinking; });
          }
        },
        onError: function() {
          Alpine.store('app').wsConnected = false;
        }
      });
    },

    handleWsMessage(data) {
      switch (data.type) {
        case 'connected': break;

        // Legacy thinking event (backward compat)
        case 'thinking':
          if (!this.messages.length || !this.messages[this.messages.length - 1].thinking) {
            var thinkLabel = data.level ? 'Thinking (' + data.level + ')...' : 'Processing...';
            this.messages.push({ id: ++msgId, role: 'agent', text: thinkLabel, meta: '', thinking: true, streaming: true, tools: [] });
            this.scrollToBottom();
            this._resetTypingTimeout();
          } else if (data.level) {
            var lastThink = this.messages[this.messages.length - 1];
            if (lastThink && lastThink.thinking) lastThink.text = 'Thinking (' + data.level + ')...';
          }
          break;

        // New typing lifecycle
        case 'typing':
          if (data.state === 'start') {
            // Mark last user message as delivered
            for (var si = this.messages.length - 1; si >= 0; si--) {
              if (this.messages[si].role === 'user' && this.messages[si].status === 'sending') {
                this.messages[si].status = 'sent';
                break;
              }
            }
            if (!this.messages.length || !this.messages[this.messages.length - 1].thinking) {
              this.messages.push({ id: ++msgId, role: 'agent', text: 'Processing...', meta: '', thinking: true, streaming: true, tools: [] });
              this.scrollToBottom();
            }
            this._resetTypingTimeout();
          } else if (data.state === 'tool') {
            var typingMsg = this.messages.length ? this.messages[this.messages.length - 1] : null;
            if (typingMsg && (typingMsg.thinking || typingMsg.streaming)) {
              typingMsg.text = 'Using ' + escapeHtml(data.tool || 'tool') + '...';
            }
            this._resetTypingTimeout();
          } else if (data.state === 'stop') {
            this._clearTypingTimeout();
          }
          break;

        case 'phase':
          // Show tool/phase progress so the user sees the agent is working
          var phaseMsg = this.messages.length ? this.messages[this.messages.length - 1] : null;
          if (phaseMsg && (phaseMsg.thinking || phaseMsg.streaming)) {
            var detail = data.detail || data.phase || 'Working...';
            // Context warning: show prominently
            if (data.phase === 'context_warning') {
              this.messages.push({ id: ++msgId, role: 'system', text: detail, meta: '', tools: [] });
            } else if (data.phase === 'thinking' && this.thinkingMode === 'stream') {
              // Stream reasoning tokens to a collapsible panel
              if (!phaseMsg._reasoning) phaseMsg._reasoning = '';
              phaseMsg._reasoning += (detail || '') + '\n';
              phaseMsg.text = '<details><summary>Reasoning...</summary>\n\n' + phaseMsg._reasoning + '</details>';
            } else {
              phaseMsg.text = detail;
            }
          }
          this.scrollToBottom();
          break;

        case 'thinking_delta':
          // Reasoning tokens from extended thinking (Anthropic) or DeepSeek R1.
          // Accumulate in _reasoning; keep the thinking indicator text up-to-date.
          var tLast = this.messages.length ? this.messages[this.messages.length - 1] : null;
          if (tLast && tLast.streaming) {
            if (!tLast._reasoning) tLast._reasoning = '';
            tLast._reasoning += data.content;
            if (tLast.thinking) {
              var rTokens = Math.round(tLast._reasoning.length / 4);
              tLast.text = 'Reasoning… (~' + rTokens + ' tokens)';
            }
          }
          this.scrollToBottom();
          break;

        case 'text_delta':
          var last = this.messages.length ? this.messages[this.messages.length - 1] : null;
          if (last && last.streaming) {
            if (last.thinking) { last.text = ''; last.thinking = false; }
            // If we already detected a text-based tool call, skip further text
            if (last._toolTextDetected) break;
            last.text += data.content;
            // Detect function-call patterns streamed as text and convert to tool cards
            var fcIdx = last.text.search(/\w+<\/function[=,>]/);
            if (fcIdx === -1) fcIdx = last.text.search(/<function=\w+>/);
            if (fcIdx !== -1) {
              var fcPart = last.text.substring(fcIdx);
              var toolMatch = fcPart.match(/^(\w+)<\/function/) || fcPart.match(/^<function=(\w+)>/);
              last.text = last.text.substring(0, fcIdx).trim();
              last._toolTextDetected = true;
              if (toolMatch) {
                if (!last.tools) last.tools = [];
                var inputMatch = fcPart.match(/[=,>]\s*(\{[\s\S]*)/);
                last.tools.push({
                  id: toolMatch[1] + '-txt-' + Date.now(),
                  name: toolMatch[1],
                  running: true,
                  expanded: false,
                  input: inputMatch ? inputMatch[1].replace(/<\/function>?\s*$/, '').trim() : '',
                  result: '',
                  is_error: false
                });
              }
            }
            this.tokenCount = Math.round(last.text.length / 4);
            var elapsed = (Date.now() - this.streamStartTime) / 1000;
            if (elapsed > 0.5) this.tokensPerSec = Math.round(this.tokenCount / elapsed);
          } else {
            this.streamStartTime = Date.now();
            this.tokensPerSec = 0;
            this.messages.push({ id: ++msgId, role: 'agent', text: data.content, meta: '', streaming: true, tools: [] });
          }
          this.scrollToBottom();
          break;

        case 'tool_start':
          var lastMsg = this.messages.length ? this.messages[this.messages.length - 1] : null;
          if (lastMsg && lastMsg.streaming) {
            if (!lastMsg.tools) lastMsg.tools = [];
            lastMsg.tools.push({ id: data.tool + '-' + Date.now(), name: data.tool, running: true, expanded: false, input: '', result: '', is_error: false });
          }
          // Activity log
          this.actionLog.push({ ts: Date.now(), tool: data.tool, input: '', result: '', is_error: false, running: true, expanded: false });
          this.scrollToBottom();
          break;

        case 'tool_end':
          // Tool call parsed by LLM — update tool card with input params
          var lastMsg2 = this.messages.length ? this.messages[this.messages.length - 1] : null;
          if (lastMsg2 && lastMsg2.tools) {
            for (var ti = lastMsg2.tools.length - 1; ti >= 0; ti--) {
              if (lastMsg2.tools[ti].name === data.tool && lastMsg2.tools[ti].running) {
                lastMsg2.tools[ti].input = data.input || '';
                break;
              }
            }
          }
          // Activity log — update input on matching entry
          for (var ali = this.actionLog.length - 1; ali >= 0; ali--) {
            if (this.actionLog[ali].tool === data.tool && this.actionLog[ali].running) {
              this.actionLog[ali].input = data.input || '';
              break;
            }
          }
          break;

        case 'tool_result':
          // Tool execution completed — update tool card with result
          var lastMsg3 = this.messages.length ? this.messages[this.messages.length - 1] : null;
          if (lastMsg3 && lastMsg3.tools) {
            for (var ri = lastMsg3.tools.length - 1; ri >= 0; ri--) {
              if (lastMsg3.tools[ri].name === data.tool && lastMsg3.tools[ri].running) {
                lastMsg3.tools[ri].running = false;
                lastMsg3.tools[ri].result = data.result || '';
                lastMsg3.tools[ri].is_error = !!data.is_error;
                // Auto-expand tool card so result is immediately visible
                if (data.result) lastMsg3.tools[ri].expanded = true;
                // Extract image URLs from image_generate or browser_screenshot results
                if ((data.tool === 'image_generate' || data.tool === 'browser_screenshot') && !data.is_error) {
                  try {
                    var parsed = JSON.parse(data.result);
                    if (parsed.image_urls && parsed.image_urls.length) {
                      lastMsg3.tools[ri]._imageUrls = parsed.image_urls;
                    }
                  } catch(e) { /* not JSON */ }
                }
                // Extract audio file path from text_to_speech results
                if (data.tool === 'text_to_speech' && !data.is_error) {
                  try {
                    var ttsResult = JSON.parse(data.result);
                    if (ttsResult.saved_to) {
                      lastMsg3.tools[ri]._audioFile = ttsResult.saved_to;
                      lastMsg3.tools[ri]._audioDuration = ttsResult.duration_estimate_ms;
                    }
                  } catch(e) { /* not JSON */ }
                }
                break;
              }
            }
          }
          // Activity log — update result on matching entry
          for (var rli = this.actionLog.length - 1; rli >= 0; rli--) {
            if (this.actionLog[rli].tool === data.tool && this.actionLog[rli].running) {
              this.actionLog[rli].running = false;
              this.actionLog[rli].result = data.result || '';
              this.actionLog[rli].is_error = !!data.is_error;
              break;
            }
          }
          this.scrollToBottom();
          break;

        case 'response':
          this._clearTypingTimeout();
          // Update context pressure from response
          if (data.context_pressure) {
            this.contextPressure = data.context_pressure;
          }
          // Collect streamed text and tools before removing streaming messages
          // Tools are collected from ALL streaming messages (including thinking)
          // because tools may be added to a thinking message before any text_delta arrives
          var streamedText = '';
          var streamedTools = [];
          this.messages.forEach(function(m) {
            if (m.streaming && m.role === 'agent') {
              if (!m.thinking) streamedText += m.text || '';
              streamedTools = streamedTools.concat(m.tools || []);
            }
          });
          streamedTools.forEach(function(t) {
            t.running = false;
            // Text-detected tool calls (model leaked as text) — mark as not executed
            if (t.id && t.id.indexOf('-txt-') !== -1 && !t.result) {
              t.result = 'Model attempted this call as text (not executed via tool system)';
              t.is_error = true;
            }
          });
          this.messages = this.messages.filter(function(m) { return !m.thinking && !m.streaming; });
          if (data.cost_usd != null) this.sessionCostUsd += data.cost_usd;
          var meta = (data.input_tokens || 0) + ' in / ' + (data.output_tokens || 0) + ' out';
          if (data.cost_usd != null) meta += ' | $' + data.cost_usd.toFixed(4);
          if (data.iterations) meta += ' | ' + data.iterations + ' iter';
          if (data.fallback_model) meta += ' | fallback: ' + data.fallback_model;
          // Use server response if non-empty, otherwise preserve accumulated streamed text
          var finalText = (data.content && data.content.trim()) ? data.content : streamedText;
          // Strip raw function-call JSON that some models leak as text
          finalText = this.sanitizeToolText(finalText);
          // If text is empty but tools ran, show a summary
          if (!finalText.trim() && streamedTools.length) {
            finalText = '';
          }
          this.messages.push({ id: ++msgId, role: 'agent', text: finalText, meta: meta, tools: streamedTools, ts: Date.now() });
          this.sending = false;
          this.tokenCount = 0;
          this.tokensPerSec = 0;
          this._needsCopyButtonInject = true;
          this.scrollToBottom();
          var self3 = this;
          this.$nextTick(function() {
            var el = document.getElementById('msg-input'); if (el) el.focus();
            self3._processQueue();
          });
          break;

        case 'silent_complete':
          // Agent intentionally chose not to reply (NO_REPLY)
          this._clearTypingTimeout();
          this.messages = this.messages.filter(function(m) { return !m.thinking && !m.streaming; });
          this.sending = false;
          this.tokenCount = 0;
          // No message bubble added — the agent was silent
          var selfSilent = this;
          this.$nextTick(function() { selfSilent._processQueue(); });
          break;

        case 'error':
          this._clearTypingTimeout();
          this.messages = this.messages.filter(function(m) { return !m.thinking && !m.streaming; });
          this.messages.push({ id: ++msgId, role: 'system', text: 'Error: ' + escapeHtml(data.content || ''), meta: '', tools: [], ts: Date.now(), retryable: true });
          this.sending = false;
          this.tokenCount = 0;
          this.scrollToBottom();
          var self2 = this;
          this.$nextTick(function() {
            var el = document.getElementById('msg-input'); if (el) el.focus();
            self2._processQueue();
          });
          break;

        case 'agents_updated':
          if (data.agents) {
            Alpine.store('app').agents = data.agents;
            Alpine.store('app').agentCount = data.agents.length;
          }
          break;

        case 'command_result':
          // Update context pressure if included in command result
          if (data.context_pressure) {
            this.contextPressure = data.context_pressure;
          }
          // Retry: pop last agent message from local state, re-run with returned message
          if (data.command === 'retry' && data.retry_message != null) {
            // Remove trailing assistant messages from view
            while (this.messages.length && this.messages[this.messages.length - 1].role !== 'user') {
              this.messages.pop();
            }
            // Also pop the last user message since it'll be re-added by _sendPayload
            if (this.messages.length && this.messages[this.messages.length - 1].role === 'user') {
              this.messages.pop();
            }
            // Re-send via normal path (adds user bubble + re-runs agent)
            this._sendPayload(data.retry_message, [], []);
            break;
          }
          this.messages.push({ id: ++msgId, role: 'system', text: data.message || 'Command executed.', meta: '', tools: [] });
          this.scrollToBottom();
          break;

        case 'canvas':
          // Agent presented an interactive canvas — render it in an iframe sandbox
          var canvasHtml = '<div class="canvas-panel" style="border:1px solid var(--border);border-radius:8px;margin:8px 0;overflow:hidden;">';
          canvasHtml += '<div style="padding:6px 12px;background:var(--surface);border-bottom:1px solid var(--border);font-size:0.85em;display:flex;justify-content:space-between;align-items:center;">';
          canvasHtml += '<span>' + escapeHtml(data.title || 'Canvas') + '</span>';
          canvasHtml += '<span style="opacity:0.5;font-size:0.8em;">' + escapeHtml((data.canvas_id || '').substring(0, 8)) + '</span></div>';
          canvasHtml += '<iframe sandbox="allow-scripts" srcdoc="' + (data.html || '').replace(/"/g, '&quot;') + '" ';
          canvasHtml += 'style="width:100%;min-height:300px;border:none;background:#fff;" loading="lazy"></iframe></div>';
          this.messages.push({ id: ++msgId, role: 'agent', text: canvasHtml, meta: 'canvas', isHtml: true, tools: [] });
          this.scrollToBottom();
          break;

        case 'pong': break;
      }
    },

    // Format timestamp for display
    formatTime: function(ts) {
      if (!ts) return '';
      var d = new Date(ts);
      var h = d.getHours();
      var m = d.getMinutes();
      var ampm = h >= 12 ? 'PM' : 'AM';
      h = h % 12 || 12;
      return h + ':' + (m < 10 ? '0' : '') + m + ' ' + ampm;
    },

    // Copy raw message text (preserves markdown) to clipboard
    copyMessage: function(msg) {
      var text = msg.text || '';
      navigator.clipboard.writeText(text).then(function() {
        msg._copied = true;
        setTimeout(function() { msg._copied = false; }, 1500);
      }).catch(function() {
        if (typeof RustyHandToast !== 'undefined') RustyHandToast.error('Clipboard copy failed');
      });
    },

    // Delete a message from the local chat (frontend-only)
    deleteMessage: function(msg) {
      if (msg._confirmDelete) {
        // Second click = confirm
        var self = this;
        self.messages = self.messages.filter(function(m) { return m.id !== msg.id; });
        if (typeof RustyHandToast !== 'undefined') RustyHandToast.success('Message removed');
      } else {
        // First click = show inline confirmation
        msg._confirmDelete = true;
        setTimeout(function() { msg._confirmDelete = false; }, 3000);
      }
    },

    // Inject copy buttons into code blocks after markdown renders
    injectCodeCopyButtons: function() {
      var self = this;
      self.$nextTick(function() {
        var container = document.getElementById('messages');
        if (!container) return;
        var blocks = container.querySelectorAll('.message-bubble pre');
        blocks.forEach(function(pre) {
          // Skip if already has a copy button
          if (pre.querySelector('.code-copy-btn')) return;
          // Ensure pre is relatively positioned for absolute button
          pre.style.position = 'relative';
          var btn = document.createElement('button');
          btn.className = 'code-copy-btn';
          btn.textContent = 'Copy';
          btn.addEventListener('click', function(e) {
            e.stopPropagation();
            var code = pre.querySelector('code');
            var text = code ? code.textContent : pre.textContent;
            navigator.clipboard.writeText(text).then(function() {
              btn.textContent = 'Copied!';
              btn.classList.add('copied');
              setTimeout(function() {
                btn.textContent = 'Copy';
                btn.classList.remove('copied');
              }, 1500);
            }).catch(function() {
              if (typeof RustyHandToast !== 'undefined') RustyHandToast.error('Clipboard copy failed');
            });
          });
          pre.appendChild(btn);
        });
      });
    },

    // Process queued messages after current response completes
    _processQueue: function() {
      if (!this.messageQueue.length || this.sending) return;
      var next = this.messageQueue.shift();
      this._sendPayload(next.text, next.files, next.images);
    },

    async sendMessage() {
      if (!this.currentAgent || (!this.inputText.trim() && !this.attachments.length)) return;
      var text = this.inputText.trim();

      // Handle slash commands
      if (text.startsWith('/') && !this.attachments.length) {
        var cmd = text.split(' ')[0].toLowerCase();
        var cmdArgs = text.substring(cmd.length).trim();
        var matched = this.slashCommands.find(function(c) { return c.cmd === cmd; });
        if (matched) {
          this.executeSlashCommand(matched.cmd, cmdArgs);
          return;
        }
      }

      this.inputText = '';
      // Clear draft on send
      if (this.currentAgent) { try { localStorage.removeItem('rh-draft-' + this.currentAgent.id); } catch(_) {} }

      // Reset textarea height to single line
      var ta = document.getElementById('msg-input');
      if (ta) ta.style.height = '';

      // Upload attachments first if any
      var fileRefs = [];
      var uploadedFiles = [];
      if (this.attachments.length) {
        for (var i = 0; i < this.attachments.length; i++) {
          var att = this.attachments[i];
          att.uploading = true;
          try {
            var uploadRes = await RustyHandAPI.upload(this.currentAgent.id, att.file);
            fileRefs.push('[File: ' + att.file.name + ']');
            uploadedFiles.push({ file_id: uploadRes.file_id, filename: uploadRes.filename, content_type: uploadRes.content_type });
          } catch(e) {
            RustyHandToast.error('Failed to upload ' + att.file.name);
            fileRefs.push('[File: ' + att.file.name + ' (upload failed)]');
          }
          att.uploading = false;
        }
        // Clean up previews
        for (var j = 0; j < this.attachments.length; j++) {
          if (this.attachments[j].preview) URL.revokeObjectURL(this.attachments[j].preview);
        }
        this.attachments = [];
      }

      // Build final message text
      var finalText = text;
      if (fileRefs.length) {
        finalText = (text ? text + '\n' : '') + fileRefs.join('\n');
      }

      // Collect image references for inline rendering
      var msgImages = uploadedFiles.filter(function(f) { return f.content_type && f.content_type.startsWith('image/'); });

      // Always show user message immediately with sending status
      this.messages.push({ id: ++msgId, role: 'user', text: finalText, meta: '', tools: [], images: msgImages, ts: Date.now(), status: 'sending' });
      this.scrollToBottom();
      localStorage.setItem('rh-first-msg', 'true');

      // If already streaming, queue this message
      if (this.sending) {
        this.messageQueue.push({ text: finalText, files: uploadedFiles, images: msgImages });
        return;
      }

      this._sendPayload(finalText, uploadedFiles, msgImages);
    },

    async _sendPayload(finalText, uploadedFiles, msgImages) {
      this.sending = true;

      // Try WebSocket first
      var wsPayload = { type: 'message', content: finalText };
      if (uploadedFiles && uploadedFiles.length) wsPayload.attachments = uploadedFiles;
      if (RustyHandAPI.wsSend(wsPayload)) {
        this.messages.push({ id: ++msgId, role: 'agent', text: '', meta: '', thinking: true, streaming: true, tools: [], ts: Date.now() });
        this.scrollToBottom();
        return;
      }

      // HTTP fallback — only notify once per disconnection episode
      if (!RustyHandAPI.isWsConnected() && !this._httpFallbackNotified) {
        RustyHandToast.info('Using HTTP mode (no streaming)');
        this._httpFallbackNotified = true;
      }
      this.messages.push({ id: ++msgId, role: 'agent', text: '', meta: '', thinking: true, tools: [], ts: Date.now() });
      this.scrollToBottom();

      try {
        var httpBody = { message: finalText };
        if (uploadedFiles && uploadedFiles.length) httpBody.attachments = uploadedFiles;
        var res = await RustyHandAPI.post('/api/agents/' + this.currentAgent.id + '/message', httpBody);
        this.messages = this.messages.filter(function(m) { return !m.thinking; });
        var httpMeta = (res.input_tokens || 0) + ' in / ' + (res.output_tokens || 0) + ' out';
        if (res.cost_usd != null) httpMeta += ' | $' + res.cost_usd.toFixed(4);
        if (res.iterations) httpMeta += ' | ' + res.iterations + ' iter';
        this.messages.push({ id: ++msgId, role: 'agent', text: res.response, meta: httpMeta, tools: [], ts: Date.now() });
      } catch(e) {
        this.messages = this.messages.filter(function(m) { return !m.thinking; });
        this.messages.push({ id: ++msgId, role: 'system', text: 'Error: ' + e.message, meta: '', tools: [], ts: Date.now() });
      }
      this.sending = false;
      this._needsCopyButtonInject = true;
      this.scrollToBottom();
      // Process next queued message
      var self = this;
      this.$nextTick(function() {
        var el = document.getElementById('msg-input'); if (el) el.focus();
        self._processQueue();
      });
    },

    // Stop the current agent run
    stopAgent: function() {
      if (!this.currentAgent) return;
      var self = this;
      RustyHandAPI.post('/api/agents/' + this.currentAgent.id + '/stop', {}).then(function(res) {
        self.messages.push({ id: ++msgId, role: 'system', text: res.message || 'Run cancelled', meta: '', tools: [], ts: Date.now() });
        self.sending = false;
        self.scrollToBottom();
        self.$nextTick(function() { self._processQueue(); });
      }).catch(function(e) { RustyHandToast.error('Stop failed: ' + e.message); });
    },

    killAgent() {
      if (!this.currentAgent) return;
      var self = this;
      var name = this.currentAgent.name;
      RustyHandToast.confirm('Stop Agent', 'Stop agent "' + name + '"? The agent will be shut down.', async function() {
        try {
          await RustyHandAPI.del('/api/agents/' + self.currentAgent.id);
          RustyHandAPI.wsDisconnect();
          self._wsAgent = null;
          self.currentAgent = null;
          self.messages = [];
          RustyHandToast.success('Agent "' + name + '" stopped');
          Alpine.store('app').refreshAgents();
        } catch(e) {
          RustyHandToast.error('Failed to stop agent: ' + e.message);
        }
      });
    },

    scrollToBottom() {
      var self = this;
      var el = document.getElementById('messages');
      if (el) self.$nextTick(function() {
        el.scrollTop = el.scrollHeight;
        self.isScrolledUp = false;
        if (self._needsCopyButtonInject) {
          self._needsCopyButtonInject = false;
          self.injectCodeCopyButtons();
        }
      });
    },

    checkScroll() {
      var el = document.getElementById('messages');
      if (!el) return;
      this.isScrolledUp = (el.scrollTop + el.clientHeight) < (el.scrollHeight - 100);
    },

    retry(msg) {
      // Find the last user message before this error
      var idx = this.messages.indexOf(msg);
      var lastUserMsg = null;
      for (var i = idx - 1; i >= 0; i--) {
        if (this.messages[i].role === 'user') {
          lastUserMsg = this.messages[i].text;
          break;
        }
      }
      if (lastUserMsg) {
        // Remove the error message
        this.messages = this.messages.filter(function(m) { return m !== msg; });
        // Re-send
        this._sendPayload(lastUserMsg, [], []);
      }
    },

    exportChat() {
      if (!this.messages.length) { RustyHandToast.info('No messages to export'); return; }
      var name = this.currentAgent ? this.currentAgent.name : 'chat';
      var md = '# Chat with ' + name + '\n\n';
      for (var i = 0; i < this.messages.length; i++) {
        var m = this.messages[i];
        var role = m.role === 'user' ? 'You' : m.role === 'agent' ? name : 'System';
        var time = m.ts ? new Date(m.ts).toLocaleString() : '';
        md += '### ' + role + (time ? ' (' + time + ')' : '') + '\n\n';
        if (m._reasoning) {
          md += '<details><summary>Reasoning</summary>\n\n' + m._reasoning + '\n\n</details>\n\n';
        }
        md += (m.text || '') + '\n\n';
        if (m.tools && m.tools.length) {
          for (var j = 0; j < m.tools.length; j++) {
            md += '> Tool: ' + m.tools[j].name + '\n';
          }
          md += '\n';
        }
      }
      var blob = new Blob([md], { type: 'text/markdown' });
      var url = URL.createObjectURL(blob);
      var a = document.createElement('a');
      a.href = url;
      a.download = name + '-chat-' + new Date().toISOString().slice(0, 10) + '.md';
      a.click();
      URL.revokeObjectURL(url);
      RustyHandToast.success('Chat exported');
    },

    addFiles(files) {
      var self = this;
      var allowed = ['image/png', 'image/jpeg', 'image/gif', 'image/webp', 'text/plain', 'application/pdf',
                      'text/markdown', 'application/json', 'text/csv'];
      var allowedExts = ['.txt', '.pdf', '.md', '.json', '.csv'];
      for (var i = 0; i < files.length; i++) {
        var file = files[i];
        if (file.size > 10 * 1024 * 1024) {
          RustyHandToast.warn('File "' + file.name + '" exceeds 10MB limit');
          continue;
        }
        var typeOk = allowed.indexOf(file.type) !== -1;
        if (!typeOk) {
          var ext = file.name.lastIndexOf('.') !== -1 ? file.name.substring(file.name.lastIndexOf('.')).toLowerCase() : '';
          typeOk = allowedExts.indexOf(ext) !== -1 || file.type.startsWith('image/');
        }
        if (!typeOk) {
          RustyHandToast.warn('File type not supported: ' + file.name);
          continue;
        }
        var preview = null;
        if (file.type.startsWith('image/')) {
          preview = URL.createObjectURL(file);
        }
        self.attachments.push({ file: file, preview: preview, uploading: false });
      }
    },

    removeAttachment(idx) {
      var att = this.attachments[idx];
      if (att && att.preview) URL.revokeObjectURL(att.preview);
      this.attachments.splice(idx, 1);
    },

    handleDrop(e) {
      e.preventDefault();
      if (e.dataTransfer && e.dataTransfer.files && e.dataTransfer.files.length) {
        this.addFiles(e.dataTransfer.files);
      }
    },

    isGrouped(idx) {
      if (idx === 0) return false;
      var prev = this.messages[idx - 1];
      var curr = this.messages[idx];
      return prev && curr && prev.role === curr.role && !curr.thinking && !prev.thinking;
    },

    // Strip raw function-call text that some models (Llama, Groq, etc.) leak into output.
    // These models don't use proper tool_use blocks — they output function calls as plain text.
    sanitizeToolText: function(text) {
      if (!text) return text;
      // Strip <think>...</think> blocks from reasoning models
      text = text.replace(/<think>[\s\S]*?<\/think>\s*/g, '');
      text = text.replace(/<think>[\s\S]*$/g, '');
      // Pattern: tool_name</function={"key":"value"} or tool_name</function,{...}
      text = text.replace(/\s*\w+<\/function[=,]?\s*\{[\s\S]*$/gm, '');
      // Pattern: <function=tool_name>{...}</function>
      text = text.replace(/<function=\w+>[\s\S]*?<\/function>/g, '');
      // Pattern: tool_name{"type":"function",...}
      text = text.replace(/\s*\w+\{"type"\s*:\s*"function"[\s\S]*$/gm, '');
      // Pattern: lone </function...> tags
      text = text.replace(/<\/function[^>]*>/g, '');
      // Pattern: <|python_tag|> or similar special tokens
      text = text.replace(/<\|[\w_]+\|>/g, '');
      return text.trim();
    },

    formatToolJson: function(text) {
      if (!text) return '';
      try { return JSON.stringify(JSON.parse(text), null, 2); }
      catch(e) { return text; }
    },

    formatLogTime: function(ts) {
      if (!ts) return '';
      var d = new Date(ts);
      return d.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit', second: '2-digit' });
    },

    // Voice: start recording
    startRecording: async function() {
      if (this.recording) return;
      try {
        var stream = await navigator.mediaDevices.getUserMedia({ audio: true });
        var mimeType = MediaRecorder.isTypeSupported('audio/webm;codecs=opus') ? 'audio/webm;codecs=opus' :
                       MediaRecorder.isTypeSupported('audio/webm') ? 'audio/webm' : 'audio/ogg';
        this._audioChunks = [];
        this._mediaRecorder = new MediaRecorder(stream, { mimeType: mimeType });
        var self = this;
        this._mediaRecorder.ondataavailable = function(e) {
          if (e.data.size > 0) self._audioChunks.push(e.data);
        };
        this._mediaRecorder.onstop = function() {
          stream.getTracks().forEach(function(t) { t.stop(); });
          self._handleRecordingComplete();
        };
        this._mediaRecorder.start(250);
        this.recording = true;
        this.recordingTime = 0;
        this._recordingTimer = setInterval(function() { self.recordingTime++; }, 1000);
      } catch(e) {
        if (typeof RustyHandToast !== 'undefined') RustyHandToast.error('Microphone access denied');
      }
    },

    // Voice: stop recording
    stopRecording: function() {
      if (!this.recording || !this._mediaRecorder) return;
      this._mediaRecorder.stop();
      this.recording = false;
      if (this._recordingTimer) { clearInterval(this._recordingTimer); this._recordingTimer = null; }
    },

    // Voice: handle completed recording — upload and transcribe
    _handleRecordingComplete: async function() {
      if (!this._audioChunks.length || !this.currentAgent) return;
      var blob = new Blob(this._audioChunks, { type: this._audioChunks[0].type || 'audio/webm' });
      this._audioChunks = [];
      if (blob.size < 100) return; // too small

      // Show a temporary "Transcribing..." message
      this.messages.push({ id: ++msgId, role: 'system', text: 'Transcribing audio...', thinking: true, ts: Date.now(), tools: [] });
      this.scrollToBottom();

      try {
        // Upload audio file
        var ext = blob.type.includes('webm') ? 'webm' : blob.type.includes('ogg') ? 'ogg' : 'mp3';
        var file = new File([blob], 'voice_' + Date.now() + '.' + ext, { type: blob.type });
        var upload = await RustyHandAPI.upload(this.currentAgent.id, file);

        // Remove the "Transcribing..." message
        this.messages = this.messages.filter(function(m) { return !m.thinking || m.role !== 'system'; });

        // Use server-side transcription if available, otherwise fall back to placeholder
        var text = (upload.transcription && upload.transcription.trim())
          ? upload.transcription.trim()
          : '[Voice message - audio: ' + upload.filename + ']';
        this._sendPayload(text, [upload], []);
      } catch(e) {
        this.messages = this.messages.filter(function(m) { return !m.thinking || m.role !== 'system'; });
        if (typeof RustyHandToast !== 'undefined') RustyHandToast.error('Failed to upload audio: ' + (e.message || 'unknown error'));
      }
    },

    // Voice: format recording time as MM:SS
    formatRecordingTime: function() {
      var m = Math.floor(this.recordingTime / 60);
      var s = this.recordingTime % 60;
      return (m < 10 ? '0' : '') + m + ':' + (s < 10 ? '0' : '') + s;
    },

    // Search: toggle open/close
    // ── Starred messages (localStorage, per agent) ──
    _starsKey: function() {
      return 'rh-stars-' + (this.currentAgent ? this.currentAgent.id : 'global');
    },
    loadStars: function() {
      try { this.starredMessages = JSON.parse(localStorage.getItem(this._starsKey()) || '[]'); }
      catch(_) { this.starredMessages = []; }
    },
    toggleStar: function(msg) {
      var idx = this.starredMessages.findIndex(function(m) { return m.id === msg.id; });
      if (idx >= 0) {
        this.starredMessages.splice(idx, 1);
        msg._starred = false;
      } else {
        this.starredMessages.push(Object.assign({}, msg));
        msg._starred = true;
      }
      try { localStorage.setItem(this._starsKey(), JSON.stringify(this.starredMessages)); } catch(_) {}
    },
    clearStars: function() {
      var self = this;
      this.messages.forEach(function(m) { m._starred = false; });
      this.starredMessages = [];
      try { localStorage.removeItem(self._starsKey()); } catch(_) {}
    },
    scrollToStarred: function(starred) {
      var msg = this.messages.find(function(m) { return m.id === starred.id; });
      if (!msg) return;
      this.$nextTick(function() {
        var el = document.querySelector('[data-msg-id="' + starred.id + '"]');
        if (el) { el.scrollIntoView({ behavior: 'smooth', block: 'center' }); el.classList.add('msg-highlight'); setTimeout(function() { el.classList.remove('msg-highlight'); }, 1500); }
      });
    },

    // ↑ in empty input recalls the last user message for editing
    recallLastMessage: function() {
      var last = null;
      for (var i = this.messages.length - 1; i >= 0; i--) {
        if (this.messages[i].role === 'user') { last = this.messages[i]; break; }
      }
      if (!last) return;
      this.inputText = last.text || '';
      this.$nextTick(function() {
        var el = document.getElementById('msg-input');
        if (el) { el.style.height = 'auto'; el.style.height = Math.min(el.scrollHeight, 150) + 'px'; el.setSelectionRange(el.value.length, el.value.length); }
      });
    },

    toggleSearch: function() {
      this.searchOpen = !this.searchOpen;
      if (this.searchOpen) {
        var self = this;
        this.$nextTick(function() {
          var el = document.getElementById('chat-search-input');
          if (el) el.focus();
        });
      } else {
        this.searchQuery = '';
      }
    },

    // Search: filter messages by query
    get filteredMessages() {
      if (!this.searchQuery.trim()) return this.messages;
      var q = this.searchQuery.toLowerCase();
      return this.messages.filter(function(m) {
        return (m.text && m.text.toLowerCase().indexOf(q) !== -1) ||
               (m.tools && m.tools.some(function(t) { return t.name.toLowerCase().indexOf(q) !== -1; }));
      });
    },

    // Search: highlight matched text in a string
    highlightSearch: function(html) {
      if (!this.searchQuery.trim() || !html) return html;
      var q = this.searchQuery.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
      var regex = new RegExp('(' + q + ')(?![^<]*>)', 'gi');
      return html.replace(regex, '<mark style="background:var(--warning);color:var(--bg);border-radius:2px;padding:0 2px">$1</mark>');
    },

    renderMarkdown: renderMarkdown,
    escapeHtml: escapeHtml
  };
}
