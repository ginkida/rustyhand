// RustyHand Automation Page — Cron job management via /api/cron/jobs
'use strict';

function automationPage() {
  return {
    tab: 'jobs',

    // -- Scheduled Jobs state --
    jobs: [],
    loading: true,
    loadError: '',

    // -- Filter --
    filterAgentId: '',
    agents: [],
    agentsLoadError: '',

    // -- Workflows (for workflow_run action type) --
    workflows: [],

    // -- Event Triggers state --
    triggers: [],
    trigLoading: false,
    trigLoadError: '',

    // -- Run History state --
    history: [],
    historyLoading: false,

    // -- Create/Edit Job form --
    showJobForm: false,
    editingJobId: null,
    jobForm: {
      name: '',
      agent_id: '',
      schedule_type: 'cron',
      cron_expr: '',
      every_secs: 3600,
      at_datetime: '',
      action_type: 'agent_turn',
      message: '',
      event_text: '',
      workflow_id: '',
      workflow_input: '',
      model_override: '',
      timeout_secs: 300,
      delivery_type: 'none',
      delivery_channel: '',
      delivery_to: '',
      webhook_url: '',
      one_shot: false,
      enabled: true,
      _showDelivery: false
    },
    saving: false,
    _editingAgentName: '',

    // -- Run Now state --
    runningJobId: '',

    // Cron presets
    cronPresets: [
      { label: 'Every 5 min', cron: '*/5 * * * *' },
      { label: 'Every 15 min', cron: '*/15 * * * *' },
      { label: 'Every 30 min', cron: '*/30 * * * *' },
      { label: 'Every hour', cron: '0 * * * *' },
      { label: 'Every 6 hours', cron: '0 */6 * * *' },
      { label: 'Daily midnight', cron: '0 0 * * *' },
      { label: 'Daily 9am', cron: '0 9 * * *' },
      { label: 'Weekdays 9am', cron: '0 9 * * 1-5' },
      { label: 'Monday 9am', cron: '0 9 * * 1' },
      { label: '1st of month', cron: '0 0 1 * *' }
    ],

    // Interval presets (in seconds)
    intervalPresets: [
      { label: '1 min', secs: 60 },
      { label: '5 min', secs: 300 },
      { label: '15 min', secs: 900 },
      { label: '30 min', secs: 1800 },
      { label: '1 hour', secs: 3600 },
      { label: '2 hours', secs: 7200 },
      { label: '6 hours', secs: 21600 },
      { label: '12 hours', secs: 43200 },
      { label: '24 hours', secs: 86400 }
    ],

    // ── Lifecycle ──

    async loadData() {
      this.loading = true;
      this.loadError = '';
      try {
        await this.loadAgents();
      } catch(e) {
        // Jobs can still load with fallback labels even if agent refresh fails.
      }
      try {
        await this.loadWorkflows();
      } catch(e) {
        // Workflow list is optional — jobs that reference workflows will still work.
      }
      try {
        await this.loadJobs();
      } catch(e) {
        this.loadError = e.message || 'Could not load automation data.';
      }
      this.loading = false;
    },

    async loadWorkflows() {
      try {
        var data = await RustyHandAPI.get('/api/workflows');
        this.workflows = Array.isArray(data) ? data : (data.workflows || []);
      } catch (e) {
        this.workflows = [];
      }
    },

    async loadAgents() {
      try {
        var data = await RustyHandAPI.get('/api/agents');
        this.agents = Array.isArray(data) ? data : [];
        this.agentsLoadError = '';
      } catch(e) {
        this.agents = Alpine.store('app').agents || [];
        this.agentsLoadError = e.message || 'Could not load agents.';
        throw e;
      }
    },

    async loadJobs() {
      var all = [];
      var errors = [];

      try {
        var url = '/api/cron/jobs';
        if (this.filterAgentId) {
          url += '?agent_id=' + encodeURIComponent(this.filterAgentId);
        }
        var cronData = await RustyHandAPI.get(url);
        var cronJobs = cronData.jobs || [];
        for (var i = 0; i < cronJobs.length; i++) {
          var cronJob = cronJobs[i];
          cronJob.agent_id = this.normalizeAgentId(cronJob.agent_id);
          cronJob._agent_name = this.resolvedAgentName(cronJob.agent_id);
          all.push(cronJob);
        }
      } catch(e) {
        errors.push('/api/cron/jobs: ' + (e.message || e));
      }

      this.jobs = all;
      if (!all.length && errors.length) {
        throw new Error(errors.join(' | '));
      }
    },

    async loadTriggers() {
      this.trigLoading = true;
      this.trigLoadError = '';
      try {
        var data = await RustyHandAPI.get('/api/triggers');
        this.triggers = Array.isArray(data) ? data : [];
      } catch(e) {
        this.triggers = [];
        this.trigLoadError = e.message || 'Could not load triggers.';
      }
      this.trigLoading = false;
    },

    async loadHistory() {
      this.historyLoading = true;
      try {
        var historyItems = [];
        var jobs = this.jobs || [];
        for (var i = 0; i < jobs.length; i++) {
          var job = jobs[i];
          if (job.last_run) {
            historyItems.push({
              timestamp: job.last_run,
              name: job.name || '(unnamed)',
              agent: this.agentName(job.agent_id),
              type: 'job',
              status: 'completed',
              schedule_desc: this.describeJobSchedule(job),
              run_count: job.run_count || job._run_count || 0
            });
          }
        }
        var triggers = this.triggers || [];
        for (var j = 0; j < triggers.length; j++) {
          var t = triggers[j];
          if (t.fire_count > 0) {
            historyItems.push({
              timestamp: t.last_fired_at || t.created_at,
              name: 'Trigger: ' + this.triggerType(t.pattern),
              agent: this.agentName(t.agent_id),
              type: 'trigger',
              status: 'fired',
              schedule_desc: '',
              run_count: t.fire_count || 0
            });
          }
        }
        historyItems.sort(function(a, b) {
          return new Date(b.timestamp).getTime() - new Date(a.timestamp).getTime();
        });
        this.history = historyItems;
      } catch(e) {
        this.history = [];
      }
      this.historyLoading = false;
    },

    // ── Form helpers ──

    resetJobForm() {
      this.editingJobId = null;
      this.jobForm = {
        name: '',
        agent_id: '',
        schedule_type: 'cron',
        cron_expr: '',
        every_secs: 3600,
        at_datetime: '',
        action_type: 'agent_turn',
        message: '',
        event_text: '',
        model_override: '',
        timeout_secs: 300,
        delivery_type: 'none',
        delivery_channel: '',
        delivery_to: '',
        webhook_url: '',
        one_shot: false,
        enabled: true,
        _showDelivery: false
      };
      this._editingAgentName = '';
    },

    openCreateForm() {
      this.resetJobForm();
      this.showJobForm = true;
    },

    openCreateForAgent(agentId) {
      this.resetJobForm();
      this.jobForm.agent_id = agentId;
      this.showJobForm = true;
    },

    resolveAgentName(name) {
      if (!name) return '';
      var agents = this.availableAgents;
      var lower = name.toLowerCase();
      for (var i = 0; i < agents.length; i++) {
        if (agents[i].name === name || agents[i].name.toLowerCase() === lower) return agents[i].id;
      }
      return '';
    },

    editJob(job) {
      // Reset form first to prevent field bleed between edits
      this.resetJobForm();

      this.editingJobId = job.id;
      this._editingAgentName = job._agent_name || this.resolvedAgentName(job.agent_id) || this.shortAgentId(job.agent_id);
      var f = this.jobForm;
      f.name = job.name || '';

      var rawAgentId = this.normalizeAgentId(job.agent_id);
      f.agent_id = rawAgentId;
      if (!f.agent_id && this._editingAgentName) {
        f.agent_id = this.resolveAgentName(this._editingAgentName);
      }
      f.enabled = job.enabled !== false;

      // Parse schedule
      var sched = job.schedule || {};
      if (sched.kind === 'every') {
        f.schedule_type = 'every';
        f.every_secs = sched.every_secs || 3600;
      } else if (sched.kind === 'at') {
        f.schedule_type = 'at';
        f.at_datetime = sched.at ? sched.at.substring(0, 16) : '';
      } else {
        f.schedule_type = 'cron';
        f.cron_expr = sched.expr || '';
      }

      // Parse action
      var act = job.action || {};
      if (act.kind === 'system_event') {
        f.action_type = 'system_event';
        f.event_text = act.text || '';
      } else if (act.kind === 'workflow_run') {
        f.action_type = 'workflow_run';
        f.workflow_id = act.workflow_id || '';
        f.workflow_input = act.input || '';
        f.timeout_secs = act.timeout_secs || 300;
      } else {
        f.action_type = 'agent_turn';
        f.message = act.message || '';
        f.model_override = act.model_override || '';
        f.timeout_secs = act.timeout_secs || 300;
      }

      // Parse delivery
      var del = job.delivery || {};
      if (del.kind === 'channel') {
        f.delivery_type = 'channel';
        f.delivery_channel = del.channel || '';
        f.delivery_to = del.to || '';
      } else if (del.kind === 'last_channel') {
        f.delivery_type = 'last_channel';
      } else if (del.kind === 'webhook') {
        f.delivery_type = 'webhook';
        f.webhook_url = del.url || '';
      } else {
        f.delivery_type = 'none';
      }

      // Read one_shot from API response; set true for one-time jobs
      f.one_shot = job.one_shot || false;
      if (f.schedule_type === 'at') f.one_shot = true;
      f._showDelivery = f.delivery_type !== 'none';
      this.showJobForm = true;
      var self = this;
      this.$nextTick(function() {
        if (self.$refs && self.$refs.scheduleAgentSelect) {
          self.$refs.scheduleAgentSelect.value = self.normalizeAgentId(f.agent_id);
        }
      });
    },

    buildJobPayload() {
      var f = this.jobForm;
      var payload = {
        agent_id: this.normalizeAgentId(f.agent_id),
        name: f.name.trim()
      };

      // Schedule
      if (f.schedule_type === 'every') {
        payload.schedule = { kind: 'every', every_secs: parseInt(f.every_secs, 10) || 3600 };
      } else if (f.schedule_type === 'at') {
        var dt = f.at_datetime ? new Date(f.at_datetime).toISOString() : '';
        payload.schedule = { kind: 'at', at: dt };
      } else {
        payload.schedule = { kind: 'cron', expr: f.cron_expr.trim() };
      }

      // Action
      if (f.action_type === 'system_event') {
        payload.action = { kind: 'system_event', text: f.event_text.trim() };
      } else if (f.action_type === 'workflow_run') {
        var wfAction = {
          kind: 'workflow_run',
          workflow_id: f.workflow_id.trim(),
          input: f.workflow_input
        };
        if (f.timeout_secs) wfAction.timeout_secs = parseInt(f.timeout_secs, 10);
        payload.action = wfAction;
      } else {
        var action = { kind: 'agent_turn', message: f.message.trim() };
        if (f.model_override.trim()) action.model_override = f.model_override.trim();
        if (f.timeout_secs) action.timeout_secs = parseInt(f.timeout_secs, 10);
        payload.action = action;
      }

      // Delivery
      if (f.delivery_type === 'channel') {
        payload.delivery = { kind: 'channel', channel: f.delivery_channel.trim(), to: f.delivery_to.trim() };
      } else if (f.delivery_type === 'last_channel') {
        payload.delivery = { kind: 'last_channel' };
      } else if (f.delivery_type === 'webhook') {
        payload.delivery = { kind: 'webhook', url: f.webhook_url.trim() };
      } else {
        payload.delivery = { kind: 'none' };
      }

      payload.one_shot = f.one_shot;
      payload.enabled = f.enabled;
      return payload;
    },

    // ── Job CRUD ──

    async saveJob() {
      var f = this.jobForm;
      if (!f.name.trim()) {
        RustyHandToast.warn('Please enter a job name');
        return;
      }
      if (!f.agent_id) {
        if (this._editingAgentName) {
          RustyHandToast.warn('Agent "' + this._editingAgentName + '" is not running — start it first to reassign');
        } else {
          RustyHandToast.warn('Please select a target agent');
        }
        return;
      }
      if (f.schedule_type === 'cron' && !f.cron_expr.trim()) {
        RustyHandToast.warn('Please enter a cron expression');
        return;
      }
      if (f.schedule_type === 'at' && !f.at_datetime) {
        RustyHandToast.warn('Please select a date and time');
        return;
      }
      if (f.action_type === 'agent_turn' && !f.message.trim()) {
        RustyHandToast.warn('Please enter a message for the agent');
        return;
      }
      if (f.action_type === 'system_event' && !f.event_text.trim()) {
        RustyHandToast.warn('Please enter event text');
        return;
      }
      if (f.action_type === 'workflow_run' && !f.workflow_id.trim()) {
        RustyHandToast.warn('Please select a workflow to run');
        return;
      }

      this.saving = true;
      try {
        var payload = this.buildJobPayload();

        if (this.editingJobId) {
          var editId = this.jobId({ id: this.editingJobId });
          await RustyHandAPI.put('/api/cron/jobs/' + editId, payload);
          RustyHandToast.success('Job "' + f.name + '" updated');
        } else {
          await RustyHandAPI.post('/api/cron/jobs', payload);
          RustyHandToast.success('Job "' + f.name + '" created');
        }
        this.showJobForm = false;
        this.resetJobForm();
        await this.loadJobs();
      } catch(e) {
        RustyHandToast.error('Failed to save job: ' + (e.message || e));
      }
      this.saving = false;
    },

    async toggleJob(job) {
      try {
        var newState = !job.enabled;
        var id = this.jobId(job);
        await RustyHandAPI.put('/api/cron/jobs/' + id + '/enable', { enabled: newState });
        job.enabled = newState;
        RustyHandToast.success('Job ' + (newState ? 'enabled' : 'paused'));
      } catch(e) {
        RustyHandToast.error('Failed to toggle job: ' + (e.message || e));
      }
    },

    deleteJob(job) {
      var self = this;
      var jobName = job.name || 'this job';
      var id = this.jobId(job);
      RustyHandToast.confirm('Delete Job', 'Delete "' + jobName + '"? This cannot be undone.', async function() {
        try {
          await RustyHandAPI.del('/api/cron/jobs/' + id);
          self.jobs = self.jobs.filter(function(j) { return self.jobId(j) !== id; });
          RustyHandToast.success('Job "' + jobName + '" deleted');
        } catch(e) {
          RustyHandToast.error('Failed to delete job: ' + (e.message || e));
        }
      });
    },

    // ── Trigger helpers ──

    triggerType(pattern) {
      if (!pattern) return 'unknown';
      if (typeof pattern === 'string') return pattern;
      var keys = Object.keys(pattern);
      if (keys.length === 0) return 'unknown';
      var key = keys[0];
      var names = {
        lifecycle: 'Lifecycle',
        agent_spawned: 'Agent Spawned',
        agent_terminated: 'Agent Terminated',
        system: 'System',
        system_keyword: 'System Keyword',
        memory_update: 'Memory Update',
        memory_key_pattern: 'Memory Key',
        all: 'All Events',
        content_match: 'Content Match'
      };
      return names[key] || key.replace(/_/g, ' ');
    },

    async toggleTrigger(trigger) {
      try {
        var newState = !trigger.enabled;
        await RustyHandAPI.put('/api/triggers/' + trigger.id, { enabled: newState });
        trigger.enabled = newState;
        RustyHandToast.success('Trigger ' + (newState ? 'enabled' : 'disabled'));
      } catch(e) {
        RustyHandToast.error('Failed to toggle trigger: ' + (e.message || e));
      }
    },

    deleteTrigger(trigger) {
      var self = this;
      RustyHandToast.confirm('Delete Trigger', 'Delete this trigger? This cannot be undone.', async function() {
        try {
          await RustyHandAPI.del('/api/triggers/' + trigger.id);
          self.triggers = self.triggers.filter(function(t) { return t.id !== trigger.id; });
          RustyHandToast.success('Trigger deleted');
        } catch(e) {
          RustyHandToast.error('Failed to delete trigger: ' + (e.message || e));
        }
      });
    },

    // ── Utility ──

    get availableAgents() {
      if (this.agents && this.agents.length) return this.agents;
      return Alpine.store('app').agents || [];
    },

    normalizeAgentId(agentId) {
      if (!agentId) return '';
      if (typeof agentId === 'string') return agentId;
      if (typeof agentId === 'object') {
        if (agentId['0']) return String(agentId['0']);
        if (Array.isArray(agentId) && agentId.length > 0) return String(agentId[0]);
      }
      return String(agentId);
    },

    resolvedAgentName(agentId) {
      var id = this.normalizeAgentId(agentId);
      if (!id) return '';
      var agents = this.availableAgents;
      for (var i = 0; i < agents.length; i++) {
        if (agents[i].id === id) return agents[i].name;
      }
      return '';
    },

    shortAgentId(agentId) {
      var id = this.normalizeAgentId(agentId);
      if (!id) return '(any)';
      if (id.length > 12) return id.substring(0, 8) + '...';
      return id;
    },

    hasAgentOption(agentId) {
      var id = this.normalizeAgentId(agentId);
      if (!id) return false;
      var agents = this.availableAgents;
      for (var i = 0; i < agents.length; i++) {
        if (agents[i].id === id) return true;
      }
      return false;
    },

    jobAgentLabel(job) {
      if (job && job._agent_name) return job._agent_name;
      var resolved = this.resolvedAgentName(job ? job.agent_id : '');
      if (resolved) return resolved;
      return this.shortAgentId(job ? job.agent_id : '');
    },

    agentName(agentId) {
      var resolved = this.resolvedAgentName(agentId);
      if (resolved) return resolved;
      return this.shortAgentId(agentId);
    },

    jobId(job) {
      if (!job.id) return '';
      if (typeof job.id === 'object' && job.id['0']) return job.id['0'];
      return String(job.id);
    },

    describeJobSchedule(job) {
      var sched = job.schedule || {};
      if (sched.kind === 'every') {
        return this.describeInterval(sched.every_secs);
      }
      if (sched.kind === 'at') {
        return 'Once at ' + this.formatTime(sched.at);
      }
      if (sched.kind === 'cron') {
        return this.describeCron(sched.expr);
      }
      return '?';
    },

    scheduleCode(job) {
      var sched = job.schedule || {};
      if (sched.kind === 'every') return 'every ' + sched.every_secs + 's';
      if (sched.kind === 'at') return 'at ' + (sched.at || '').substring(0, 16);
      if (sched.kind === 'cron') return sched.expr || '';
      return '';
    },

    describeInterval(secs) {
      if (!secs) return '';
      if (secs < 120) return 'Every ' + secs + ' seconds';
      if (secs < 7200) return 'Every ' + Math.round(secs / 60) + ' minutes';
      if (secs < 172800) return 'Every ' + Math.round(secs / 3600) + ' hours';
      return 'Every ' + Math.round(secs / 86400) + ' days';
    },

    describeCron(expr) {
      if (!expr) return '';
      var map = {
        '* * * * *': 'Every minute',
        '*/5 * * * *': 'Every 5 minutes',
        '*/10 * * * *': 'Every 10 minutes',
        '*/15 * * * *': 'Every 15 minutes',
        '*/30 * * * *': 'Every 30 minutes',
        '0 * * * *': 'Every hour',
        '0 */2 * * *': 'Every 2 hours',
        '0 */4 * * *': 'Every 4 hours',
        '0 */6 * * *': 'Every 6 hours',
        '0 */12 * * *': 'Every 12 hours',
        '0 0 * * *': 'Daily at midnight',
        '0 6 * * *': 'Daily at 6:00 AM',
        '0 9 * * *': 'Daily at 9:00 AM',
        '0 12 * * *': 'Daily at noon',
        '0 18 * * *': 'Daily at 6:00 PM',
        '0 9 * * 1-5': 'Weekdays at 9:00 AM',
        '0 9 * * 1': 'Mondays at 9:00 AM',
        '0 0 * * 0': 'Sundays at midnight',
        '0 0 1 * *': '1st of every month',
        '0 0 * * 1': 'Mondays at midnight'
      };
      if (map[expr]) return map[expr];
      var parts = expr.split(' ');
      if (parts.length !== 5) return expr;
      var min = parts[0], hour = parts[1], dom = parts[2], mon = parts[3], dow = parts[4];
      if (min.indexOf('*/') === 0 && hour === '*' && dom === '*' && mon === '*' && dow === '*') {
        return 'Every ' + min.substring(2) + ' minutes';
      }
      if (min === '0' && hour.indexOf('*/') === 0 && dom === '*' && mon === '*' && dow === '*') {
        return 'Every ' + hour.substring(2) + ' hours';
      }
      if (dom === '*' && mon === '*' && dow === '*' && min.match(/^\d+$/) && hour.match(/^\d+$/)) {
        var h = parseInt(hour, 10), m = parseInt(min, 10);
        var ampm = h >= 12 ? 'PM' : 'AM';
        var h12 = h === 0 ? 12 : (h > 12 ? h - 12 : h);
        var mStr = m < 10 ? '0' + m : '' + m;
        return 'Daily at ' + h12 + ':' + mStr + ' ' + ampm;
      }
      return expr;
    },

    applyCronPreset(preset) {
      this.jobForm.cron_expr = preset.cron;
    },

    applyIntervalPreset(preset) {
      this.jobForm.every_secs = preset.secs;
    },

    scheduleTypeName(kind) {
      var names = { cron: 'Cron', every: 'Interval', at: 'One-time' };
      return names[kind] || kind;
    },

    actionTypeName(kind) {
      var names = { agent_turn: 'Agent Message', system_event: 'System Event', workflow_run: 'Workflow' };
      return names[kind] || kind;
    },

    actionPreviewFull(job) {
      var a = job && job.action || {};
      if (a.kind === 'system_event') return a.text || '';
      if (a.kind === 'workflow_run') {
        var wfName = '';
        for (var i = 0; i < this.workflows.length; i++) {
          if (this.workflows[i].id === a.workflow_id) { wfName = this.workflows[i].name; break; }
        }
        var label = wfName ? 'workflow: ' + wfName : 'workflow: ' + (a.workflow_id || '?');
        return a.input ? label + ' — ' + a.input : label;
      }
      return a.message || '';
    },

    actionPreview(job) {
      var full = this.actionPreviewFull(job);
      if (!full) return '';
      return full.length > 50 ? full.substring(0, 50) + '...' : full;
    },

    deliveryTypeName(kind) {
      var names = { none: 'None', last_channel: 'Last Channel', channel: 'Channel', webhook: 'Webhook' };
      return names[kind] || kind;
    },

    deliveryDesc(job) {
      var d = job.delivery || {};
      if (d.kind === 'channel') return d.channel + ':' + d.to;
      if (d.kind === 'webhook') return 'Webhook';
      if (d.kind === 'last_channel') return 'Last channel';
      return '-';
    },

    formatTime(ts) {
      if (!ts) return '-';
      try {
        var d = new Date(ts);
        if (isNaN(d.getTime())) return '-';
        return d.toLocaleString();
      } catch(e) { return '-'; }
    },

    relativeTime(ts) {
      if (!ts) return 'never';
      try {
        var diff = Date.now() - new Date(ts).getTime();
        if (isNaN(diff)) return 'never';
        if (diff < 0) return 'in ' + this.humanDuration(-diff);
        if (diff < 60000) return 'just now';
        if (diff < 3600000) return Math.floor(diff / 60000) + 'm ago';
        if (diff < 86400000) return Math.floor(diff / 3600000) + 'h ago';
        return Math.floor(diff / 86400000) + 'd ago';
      } catch(e) { return 'never'; }
    },

    nextRunText(job) {
      if (!job.next_run) return '-';
      var diff = new Date(job.next_run).getTime() - Date.now();
      if (diff < 0) return 'overdue';
      return 'in ' + this.humanDuration(diff);
    },

    humanDuration(ms) {
      if (ms < 60000) return Math.ceil(ms / 1000) + 's';
      if (ms < 3600000) return Math.floor(ms / 60000) + 'm';
      if (ms < 86400000) return Math.floor(ms / 3600000) + 'h ' + Math.floor((ms % 3600000) / 60000) + 'm';
      return Math.floor(ms / 86400000) + 'd';
    },

    jobCount() {
      var enabled = 0;
      for (var i = 0; i < this.jobs.length; i++) {
        if (this.jobs[i].enabled) enabled++;
      }
      return enabled;
    },

    get filteredJobs() {
      return this.jobs;
    }
  };
}
