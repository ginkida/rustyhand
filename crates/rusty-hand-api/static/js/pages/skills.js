// RustyHand Skills Page — installed skills + MCP servers
'use strict';

function skillsPage() {
  return {
    tab: 'installed',
    skills: [],
    loading: true,
    loadError: '',

    // Install form state
    installOpen: false,
    installSaving: false,
    installForm: {
      name: '',
      language: 'python',
      description: '',
      content: '',
      overwrite: false,
    },

    mcpServers: { configured: [], connected: [], total_configured: 0, total_connected: 0 },
    mcpLoading: false,

    openInstallForm() {
      this.installForm = {
        name: '',
        language: 'python',
        description: '',
        content: 'def run(input):\n    # input is a dict; return any JSON-serialisable value\n    return {"echo": input}\n',
        overwrite: false,
      };
      this.installOpen = true;
    },

    onLanguageChange() {
      var f = this.installForm;
      if (f.language === 'python' && !f.content.includes('def run')) {
        f.content = 'def run(input):\n    # input is a dict; return any JSON-serialisable value\n    return {"echo": input}\n';
      } else if (f.language === 'node' && !f.content.includes('function run')) {
        f.content = 'function run(input) {\n  // input is an object; return any JSON-serialisable value\n  return { echo: input };\n}\n';
      }
    },

    async submitInstall() {
      var f = this.installForm;
      if (!/^[a-z][a-z0-9_]{0,63}$/.test(f.name)) {
        RustyHandToast.error('Name must match ^[a-z][a-z0-9_]{0,63}$');
        return;
      }
      if (!f.content.trim()) {
        RustyHandToast.error('Skill content is required');
        return;
      }
      this.installSaving = true;
      try {
        var res = await RustyHandAPI.post('/api/skills/install-custom', {
          name: f.name,
          language: f.language,
          description: f.description,
          content: f.content,
          overwrite: f.overwrite,
        });
        RustyHandToast.success('Skill installed: ' + (res.message || f.name));
        this.installOpen = false;
        await this.loadSkills();
      } catch (e) {
        RustyHandToast.error('Install failed: ' + e.message);
      }
      this.installSaving = false;
    },

    runtimeBadge: function(rt) {
      var r = (rt || '').toLowerCase();
      if (r === 'python' || r === 'py') return { text: 'PY', cls: 'runtime-badge-py' };
      if (r === 'node' || r === 'nodejs' || r === 'js' || r === 'javascript') return { text: 'JS', cls: 'runtime-badge-js' };
      if (r === 'wasm' || r === 'webassembly') return { text: 'WASM', cls: 'runtime-badge-wasm' };
      if (r === 'prompt_only' || r === 'prompt' || r === 'promptonly') return { text: 'PROMPT', cls: 'runtime-badge-prompt' };
      return { text: r.toUpperCase().substring(0, 4), cls: 'runtime-badge-prompt' };
    },

    sourceBadge: function(source) {
      if (!source) return { text: 'Local', cls: 'badge-dim' };
      switch (source.type) {
        case 'clawhub': return { text: 'ClawHub', cls: 'badge-info' };
        case 'openclaw': return { text: 'OpenClaw', cls: 'badge-info' };
        case 'bundled': return { text: 'Built-in', cls: 'badge-success' };
        default: return { text: 'Local', cls: 'badge-dim' };
      }
    },

    async loadSkills() {
      this.loading = true;
      this.loadError = '';
      try {
        var data = await RustyHandAPI.get('/api/skills');
        this.skills = (data.skills || []).map(function(s) {
          return {
            name: s.name,
            description: s.description || '',
            version: s.version || '',
            author: s.author || '',
            runtime: s.runtime || 'unknown',
            tools_count: s.tools_count || 0,
            enabled: s.enabled !== false,
            source: s.source || { type: 'local' },
            has_prompt_context: !!s.has_prompt_context
          };
        });
      } catch (e) {
        this.skills = [];
        this.loadError = e.message || 'Could not load skills.';
      }
      this.loading = false;
    },

    async loadData() {
      await this.loadSkills();
    },

    uninstallSkill: function(name) {
      var self = this;
      RustyHandToast.confirm('Uninstall Skill', 'Uninstall skill "' + name + '"? This cannot be undone.', async function() {
        try {
          await RustyHandAPI.post('/api/skills/uninstall', { name: name });
          RustyHandToast.success('Skill "' + name + '" uninstalled');
          await self.loadSkills();
        } catch (e) {
          RustyHandToast.error('Failed to uninstall skill: ' + e.message);
        }
      });
    },

    async loadMcpServers() {
      this.mcpLoading = true;
      try {
        var data = await RustyHandAPI.get('/api/mcp/servers');
        this.mcpServers = data;
      } catch (e) {
        this.mcpServers = { configured: [], connected: [], total_configured: 0, total_connected: 0 };
      }
      this.mcpLoading = false;
    }
  };
}
