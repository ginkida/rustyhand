// RustyHand Visual Workflow Builder — Drag-and-drop workflow designer
'use strict';

function workflowBuilder() {
  return {
    // -- Canvas state --
    nodes: [],
    connections: [],
    selectedNode: null,
    selectedConnection: null,
    dragging: null,
    dragOffset: { x: 0, y: 0 },
    connecting: null, // { fromId, fromPort }
    connectPreview: null, // { x, y } mouse position during connect drag
    canvasOffset: { x: 0, y: 0 },
    canvasDragging: false,
    canvasDragStart: { x: 0, y: 0 },
    zoom: 1,
    nextId: 1,
    workflowName: '',
    workflowDescription: '',
    showSaveModal: false,
    showNodeEditor: false,
    showTomlPreview: false,
    tomlOutput: '',
    agents: [],
    agentsLoadError: '',
    _canvasEl: null,

    // Node types with their configs
    nodeTypes: [
      { type: 'agent', label: 'Agent Step', color: '#6366f1', icon: 'A', ports: { in: 1, out: 1 } },
      { type: 'parallel', label: 'Parallel Fan-out', color: '#f59e0b', icon: 'P', ports: { in: 1, out: 3 } },
      { type: 'condition', label: 'Condition', color: '#10b981', icon: '?', ports: { in: 1, out: 2 } },
      { type: 'loop', label: 'Loop', color: '#ef4444', icon: 'L', ports: { in: 1, out: 1 } },
      { type: 'collect', label: 'Collect', color: '#8b5cf6', icon: 'C', ports: { in: 3, out: 1 } },
      { type: 'start', label: 'Start', color: '#22c55e', icon: 'S', ports: { in: 0, out: 1 } },
      { type: 'end', label: 'End', color: '#ef4444', icon: 'E', ports: { in: 1, out: 0 } }
    ],

    async init() {
      var self = this;
      await this.loadAgents();
      // Add default start node
      self.addNode('start', 60, 200);
    },

    async loadAgents() {
      try {
        var list = await RustyHandAPI.get('/api/agents');
        this.agents = Array.isArray(list) ? list : [];
        this.agentsLoadError = '';
      } catch(e) {
        this.agents = Alpine.store('app').agents || [];
        this.agentsLoadError = e.message || 'Could not load agents.';
      }
    },

    hasAgentNameOption(name) {
      if (!name) return false;
      for (var i = 0; i < this.agents.length; i++) {
        if (this.agents[i].name === name) return true;
      }
      return false;
    },

    // ── Node Management ──────────────────────────────────

    addNode: function(type, x, y) {
      var def = null;
      for (var i = 0; i < this.nodeTypes.length; i++) {
        if (this.nodeTypes[i].type === type) { def = this.nodeTypes[i]; break; }
      }
      if (!def) return;
      var node = {
        id: 'node-' + this.nextId++,
        type: type,
        label: def.label,
        color: def.color,
        icon: def.icon,
        x: x || 200,
        y: y || 200,
        width: 180,
        height: 70,
        ports: { in: def.ports.in, out: def.ports.out },
        config: {}
      };
      if (type === 'agent') {
        node.config = { agent_name: '', prompt: '{{input}}', model: '' };
      } else if (type === 'condition') {
        node.config = { expression: '', true_label: 'Yes', false_label: 'No' };
      } else if (type === 'loop') {
        node.config = { max_iterations: 5, until: '' };
      } else if (type === 'parallel') {
        node.config = { fan_count: 3 };
      } else if (type === 'collect') {
        node.config = { strategy: 'all' };
      }
      this.nodes.push(node);
      return node;
    },

    deleteNode: function(nodeId) {
      this.connections = this.connections.filter(function(c) {
        return c.from !== nodeId && c.to !== nodeId;
      });
      this.nodes = this.nodes.filter(function(n) { return n.id !== nodeId; });
      if (this.selectedNode && this.selectedNode.id === nodeId) {
        this.selectedNode = null;
        this.showNodeEditor = false;
      }
    },

    duplicateNode: function(node) {
      var newNode = this.addNode(node.type, node.x + 30, node.y + 30);
      if (newNode) {
        newNode.config = JSON.parse(JSON.stringify(node.config));
        newNode.label = node.label + ' copy';
      }
    },

    getNode: function(id) {
      for (var i = 0; i < this.nodes.length; i++) {
        if (this.nodes[i].id === id) return this.nodes[i];
      }
      return null;
    },

    // ── Port Positions ───────────────────────────────────

    getInputPortPos: function(node, portIndex) {
      var total = node.ports.in;
      var spacing = node.width / (total + 1);
      return { x: node.x + spacing * (portIndex + 1), y: node.y };
    },

    getOutputPortPos: function(node, portIndex) {
      var total = node.ports.out;
      var spacing = node.width / (total + 1);
      return { x: node.x + spacing * (portIndex + 1), y: node.y + node.height };
    },

    // ── Connection Management ────────────────────────────

    startConnect: function(nodeId, portIndex, e) {
      e.stopPropagation();
      this.connecting = { fromId: nodeId, fromPort: portIndex };
      var node = this.getNode(nodeId);
      var pos = this.getOutputPortPos(node, portIndex);
      this.connectPreview = { x: pos.x, y: pos.y };
    },

    endConnect: function(nodeId, portIndex, e) {
      e.stopPropagation();
      if (!this.connecting) return;
      if (this.connecting.fromId === nodeId) {
        this.connecting = null;
        this.connectPreview = null;
        return;
      }
      // Check for duplicate
      var fromId = this.connecting.fromId;
      var fromPort = this.connecting.fromPort;
      var dup = false;
      for (var i = 0; i < this.connections.length; i++) {
        var c = this.connections[i];
        if (c.from === fromId && c.fromPort === fromPort && c.to === nodeId && c.toPort === portIndex) {
          dup = true;
          break;
        }
      }
      if (!dup) {
        this.connections.push({
          id: 'conn-' + this.nextId++,
          from: fromId,
          fromPort: fromPort,
          to: nodeId,
          toPort: portIndex
        });
      }
      this.connecting = null;
      this.connectPreview = null;
    },

    deleteConnection: function(connId) {
      this.connections = this.connections.filter(function(c) { return c.id !== connId; });
      this.selectedConnection = null;
    },

    // ── Drag Handling ────────────────────────────────────

    onNodeMouseDown: function(node, e) {
      e.stopPropagation();
      this.selectedNode = node;
      this.selectedConnection = null;
      this.dragging = node.id;
      var rect = this._getCanvasRect();
      this.dragOffset = {
        x: (e.clientX - rect.left) / this.zoom - this.canvasOffset.x - node.x,
        y: (e.clientY - rect.top) / this.zoom - this.canvasOffset.y - node.y
      };
    },

    onCanvasMouseDown: function(e) {
      if (e.target.closest('.wf-node') || e.target.closest('.wf-port')) return;
      this.selectedNode = null;
      this.selectedConnection = null;
      this.showNodeEditor = false;
      // Start canvas pan
      this.canvasDragging = true;
      this.canvasDragStart = { x: e.clientX - this.canvasOffset.x * this.zoom, y: e.clientY - this.canvasOffset.y * this.zoom };
    },

    onCanvasMouseMove: function(e) {
      var rect = this._getCanvasRect();
      if (this.dragging) {
        var node = this.getNode(this.dragging);
        if (node) {
          node.x = Math.max(0, (e.clientX - rect.left) / this.zoom - this.canvasOffset.x - this.dragOffset.x);
          node.y = Math.max(0, (e.clientY - rect.top) / this.zoom - this.canvasOffset.y - this.dragOffset.y);
        }
      } else if (this.connecting) {
        this.connectPreview = {
          x: (e.clientX - rect.left) / this.zoom - this.canvasOffset.x,
          y: (e.clientY - rect.top) / this.zoom - this.canvasOffset.y
        };
      } else if (this.canvasDragging) {
        this.canvasOffset = {
          x: (e.clientX - this.canvasDragStart.x) / this.zoom,
          y: (e.clientY - this.canvasDragStart.y) / this.zoom
        };
      }
    },

    onCanvasMouseUp: function() {
      this.dragging = null;
      this.connecting = null;
      this.connectPreview = null;
      this.canvasDragging = false;
    },

    onCanvasWheel: function(e) {
      e.preventDefault();
      var delta = e.deltaY > 0 ? -0.05 : 0.05;
      this.zoom = Math.max(0.3, Math.min(2, this.zoom + delta));
    },

    _getCanvasRect: function() {
      if (!this._canvasEl) {
        this._canvasEl = document.getElementById('wf-canvas');
      }
      return this._canvasEl ? this._canvasEl.getBoundingClientRect() : { left: 0, top: 0 };
    },

    // ── Connection Path ──────────────────────────────────

    getConnectionPath: function(conn) {
      var fromNode = this.getNode(conn.from);
      var toNode = this.getNode(conn.to);
      if (!fromNode || !toNode) return '';
      var from = this.getOutputPortPos(fromNode, conn.fromPort);
      var to = this.getInputPortPos(toNode, conn.toPort);
      var dy = Math.abs(to.y - from.y);
      var cp = Math.max(40, dy * 0.5);
      return 'M ' + from.x + ' ' + from.y + ' C ' + from.x + ' ' + (from.y + cp) + ' ' + to.x + ' ' + (to.y - cp) + ' ' + to.x + ' ' + to.y;
    },

    getPreviewPath: function() {
      if (!this.connecting || !this.connectPreview) return '';
      var fromNode = this.getNode(this.connecting.fromId);
      if (!fromNode) return '';
      var from = this.getOutputPortPos(fromNode, this.connecting.fromPort);
      var to = this.connectPreview;
      var dy = Math.abs(to.y - from.y);
      var cp = Math.max(40, dy * 0.5);
      return 'M ' + from.x + ' ' + from.y + ' C ' + from.x + ' ' + (from.y + cp) + ' ' + to.x + ' ' + (to.y - cp) + ' ' + to.x + ' ' + to.y;
    },

    // ── Node editor ──────────────────────────────────────

    editNode: function(node) {
      this.selectedNode = node;
      this.showNodeEditor = true;
    },

    // ── TOML Generation ──────────────────────────────────

    generateToml: function() {
      var self = this;
      var lines = [];
      lines.push('[workflow]');
      lines.push('name = "' + (this.workflowName || 'untitled') + '"');
      lines.push('description = "' + (this.workflowDescription || '') + '"');
      lines.push('');

      // Topological sort the nodes (skip start/end for step generation)
      var stepNodes = this.nodes.filter(function(n) {
        return n.type !== 'start' && n.type !== 'end';
      });

      for (var i = 0; i < stepNodes.length; i++) {
        var node = stepNodes[i];
        lines.push('[[workflow.steps]]');
        lines.push('name = "' + (node.label || 'step-' + (i + 1)) + '"');

        if (node.type === 'agent') {
          lines.push('type = "agent"');
          if (node.config.agent_name) lines.push('agent_name = "' + node.config.agent_name + '"');
          lines.push('prompt = "' + (node.config.prompt || '{{input}}') + '"');
          if (node.config.model) lines.push('model = "' + node.config.model + '"');
        } else if (node.type === 'parallel') {
          lines.push('type = "fan_out"');
          lines.push('fan_count = ' + (node.config.fan_count || 3));
        } else if (node.type === 'condition') {
          lines.push('type = "conditional"');
          lines.push('expression = "' + (node.config.expression || '') + '"');
        } else if (node.type === 'loop') {
          lines.push('type = "loop"');
          lines.push('max_iterations = ' + (node.config.max_iterations || 5));
          if (node.config.until) lines.push('until = "' + node.config.until + '"');
        } else if (node.type === 'collect') {
          lines.push('type = "collect"');
          lines.push('strategy = "' + (node.config.strategy || 'all') + '"');
        }

        // Find what this node connects to
        var outConns = self.connections.filter(function(c) { return c.from === node.id; });
        if (outConns.length === 1) {
          var target = self.getNode(outConns[0].to);
          if (target && target.type !== 'end') {
            lines.push('next = "' + target.label + '"');
          }
        } else if (outConns.length > 1 && node.type === 'condition') {
          for (var j = 0; j < outConns.length; j++) {
            var t2 = self.getNode(outConns[j].to);
            if (t2 && t2.type !== 'end') {
              var branchLabel = j === 0 ? 'true' : 'false';
              lines.push('next_' + branchLabel + ' = "' + t2.label + '"');
            }
          }
        } else if (outConns.length > 1 && node.type === 'parallel') {
          var targets = [];
          for (var k = 0; k < outConns.length; k++) {
            var t3 = self.getNode(outConns[k].to);
            if (t3 && t3.type !== 'end') targets.push('"' + t3.label + '"');
          }
          if (targets.length) lines.push('fan_targets = [' + targets.join(', ') + ']');
        }

        lines.push('');
      }

      this.tomlOutput = lines.join('\n');
      this.showTomlPreview = true;
    },

    // ── Save Workflow ────────────────────────────────────

    async saveWorkflow() {
      var steps = [];
      var stepNodes = this.nodes.filter(function(n) {
        return n.type !== 'start' && n.type !== 'end';
      });
      for (var i = 0; i < stepNodes.length; i++) {
        var node = stepNodes[i];
        var step = {
          name: node.label || 'step-' + (i + 1),
          mode: node.type === 'parallel' ? 'fan_out' : node.type === 'loop' ? 'loop' : 'sequential'
        };
        if (node.type === 'agent') {
          step.agent_name = node.config.agent_name || '';
          step.prompt = node.config.prompt || '{{input}}';
        }
        steps.push(step);
      }
      try {
        await RustyHandAPI.post('/api/workflows', {
          name: this.workflowName || 'untitled',
          description: this.workflowDescription || '',
          steps: steps
        });
        RustyHandToast.success('Workflow saved!');
        this.showSaveModal = false;
      } catch(e) {
        RustyHandToast.error('Failed to save: ' + e.message);
      }
    },

    // ── Palette drop ─────────────────────────────────────

    onPaletteDragStart: function(type, e) {
      e.dataTransfer.setData('text/plain', type);
      e.dataTransfer.effectAllowed = 'copy';
    },

    onCanvasDrop: function(e) {
      e.preventDefault();
      var type = e.dataTransfer.getData('text/plain');
      if (!type) return;
      var rect = this._getCanvasRect();
      var x = (e.clientX - rect.left) / this.zoom - this.canvasOffset.x;
      var y = (e.clientY - rect.top) / this.zoom - this.canvasOffset.y;
      this.addNode(type, x - 90, y - 35);
    },

    onCanvasDragOver: function(e) {
      e.preventDefault();
      e.dataTransfer.dropEffect = 'copy';
    },

    // ── Auto Layout ──────────────────────────────────────

    autoLayout: function() {
      // Simple top-to-bottom layout
      var y = 40;
      var x = 200;
      for (var i = 0; i < this.nodes.length; i++) {
        this.nodes[i].x = x;
        this.nodes[i].y = y;
        y += 120;
      }
    },

    // ── Clear ────────────────────────────────────────────

    clearCanvas: function() {
      this.nodes = [];
      this.connections = [];
      this.selectedNode = null;
      this.nextId = 1;
      this.addNode('start', 60, 200);
    },

    // ── Zoom controls ────────────────────────────────────

    zoomIn: function() {
      this.zoom = Math.min(2, this.zoom + 0.1);
    },

    zoomOut: function() {
      this.zoom = Math.max(0.3, this.zoom - 0.1);
    },

    zoomReset: function() {
      this.zoom = 1;
      this.canvasOffset = { x: 0, y: 0 };
    }
  };
}
