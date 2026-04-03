// RustyHand Agents Page — Multi-step spawn wizard, detail view with tabs, file editor, personality presets
'use strict';

function agentsPage() {
  return {
    tab: 'agents',
    // -- Agents state --
    showSpawnModal: false,
    showDetailModal: false,
    detailAgent: null,
    spawnMode: 'wizard',
    spawning: false,
    spawnToml: '',
    loading: true,
    loadError: '',
    spawnForm: {
      name: '',
      group: '',
      provider: '',
      model: '',
      systemPrompt: 'You are a helpful assistant.',
      profile: 'full',
      caps: { memory_read: true, memory_write: true, network: false, shell: false, agent_spawn: false },
      autonomous: false,
      cronExpr: '',
      cronMessage: 'Run your scheduled task now'
    },

    cronPresets: [
      { label: 'Every 5 min', expr: '*/5 * * * *' },
      { label: 'Hourly', expr: '0 * * * *' },
      { label: 'Daily 9am', expr: '0 9 * * *' },
      { label: 'Weekdays 9am', expr: '0 9 * * 1-5' },
      { label: 'Weekly Mon', expr: '0 9 * * 1' },
    ],

    // -- Multi-step wizard state --
    spawnStep: 1,
    spawnIdentity: { emoji: '', color: '#FF5C00', archetype: '' },
    selectedPreset: '',
    soulContent: '',
    emojiOptions: [
      '\u{1F916}', '\u{1F4BB}', '\u{1F50D}', '\u{270D}\uFE0F', '\u{1F4CA}', '\u{1F6E0}\uFE0F',
      '\u{1F4AC}', '\u{1F393}', '\u{1F310}', '\u{1F512}', '\u{26A1}', '\u{1F680}',
      '\u{1F9EA}', '\u{1F3AF}', '\u{1F4D6}', '\u{1F9D1}\u200D\u{1F4BB}', '\u{1F4E7}', '\u{1F3E2}',
      '\u{2764}\uFE0F', '\u{1F31F}', '\u{1F527}', '\u{1F4DD}', '\u{1F4A1}', '\u{1F3A8}'
    ],
    archetypeOptions: ['Assistant', 'Researcher', 'Coder', 'Writer', 'DevOps', 'Support', 'Analyst', 'Custom'],
    personalityPresets: [
      { id: 'professional', label: 'Professional', soul: 'Communicate in a clear, professional tone. Be direct and structured. Use formal language and data-driven reasoning. Prioritize accuracy over personality.' },
      { id: 'friendly', label: 'Friendly', soul: 'Be warm, approachable, and conversational. Use casual language and show genuine interest in the user. Add personality to your responses while staying helpful.' },
      { id: 'technical', label: 'Technical', soul: 'Focus on technical accuracy and depth. Use precise terminology. Show your work and reasoning. Prefer code examples and structured explanations.' },
      { id: 'creative', label: 'Creative', soul: 'Be imaginative and expressive. Use vivid language, analogies, and unexpected connections. Encourage creative thinking and explore multiple perspectives.' },
      { id: 'concise', label: 'Concise', soul: 'Be extremely brief and to the point. No filler, no pleasantries. Answer in the fewest words possible while remaining accurate and complete.' },
      { id: 'mentor', label: 'Mentor', soul: 'Be patient and encouraging like a great teacher. Break down complex topics step by step. Ask guiding questions. Celebrate progress and build confidence.' }
    ],

    // -- Detail modal tabs --
    detailTab: 'info',
    agentFiles: [],
    editingFile: null,
    fileContent: '',
    fileSaving: false,
    filesLoading: false,
    configForm: {},
    configSaving: false,

    // -- Model change in detail modal --
    editingModel: false,
    modelChangeProvider: '',
    modelChangeModel: '',
    modelChangeSaving: false,

    openModelEditor() {
      this.editingModel = true;
      this.modelChangeProvider = (this.detailAgent && this.detailAgent.model_provider) || '';
      this.modelChangeModel = (this.detailAgent && this.detailAgent.model_name) || '';
      this.loadWizardData();
    },

    get modelChangeModels() {
      var provider = this.modelChangeProvider;
      var filtered = this.wizardModelsRaw.filter(function(m) { return m.provider === provider; });
      var tierOrder = { frontier: 0, smart: 1, balanced: 2, fast: 3, local: 4 };
      filtered.sort(function(a, b) {
        var aO = tierOrder[a.tier] !== undefined ? tierOrder[a.tier] : 5;
        var bO = tierOrder[b.tier] !== undefined ? tierOrder[b.tier] : 5;
        if (aO !== bO) return aO - bO;
        return (a.display_name || a.id).localeCompare(b.display_name || b.id);
      });
      return filtered;
    },

    get modelChangeInfo() {
      var modelId = this.modelChangeModel;
      if (!modelId) return null;
      return this.wizardModelsRaw.find(function(m) { return m.id === modelId; }) || null;
    },

    selectModelChangeProvider(providerId) {
      this.modelChangeProvider = providerId;
      var models = this.modelChangeModels;
      if (models.length > 0) {
        this.modelChangeModel = models[0].id;
      }
    },

    async saveModelChange() {
      if (!this.detailAgent || !this.modelChangeModel) return;
      this.modelChangeSaving = true;
      try {
        await RustyHandAPI.put('/api/agents/' + this.detailAgent.id + '/model', { model: this.modelChangeModel });
        RustyHandToast.success('Model changed to ' + this.modelChangeModel);
        this.editingModel = false;
        await Alpine.store('app').refreshAgents();
        // Update detailAgent in place
        var updated = Alpine.store('app').agents.find(function(a) { return a.id === this.detailAgent.id; }.bind(this));
        if (updated) this.detailAgent = updated;
      } catch(e) {
        RustyHandToast.error('Failed to change model: ' + e.message);
      }
      this.modelChangeSaving = false;
    },

    // -- Templates state --
    tplTemplates: [],
    tplProviders: [],
    tplLoading: false,
    tplLoadError: '',
    selectedCategory: 'All',
    searchQuery: '',

    // ── Dynamic model/provider data for wizard ──
    wizardModelsRaw: [],
    wizardProvidersRaw: [],
    wizardDataLoaded: false,

    // ── Tool Preview in Spawn Modal ──
    spawnProfiles: [],
    spawnProfilesLoaded: false,
    spawnProfilesError: '',
    async loadSpawnProfiles() {
      if (this.spawnProfilesLoaded) return;
      try {
        var data = await RustyHandAPI.get('/api/profiles');
        this.spawnProfiles = data.profiles || [];
        this.spawnProfilesError = '';
        this.spawnProfilesLoaded = true;
      } catch(e) {
        this.spawnProfiles = [];
        this.spawnProfilesError = e.message || 'Could not load tool profiles.';
      }
    },
    get selectedProfileTools() {
      var pname = this.spawnForm.profile;
      var match = this.spawnProfiles.find(function(p) { return p.name === pname; });
      if (match && match.tools) return match.tools.slice(0, 15);
      return [];
    },

    async loadWizardData() {
      if (this.wizardDataLoaded) return;
      try {
        var results = await Promise.all([
          RustyHandAPI.get('/api/models?available=true'),
          RustyHandAPI.get('/api/providers')
        ]);
        this.wizardModelsRaw = results[0].models || [];
        this.wizardProvidersRaw = results[1].providers || [];
        this.wizardDataLoaded = true;
        // Auto-select: prefer default provider from config, then first available
        var avail = this.wizardProviders;
        var currentValid = avail.find(function(p) { return p.id === this.spawnForm.provider; }.bind(this));
        if (!currentValid && avail.length > 0) {
          this.selectProvider(avail[0].id);
        } else if (currentValid) {
          // Re-trigger model selection to pick the best model for this provider
          this.selectProvider(this.spawnForm.provider);
        }
      } catch(e) {
        console.warn('Failed to load wizard data:', e.message);
      }
    },

    get wizardProviders() {
      var defaultProvider = Alpine.store('app').defaultProvider;
      return this.wizardProvidersRaw
        .filter(function(p) {
          // Only show providers that have a key configured, or local ones that are reachable
          if (p.auth_status === 'configured' || p.auth_status === 'not_required') {
            if (p.is_local) return p.reachable;
            return true;
          }
          return false;
        })
        .sort(function(a, b) {
          // Default provider from config always first
          var aDefault = a.id === defaultProvider ? 1 : 0;
          var bDefault = b.id === defaultProvider ? 1 : 0;
          if (aDefault !== bDefault) return bDefault - aDefault;
          // Then by model count descending
          return (b.model_count || 0) - (a.model_count || 0);
        });
    },

    get wizardModels() {
      var provider = this.spawnForm.provider;
      var filtered = this.wizardModelsRaw.filter(function(m) { return m.provider === provider; });
      var tierOrder = { frontier: 0, smart: 1, balanced: 2, fast: 3, local: 4 };
      filtered.sort(function(a, b) {
        var aO = tierOrder[a.tier] !== undefined ? tierOrder[a.tier] : 5;
        var bO = tierOrder[b.tier] !== undefined ? tierOrder[b.tier] : 5;
        if (aO !== bO) return aO - bO;
        return (a.display_name || a.id).localeCompare(b.display_name || b.id);
      });
      return filtered;
    },

    get selectedModelInfo() {
      var modelId = this.spawnForm.model;
      if (!modelId) return null;
      return this.wizardModelsRaw.find(function(m) { return m.id === modelId; }) || null;
    },

    tierLabel(tier) {
      var labels = { frontier: 'Frontier', smart: 'Smart', balanced: 'Balanced', fast: 'Fast', local: 'Local' };
      return labels[tier] || tier;
    },

    tierColor(tier) {
      var colors = { frontier: '#a855f7', smart: '#3b82f6', balanced: '#22c55e', fast: '#eab308', local: '#6b7280' };
      return colors[tier] || '#6b7280';
    },

    formatCtx(n) {
      if (!n) return '?';
      if (n >= 1000000) return (n / 1000000).toFixed(1) + 'M';
      return Math.round(n / 1000) + 'K';
    },

    formatCost(n) {
      if (n === undefined || n === null) return '?';
      if (n === 0) return 'Free';
      if (n < 0.01) return '<$0.01';
      return '$' + n.toFixed(2);
    },

    selectProvider(providerId) {
      this.spawnForm.provider = providerId;
      var models = this.wizardModels;
      if (models.length > 0) {
        // Always pick the top model (frontier tier first by sort order)
        this.spawnForm.model = models[0].id;
      }
    },

    // -- Delegated to store --
    get agents() { return Alpine.store('app').agents; },
    get builtinTemplates() { return Alpine.store('app').builtinTemplates; },
    get filteredAgents() { return Alpine.store('app').filteredAgents; },
    get groupedAgents() { return Alpine.store('app').groupedAgents; },
    get runningCount() { return Alpine.store('app').runningCount; },
    get groupCount() {
      var seen = {};
      this.agents.forEach(function(agent) {
        seen[Alpine.store('app').normalizeAgentGroupLabel(agent.group)] = true;
      });
      return Object.keys(seen).length;
    },

    get stoppedCount() {
      return this.agents.filter(function(a) { return a.state !== 'Running'; }).length;
    },

    get recentAgents() {
      return this.agents
        .slice()
        .sort(function(a, b) {
          var aTime = new Date(a.last_activity || a.created_at || 0).getTime();
          var bTime = new Date(b.last_activity || b.created_at || 0).getTime();
          return bTime - aTime;
        })
        .slice(0, 6);
    },

    get providerMix() {
      var counts = {};
      for (var i = 0; i < this.agents.length; i++) {
        var provider = this.agents[i].model_provider || 'unknown';
        counts[provider] = (counts[provider] || 0) + 1;
      }
      return Object.keys(counts)
        .sort(function(a, b) { return counts[b] - counts[a]; })
        .map(function(provider) {
          return { provider: provider, count: counts[provider] };
        });
    },

    get templatePicks() {
      return this.builtinTemplates.slice(0, 6);
    },

    agentLastSeen(agent) {
      return timeAgo(agent && (agent.last_activity || agent.created_at));
    },

    syncDetailAgent() {
      if (!this.detailAgent) return;
      var updated = Alpine.store('app').agents.find(function(agent) {
        return agent.id === this.detailAgent.id;
      }.bind(this));
      if (updated) this.detailAgent = updated;
    },

    // -- Templates computed --
    get categories() {
      var cats = { 'All': true };
      Alpine.store('app').builtinTemplates.forEach(function(t) { cats[t.category] = true; });
      this.tplTemplates.forEach(function(t) { if (t.category) cats[t.category] = true; });
      return Object.keys(cats);
    },

    get filteredBuiltins() {
      var self = this;
      return Alpine.store('app').builtinTemplates.filter(function(t) {
        if (self.selectedCategory !== 'All' && t.category !== self.selectedCategory) return false;
        if (self.searchQuery) {
          var q = self.searchQuery.toLowerCase();
          if (t.name.toLowerCase().indexOf(q) === -1 &&
              t.description.toLowerCase().indexOf(q) === -1) return false;
        }
        return true;
      });
    },

    get filteredCustom() {
      var self = this;
      return this.tplTemplates.filter(function(t) {
        if (self.searchQuery) {
          var q = self.searchQuery.toLowerCase();
          if ((t.name || '').toLowerCase().indexOf(q) === -1 &&
              (t.description || '').toLowerCase().indexOf(q) === -1) return false;
        }
        return true;
      });
    },

    isProviderConfigured(providerName) {
      if (!providerName) return false;
      var p = this.tplProviders.find(function(pr) { return pr.id === providerName; });
      return p ? p.auth_status === 'configured' : false;
    },

    async init() {
      var self = this;
      var store = Alpine.store('app');
      this.loadError = '';
      // Only fetch agents on first load — the 5s poll keeps them fresh after that
      if (!store.agents.length) {
        this.loading = true;
        try {
          var loaded = await store.refreshAgents();
          if (!loaded) {
            throw new Error(store.agentRefreshError || 'Could not load agents. Is the daemon running?');
          }
        } catch(e) {
          this.loadError = e.message || 'Could not load agents. Is the daemon running?';
        }
      }
      this.loading = false;

      // If returning to agents page with a previously selected agent, re-trigger chatPage
      if (store.activeChatAgent && !store.pendingAgent) {
        store.pendingAgent = store.activeChatAgent;
      }

      // If a "new agent" request was pending, open the spawn wizard
      if (store.pendingAgent === 'new') {
        this.openSpawnWizard();
        store.pendingAgent = null;
      }

      // Watch for future pendingAgent = 'new' requests
      this.$watch('$store.app.pendingAgent', function(val) {
        if (val === 'new') {
          self.openSpawnWizard();
          Alpine.store('app').pendingAgent = null;
        }
      });

      // Watch for template spawn requests from sidebar
      this.$watch('$store.app.pendingTemplate', function(t) {
        if (t) {
          self.spawnBuiltin(t);
          Alpine.store('app').pendingTemplate = null;
        }
      });

      // Handle pending template that was set before mount
      if (store.pendingTemplate) {
        this.spawnBuiltin(store.pendingTemplate);
        store.pendingTemplate = null;
      }
    },

    async loadData() {
      this.loading = true;
      this.loadError = '';
      try {
        var loaded = await Alpine.store('app').refreshAgents();
        if (!loaded) {
          throw new Error(Alpine.store('app').agentRefreshError || 'Could not load agents.');
        }
      } catch(e) {
        this.loadError = e.message || 'Could not load agents.';
      }
      this.loading = false;
    },

    async loadTemplates() {
      this.tplLoading = true;
      this.tplLoadError = '';
      var results = await Promise.allSettled([
        RustyHandAPI.get('/api/templates'),
        RustyHandAPI.get('/api/providers')
      ]);
      var errors = [];
      if (results[0].status === 'fulfilled') {
        this.tplTemplates = results[0].value.templates || [];
      } else {
        this.tplTemplates = [];
        errors.push('/api/templates: ' + (results[0].reason && results[0].reason.message ? results[0].reason.message : results[0].reason));
      }
      if (results[1].status === 'fulfilled') {
        this.tplProviders = results[1].value.providers || [];
      } else {
        this.tplProviders = [];
        errors.push('/api/providers: ' + (results[1].reason && results[1].reason.message ? results[1].reason.message : results[1].reason));
      }
      if (errors.length) {
        this.tplLoadError = errors.join(' | ');
      }
      this.tplLoading = false;
    },

    chatWithAgent(agent) {
      Alpine.store('app').chatWithAgent(agent);
    },

    closeChat() {
      Alpine.store('app').activeChatAgent = null;
      RustyHandAPI.wsDisconnect();
    },

    showDetail(agent) {
      this.detailAgent = agent;
      this.detailTab = 'info';
      this.editingModel = false;
      this.agentFiles = [];
      this.editingFile = null;
      this.fileContent = '';
      this.configForm = {
        name: agent.name || '',
        group: agent.group || '',
        system_prompt: agent.system_prompt || '',
        emoji: (agent.identity && agent.identity.emoji) || '',
        color: (agent.identity && agent.identity.color) || '#FF5C00',
        archetype: (agent.identity && agent.identity.archetype) || '',
        vibe: (agent.identity && agent.identity.vibe) || ''
      };
      this.showDetailModal = true;
    },

    killAgent(agent) {
      var self = this;
      RustyHandToast.confirm('Stop Agent', 'Stop agent "' + agent.name + '"? The agent will be shut down.', async function() {
        try {
          await RustyHandAPI.del('/api/agents/' + agent.id);
          RustyHandToast.success('Agent "' + agent.name + '" stopped');
          self.showDetailModal = false;
          await Alpine.store('app').refreshAgents();
        } catch(e) {
          RustyHandToast.error('Failed to stop agent: ' + e.message);
        }
      });
    },

    killAllAgents() {
      var list = this.filteredAgents;
      if (!list.length) return;
      RustyHandToast.confirm('Stop All Agents', 'Stop ' + list.length + ' agent(s)? All agents will be shut down.', async function() {
        var results = await Promise.allSettled(list.map(function(agent) {
          return RustyHandAPI.del('/api/agents/' + agent.id);
        }));
        var errors = [];
        for (var i = 0; i < results.length; i++) {
          if (results[i].status === 'rejected') {
            errors.push(list[i].name + ': ' + (results[i].reason.message || results[i].reason));
          }
        }
        await Alpine.store('app').refreshAgents();
        if (errors.length) {
          RustyHandToast.error('Some agents failed to stop: ' + errors.join(', '));
        } else {
          RustyHandToast.success(list.length + ' agent(s) stopped');
        }
      });
    },

    // ── Multi-step wizard navigation ──
    openSpawnWizard() {
      var store = Alpine.store('app');
      this.showSpawnModal = true;
      this.spawnStep = 1;
      this.spawnMode = 'wizard';
      this.spawnIdentity = { emoji: '', color: '#FF5C00', archetype: '' };
      this.selectedPreset = '';
      this.soulContent = '';
      this.spawnProfilesError = '';
      this.spawnForm.name = '';
      this.spawnForm.group = '';
      this.spawnForm.provider = store.defaultProvider || '';
      this.spawnForm.model = store.defaultModel || '';
      this.spawnForm.systemPrompt = 'You are a helpful assistant.';
      this.spawnForm.profile = 'full';
      this.spawnForm.autonomous = false;
      this.spawnForm.cronExpr = '';
      this.spawnForm.cronMessage = 'Run your scheduled task now';
      // Load providers/models in background while user fills step 1
      this.loadWizardData();
    },

    nextStep() {
      if (this.spawnStep === 1 && !this.spawnForm.name.trim()) {
        RustyHandToast.warn('Please enter an agent name');
        return;
      }
      if (this.spawnStep === 2 && (!this.spawnForm.provider || !this.spawnForm.model)) {
        RustyHandToast.warn('Please select a provider and model');
        return;
      }
      if (this.spawnStep < 3) this.spawnStep++;
    },

    prevStep() {
      if (this.spawnStep > 1) this.spawnStep--;
    },

    selectPreset(preset) {
      this.selectedPreset = preset.id;
      this.soulContent = preset.soul;
    },

    generateToml() {
      var f = this.spawnForm;
      var lines = [
        'name = "' + f.name + '"',
        'module = "builtin:chat"'
      ];
      if (f.group && f.group.trim()) {
        lines.splice(1, 0, 'group = "' + f.group.trim().replace(/"/g, '\\"') + '"');
      }
      if (f.profile && f.profile !== 'custom') {
        lines.push('profile = "' + f.profile + '"');
      }
      lines.push('', '[model]');
      lines.push('provider = "' + f.provider + '"');
      lines.push('model = "' + f.model + '"');
      lines.push('system_prompt = "' + f.systemPrompt.replace(/"/g, '\\"') + '"');
      if (f.profile === 'custom') {
        lines.push('', '[capabilities]');
        if (f.caps.memory_read) lines.push('memory_read = ["*"]');
        if (f.caps.memory_write) lines.push('memory_write = ["self.*"]');
        if (f.caps.network) lines.push('network = ["*"]');
        if (f.caps.shell) lines.push('shell = ["*"]');
        if (f.caps.agent_spawn) lines.push('agent_spawn = true');
      }
      return lines.join('\n');
    },

    async setMode(agent, mode) {
      try {
        await RustyHandAPI.put('/api/agents/' + agent.id + '/mode', { mode: mode });
        agent.mode = mode;
        RustyHandToast.success('Mode set to ' + mode);
        await Alpine.store('app').refreshAgents();
        this.syncDetailAgent();
      } catch(e) {
        RustyHandToast.error('Failed to set mode: ' + e.message);
      }
    },

    async spawnAgent() {
      this.spawning = true;
      var toml = this.spawnMode === 'wizard' ? this.generateToml() : this.spawnToml;
      if (!toml.trim()) {
        this.spawning = false;
        RustyHandToast.warn('Manifest is empty \u2014 enter agent config first');
        return;
      }

      try {
        var res = await RustyHandAPI.post('/api/agents', { manifest_toml: toml });
        if (res.agent_id) {
          // Post-spawn: update identity + write SOUL.md if personality preset selected
          var patchBody = {};
          if (this.spawnIdentity.emoji) patchBody.emoji = this.spawnIdentity.emoji;
          if (this.spawnIdentity.color) patchBody.color = this.spawnIdentity.color;
          if (this.spawnIdentity.archetype) patchBody.archetype = this.spawnIdentity.archetype;
          if (this.selectedPreset) patchBody.vibe = this.selectedPreset;

          if (Object.keys(patchBody).length) {
            RustyHandAPI.patch('/api/agents/' + res.agent_id + '/config', patchBody).catch(function(e) { console.warn('Post-spawn config patch failed:', e.message); });
          }
          if (this.soulContent.trim()) {
            RustyHandAPI.put('/api/agents/' + res.agent_id + '/files/SOUL.md', { content: '# Soul\n' + this.soulContent }).catch(function(e) { console.warn('SOUL.md write failed:', e.message); });
          }

          if (this.spawnForm.autonomous && this.spawnForm.cronExpr) {
            RustyHandAPI.post('/api/cron/jobs', {
              agent_id: res.agent_id,
              name: (this.spawnForm.name || 'agent') + ' schedule',
              enabled: true,
              schedule: { kind: 'cron', expr: this.spawnForm.cronExpr },
              action: {
                kind: 'agent_turn',
                message: this.spawnForm.cronMessage || 'Run your scheduled task now',
                timeout_secs: 300
              },
              delivery: { kind: 'none' }
            }).catch(function(e) { console.warn('Auto-schedule failed:', e.message); });
          }

          this.showSpawnModal = false;
          var spawnedProvider = this.spawnForm.provider;
          var spawnedModel = this.spawnForm.model;
          var spawnedGroup = this.spawnForm.group;
          this.spawnForm.name = '';
          this.spawnForm.group = '';
          this.spawnToml = '';
          this.spawnStep = 1;
          RustyHandToast.success('Agent "' + (res.name || 'new') + '" spawned');
          await Alpine.store('app').refreshAgents();
          this.chatWithAgent({ id: res.agent_id, name: res.name, group: spawnedGroup, model_provider: spawnedProvider, model_name: spawnedModel });
        } else {
          RustyHandToast.error('Spawn failed: ' + (res.error || 'Unknown error'));
        }
      } catch(e) {
        RustyHandToast.error('Failed to spawn agent: ' + e.message);
      }
      this.spawning = false;
    },

    // ── Detail modal: Files tab ──
    async loadAgentFiles() {
      if (!this.detailAgent) return;
      this.filesLoading = true;
      try {
        var data = await RustyHandAPI.get('/api/agents/' + this.detailAgent.id + '/files');
        this.agentFiles = data.files || [];
      } catch(e) {
        this.agentFiles = [];
        RustyHandToast.error('Failed to load files: ' + e.message);
      }
      this.filesLoading = false;
    },

    async openFile(file) {
      if (!file.exists) {
        // Create with empty content
        this.editingFile = file.name;
        this.fileContent = '';
        return;
      }
      try {
        var data = await RustyHandAPI.get('/api/agents/' + this.detailAgent.id + '/files/' + encodeURIComponent(file.name));
        this.editingFile = file.name;
        this.fileContent = data.content || '';
      } catch(e) {
        RustyHandToast.error('Failed to read file: ' + e.message);
      }
    },

    async saveFile() {
      if (!this.editingFile || !this.detailAgent) return;
      this.fileSaving = true;
      try {
        await RustyHandAPI.put('/api/agents/' + this.detailAgent.id + '/files/' + encodeURIComponent(this.editingFile), { content: this.fileContent });
        RustyHandToast.success(this.editingFile + ' saved');
        await this.loadAgentFiles();
      } catch(e) {
        RustyHandToast.error('Failed to save file: ' + e.message);
      }
      this.fileSaving = false;
    },

    closeFileEditor() {
      this.editingFile = null;
      this.fileContent = '';
    },

    // ── Detail modal: Config tab ──
    async saveConfig() {
      if (!this.detailAgent) return;
      this.configSaving = true;
      try {
        await RustyHandAPI.patch('/api/agents/' + this.detailAgent.id + '/config', this.configForm);
        RustyHandToast.success('Config updated');
        await Alpine.store('app').refreshAgents();
        this.syncDetailAgent();
      } catch(e) {
        RustyHandToast.error('Failed to save config: ' + e.message);
      }
      this.configSaving = false;
    },

    // ── Clone agent ──
    async cloneAgent(agent) {
      var newName = (agent.name || 'agent') + '-copy';
      try {
        var res = await RustyHandAPI.post('/api/agents/' + agent.id + '/clone', { new_name: newName });
        if (res.agent_id) {
          RustyHandToast.success('Cloned as "' + res.name + '"');
          await Alpine.store('app').refreshAgents();
          this.showDetailModal = false;
        }
      } catch(e) {
        RustyHandToast.error('Clone failed: ' + e.message);
      }
    },

    // -- Template methods --
    async spawnFromTemplate(name) {
      try {
        var data = await RustyHandAPI.get('/api/templates/' + encodeURIComponent(name));
        if (data.manifest_toml) {
          var res = await RustyHandAPI.post('/api/agents', { manifest_toml: data.manifest_toml });
          if (res.agent_id) {
            RustyHandToast.success('Agent "' + (res.name || name) + '" spawned from template');
            await Alpine.store('app').refreshAgents();
            this.chatWithAgent({ id: res.agent_id, name: res.name || name, model_provider: '?', model_name: '?' });
          }
        }
      } catch(e) {
        RustyHandToast.error('Failed to spawn from template: ' + e.message);
      }
    },

    async spawnBuiltin(t) {
      var self = this;
      var store = Alpine.store('app');
      var provider = store.defaultProvider || 'minimax';
      var model = store.defaultModel || 'MiniMax-M2.7';
      var toml = 'name = "' + t.name + '"\n';
      toml += 'description = "' + t.description.replace(/"/g, '\\"') + '"\n';
      toml += 'module = "builtin:chat"\n';
      toml += 'profile = "' + t.profile + '"\n\n';
      toml += '[model]\nprovider = "' + provider + '"\nmodel = "' + model + '"\n';
      toml += 'system_prompt = """\n' + t.system_prompt + '\n"""\n';

      try {
        var res = await RustyHandAPI.post('/api/agents', { manifest_toml: toml });
        if (res.agent_id) {
          if (t.autonomous && t.cronExpr) {
            RustyHandAPI.post('/api/cron/jobs', {
              agent_id: res.agent_id,
              name: t.name + ' schedule',
              enabled: true,
              schedule: { kind: 'cron', expr: t.cronExpr },
              action: { kind: 'agent_turn', message: t.cronMessage || 'Run scheduled task', timeout_secs: 300 },
              delivery: { kind: 'none' }
            }).catch(function(e) { console.warn('Template auto-schedule failed:', e.message); });
          }
          RustyHandToast.success('Agent "' + t.name + '" spawned');
          await Alpine.store('app').refreshAgents();
          this.chatWithAgent({ id: res.agent_id, name: t.name, model_provider: provider, model_name: model });
        }
      } catch(e) {
        RustyHandToast.error('Failed to spawn agent: ' + e.message);
      }
    }
  };
}
