// RustyHand Approvals Page — Execution approval queue for sensitive agent actions
'use strict';

function approvalsPage() {
  return {
    approvals: [],
    filterStatus: 'all',
    loading: true,
    loadError: '',
    _pollTimer: null,
    _tickTimer: null,
    now: Date.now(),

    get filtered() {
      var f = this.filterStatus;
      var list = f === 'all' ? this.approvals : this.approvals.filter(function(a) { return a.status === f; });
      // Pending first, then by time descending
      return list.slice().sort(function(a, b) {
        if (a.status === 'pending' && b.status !== 'pending') return -1;
        if (a.status !== 'pending' && b.status === 'pending') return 1;
        return new Date(b.requested_at || b.created_at || 0) - new Date(a.requested_at || a.created_at || 0);
      });
    },

    get pendingCount() {
      return this.approvals.filter(function(a) { return a.status === 'pending'; }).length;
    },

    get approvedCount() {
      return this.approvals.filter(function(a) { return a.status === 'approved'; }).length;
    },

    get rejectedCount() {
      return this.approvals.filter(function(a) { return a.status === 'rejected' || a.status === 'expired'; }).length;
    },

    riskColor(level) {
      var colors = { low: 'var(--info)', medium: 'var(--warning)', high: 'var(--error)', critical: 'var(--error)' };
      return colors[level] || 'var(--text-dim)';
    },

    riskIcon(level) {
      var icons = { low: '\u2139\uFE0F', medium: '\u26A0\uFE0F', high: '\uD83D\uDEA8', critical: '\u2620\uFE0F' };
      return icons[level] || '\u2753';
    },

    riskLabel(level) {
      if (!level) return '';
      return level.charAt(0).toUpperCase() + level.slice(1);
    },

    timeRemaining(a) {
      if (a.status !== 'pending' || !a.requested_at || !a.timeout_secs) return '';
      var deadline = new Date(a.requested_at).getTime() + a.timeout_secs * 1000;
      var remaining = Math.max(0, Math.floor((deadline - this.now) / 1000));
      if (remaining <= 0) return 'Expired';
      var min = Math.floor(remaining / 60);
      var sec = remaining % 60;
      return min > 0 ? min + 'm ' + sec + 's' : sec + 's';
    },

    timeRemainingPct(a) {
      if (a.status !== 'pending' || !a.requested_at || !a.timeout_secs) return 0;
      var deadline = new Date(a.requested_at).getTime() + a.timeout_secs * 1000;
      var remaining = Math.max(0, deadline - this.now);
      return Math.min(100, (remaining / (a.timeout_secs * 1000)) * 100);
    },

    init() {
      var self = this;
      this._pollTimer = setInterval(function() { self.loadData(); }, 8000);
      this._tickTimer = setInterval(function() { self.now = Date.now(); }, 1000);
    },

    destroy() {
      if (this._pollTimer) { clearInterval(this._pollTimer); this._pollTimer = null; }
      if (this._tickTimer) { clearInterval(this._tickTimer); this._tickTimer = null; }
    },

    async loadData() {
      this.loadError = '';
      try {
        var data = await RustyHandAPI.get('/api/approvals');
        this.approvals = data.approvals || [];
        Alpine.store('app').pendingApprovals = this.pendingCount;
      } catch(e) {
        this.loadError = e.message || 'Could not load approvals.';
      }
      this.loading = false;
    },

    async approve(id) {
      try {
        await RustyHandAPI.post('/api/approvals/' + id + '/approve', {});
        RustyHandToast.success('Approved');
        await this.loadData();
      } catch(e) {
        RustyHandToast.error(e.message);
      }
    },

    async reject(id) {
      var self = this;
      RustyHandToast.confirm('Reject Action', 'Are you sure you want to reject this action?', async function() {
        try {
          await RustyHandAPI.post('/api/approvals/' + id + '/reject', {});
          RustyHandToast.success('Rejected');
          await self.loadData();
        } catch(e) {
          RustyHandToast.error(e.message);
        }
      });
    }
  };
}
