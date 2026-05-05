// RustyHand Workflows Page — Workflow builder + run history
'use strict';

function workflowsPage() {
  return {
    workflows: [],
    showCreateModal: false,
    runModal: null,
    runInput: '',
    runResult: '',
    running: false,
    loading: true,
    loadError: '',
    agents: [],

    newWf: {
      name: '',
      description: '',
      steps: []
    },

    emptyStep() {
      return {
        name: '',
        agent_name: '',
        mode: 'sequential',
        prompt: '{{input}}',
        error_mode: 'fail',
        timeout_secs: 120,
        condition: '',
        max_iterations: 5,
        until: '',
        output_var: '',
        _expanded: true
      };
    },

    async loadWorkflows() {
      this.loading = true;
      this.loadError = '';
      try {
        var results = await Promise.all([
          RustyHandAPI.get('/api/workflows'),
          RustyHandAPI.get('/api/agents')
        ]);
        this.workflows = Array.isArray(results[0]) ? results[0] : (results[0].workflows || []);
        this.agents = results[1].agents || (Array.isArray(results[1]) ? results[1] : []);
      } catch(e) {
        this.workflows = [];
        this.loadError = e.message || 'Could not load workflows.';
      }
      this.loading = false;
    },

    async loadData() { return this.loadWorkflows(); },

    openCreateModal() {
      this.newWf = { name: '', description: '', steps: [this.emptyStep()] };
      this.showCreateModal = true;
    },

    addStep() {
      var step = this.emptyStep();
      step._expanded = true;
      // Collapse others
      for (var i = 0; i < this.newWf.steps.length; i++) {
        this.newWf.steps[i]._expanded = false;
      }
      this.newWf.steps.push(step);
    },

    removeStep(idx) {
      this.newWf.steps.splice(idx, 1);
    },

    moveStep(idx, dir) {
      var target = idx + dir;
      if (target < 0 || target >= this.newWf.steps.length) return;
      var steps = this.newWf.steps;
      var tmp = steps[idx];
      steps[idx] = steps[target];
      steps[target] = tmp;
    },

    modeLabel(mode) {
      var labels = {
        sequential: 'Sequential',
        fan_out: 'Fan Out',
        collect: 'Collect',
        conditional: 'Conditional',
        loop: 'Loop'
      };
      return labels[mode] || mode;
    },

    modeColor(mode) {
      var colors = {
        sequential: 'var(--accent)',
        fan_out: '#f59e0b',
        collect: '#8b5cf6',
        conditional: '#10b981',
        loop: '#ef4444'
      };
      return colors[mode] || 'var(--text-dim)';
    },

    async createWorkflow() {
      if (!this.newWf.name.trim()) {
        RustyHandToast.warn('Enter a workflow name');
        return;
      }
      if (!this.newWf.steps.length) {
        RustyHandToast.warn('Add at least one step');
        return;
      }
      var steps = this.newWf.steps.map(function(s) {
        var step = {
          name: s.name || 'step',
          agent_name: s.agent_name,
          mode: s.mode,
          prompt: s.prompt || '{{input}}',
          error_mode: s.error_mode || 'fail',
          timeout_secs: parseInt(s.timeout_secs) || 120
        };
        if (s.output_var) step.output_var = s.output_var;
        if (s.mode === 'conditional' && s.condition) step.condition = s.condition;
        if (s.mode === 'loop') {
          step.max_iterations = parseInt(s.max_iterations) || 5;
          if (s.until) step.until = s.until;
        }
        return step;
      });
      try {
        var wfName = this.newWf.name;
        await RustyHandAPI.post('/api/workflows', {
          name: wfName,
          description: this.newWf.description,
          steps: steps
        });
        this.showCreateModal = false;
        RustyHandToast.success('Workflow "' + wfName + '" created');
        await this.loadWorkflows();
      } catch(e) {
        RustyHandToast.error('Failed to create workflow: ' + e.message);
      }
    },

    showRunModal(wf) {
      this.runModal = wf;
      this.runInput = '';
      this.runResult = '';
    },

    async executeWorkflow() {
      if (!this.runModal) return;
      this.running = true;
      this.runResult = '';
      try {
        var res = await RustyHandAPI.post('/api/workflows/' + this.runModal.id + '/run', { input: this.runInput });
        this.runResult = res.output || JSON.stringify(res, null, 2);
        RustyHandToast.success('Workflow completed');
      } catch(e) {
        this.runResult = 'Error: ' + e.message;
        RustyHandToast.error('Workflow failed: ' + e.message);
      }
      this.running = false;
    },

    async viewRuns(wf) {
      try {
        var runs = await RustyHandAPI.get('/api/workflows/' + wf.id + '/runs');
        this.runResult = JSON.stringify(runs, null, 2);
        this.runModal = wf;
      } catch(e) {
        RustyHandToast.error('Failed to load run history: ' + e.message);
      }
    }
  };
}
