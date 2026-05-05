// RustyHand App — Alpine.js init, hash router, global store
'use strict';

// Marked.js configuration
if (typeof marked !== 'undefined') {
  marked.setOptions({
    breaks: true,
    gfm: true,
    highlight: function(code, lang) {
      if (typeof hljs !== 'undefined' && lang && hljs.getLanguage(lang)) {
        try { return hljs.highlight(code, { language: lang }).value; } catch(e) {}
      }
      return code;
    }
  });
}

function escapeHtml(text) {
  var div = document.createElement('div');
  div.textContent = text || '';
  return div.innerHTML;
}

function renderMarkdown(text) {
  if (!text) return '';
  // Strip <think>...</think> blocks from reasoning models (DeepSeek, QwQ, etc.)
  text = text.replace(/<think>[\s\S]*?<\/think>\s*/g, '');
  // Also strip unclosed <think> blocks (streaming may cut off before </think>)
  text = text.replace(/<think>[\s\S]*$/g, '');
  if (!text.trim()) return '';
  if (typeof marked !== 'undefined') {
    var html = marked.parse(text);
    // Sanitize: strip dangerous tags/attributes to prevent XSS from agent responses
    html = sanitizeHtml(html);
    // Add copy buttons to code blocks
    html = html.replace(/<pre><code/g, '<pre><button class="copy-btn" onclick="copyCode(this)">Copy</button><code');
    return html;
  }
  return escapeHtml(text);
}

// Lightweight HTML sanitizer — strips dangerous tags/attributes from rendered markdown.
// NOTE: For maximum protection, consider migrating to DOMPurify in the future.
function sanitizeHtml(html) {
  // Remove dangerous tags entirely (including self-closing variants)
  var dangerousTags = 'script|iframe|object|embed|applet|form|input|button|textarea|select|style|link|meta|base|svg|math|audio|video|source|details|summary|dialog|template|slot|portal|noscript|plaintext|xmp|listing';
  html = html.replace(new RegExp('<\\s*(' + dangerousTags + ')([^>]*)>[\\s\\S]*?<\\/\\s*\\1\\s*>', 'gi'), '');
  html = html.replace(new RegExp('<\\s*(' + dangerousTags + ')([^>]*)\\/?>', 'gi'), '');
  // Remove event handler attributes (on*) — covers onload, onerror, ontoggle, etc.
  html = html.replace(/\s+on\w+\s*=\s*("[^"]*"|'[^']*'|[^\s>]*)/gi, '');
  // Remove javascript:/vbscript: URIs and non-image data: URIs in href/src/action
  html = html.replace(/(href|src|action)\s*=\s*["']?\s*(?:javascript|vbscript|data\s*:(?!image\/))/gi, '$1="');
  // Strip style attributes containing dangerous CSS (url(), expression(), -moz-binding)
  html = html.replace(/style\s*=\s*"[^"]*(?:url\s*\(|expression\s*\(|-moz-binding)[^"]*"/gi, '');
  html = html.replace(/style\s*=\s*'[^']*(?:url\s*\(|expression\s*\(|-moz-binding)[^']*'/gi, '');
  return html;
}

function copyCode(btn) {
  var code = btn.nextElementSibling;
  if (code) {
    navigator.clipboard.writeText(code.textContent).then(function() {
      btn.textContent = 'Copied!';
      btn.classList.add('copied');
      setTimeout(function() { btn.textContent = 'Copy'; btn.classList.remove('copied'); }, 1500);
    });
  }
}

// Tool category icon SVGs — returns inline SVG for each tool category
function toolIcon(toolName) {
  if (!toolName) return '';
  var n = toolName.toLowerCase();
  var s = 'width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"';
  // File/directory operations
  if (n.indexOf('file_') === 0 || n.indexOf('directory_') === 0)
    return '<svg ' + s + '><path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"/><path d="M14 2v6h6"/><path d="M16 13H8"/><path d="M16 17H8"/></svg>';
  // Web/fetch
  if (n.indexOf('web_') === 0 || n.indexOf('link_') === 0)
    return '<svg ' + s + '><circle cx="12" cy="12" r="10"/><path d="M2 12h20"/><path d="M12 2a15 15 0 0 1 4 10 15 15 0 0 1-4 10 15 15 0 0 1-4-10 15 15 0 0 1 4-10z"/></svg>';
  // Shell/exec
  if (n.indexOf('shell') === 0 || n.indexOf('exec_') === 0)
    return '<svg ' + s + '><polyline points="4 17 10 11 4 5"/><line x1="12" y1="19" x2="20" y2="19"/></svg>';
  // Agent operations
  if (n.indexOf('agent_') === 0)
    return '<svg ' + s + '><path d="M17 21v-2a4 4 0 0 0-4-4H5a4 4 0 0 0-4 4v2"/><circle cx="9" cy="7" r="4"/><path d="M23 21v-2a4 4 0 0 0-3-3.87"/><path d="M16 3.13a4 4 0 0 1 0 7.75"/></svg>';
  // Memory/knowledge
  if (n.indexOf('memory_') === 0 || n.indexOf('knowledge_') === 0)
    return '<svg ' + s + '><path d="M2 3h6a4 4 0 0 1 4 4v14a3 3 0 0 0-3-3H2z"/><path d="M22 3h-6a4 4 0 0 0-4 4v14a3 3 0 0 1 3-3h7z"/></svg>';
  // Cron/schedule
  if (n.indexOf('cron_') === 0 || n.indexOf('schedule_') === 0)
    return '<svg ' + s + '><circle cx="12" cy="12" r="10"/><polyline points="12 6 12 12 16 14"/></svg>';
  // Browser/playwright
  if (n.indexOf('browser_') === 0 || n.indexOf('playwright_') === 0)
    return '<svg ' + s + '><rect x="2" y="3" width="20" height="14" rx="2"/><path d="M8 21h8"/><path d="M12 17v4"/></svg>';
  // Container/docker
  if (n.indexOf('container_') === 0 || n.indexOf('docker_') === 0)
    return '<svg ' + s + '><path d="M22 12H2"/><path d="M5.45 5.11L2 12v6a2 2 0 0 0 2 2h16a2 2 0 0 0 2-2v-6l-3.45-6.89A2 2 0 0 0 16.76 4H7.24a2 2 0 0 0-1.79 1.11z"/></svg>';
  // Image/media
  if (n.indexOf('image_') === 0 || n.indexOf('tts_') === 0)
    return '<svg ' + s + '><rect x="3" y="3" width="18" height="18" rx="2"/><circle cx="8.5" cy="8.5" r="1.5"/><polyline points="21 15 16 10 5 21"/></svg>';
  // Task/collab
  if (n.indexOf('task_') === 0)
    return '<svg ' + s + '><path d="M9 11l3 3L22 4"/><path d="M21 12v7a2 2 0 01-2 2H5a2 2 0 01-2-2V5a2 2 0 012-2h11"/></svg>';
  // Default — wrench
  return '<svg ' + s + '><path d="M14.7 6.3a1 1 0 0 0 0 1.4l1.6 1.6a1 1 0 0 0 1.4 0l3.77-3.77a6 6 0 0 1-7.94 7.94l-6.91 6.91a2.12 2.12 0 0 1-3-3l6.91-6.91a6 6 0 0 1 7.94-7.94l-3.76 3.76z"/></svg>';
}

// One-time migration: of-* → rh-* localStorage keys
(function() {
  if (localStorage.getItem('rh-migrated')) return;
  var map = {
    'of-tips-off': 'rh-tips-off',
    'of-chat-tips-seen': 'rh-chat-tips-seen',
    'of-first-msg': 'rh-first-msg',
    'of-checklist-dismissed': 'rh-checklist-dismissed',
    'of-skill-browsed': 'rh-skill-browsed'
  };
  for (var oldKey in map) {
    var val = localStorage.getItem(oldKey);
    if (val !== null) {
      localStorage.setItem(map[oldKey], val);
      localStorage.removeItem(oldKey);
    }
  }
  localStorage.setItem('rh-migrated', 'true');
})();

// Alpine.js global store
document.addEventListener('alpine:init', function() {
  Alpine.store('app', {
    agents: [],
    defaultProvider: '',
    defaultModel: '',
    connected: false,
    booting: true,
    wsConnected: false,
    _wasConnected: false,
    connectionState: 'connected',
    lastPingMs: 0,
    lastError: '',
    agentRefreshError: '',
    approvalsRefreshError: '',
    version: '0.1.0',
    agentCount: 0,
    agentsTotal: 0,
    agentsOffset: 0,
    agentsLimit: 100,
    pendingAgent: null,
    pendingSession: null,
    pendingApprovals: 0,
    filterState: 'all',
    agentQuery: '',
    // Server-side search state (used when local list >= SERVER_SEARCH_THRESHOLD agents)
    _searchResultsFor: '',   // query string the current results correspond to
    searchResults: [],       // last response from /api/agents/search
    _searchTimer: null,      // debounce handle
    templatesExpanded: false,
    pinnedAgentIds: (function() {
      try { return JSON.parse(localStorage.getItem('rh-pinned-agents') || '[]'); } catch(e) { return []; }
    })(),
    agentGroupsCollapsed: (function() {
      try {
        return JSON.parse(localStorage.getItem('rusty-hand-agent-groups') || '{}') || {};
      } catch(e) {
        return {};
      }
    })(),
    activeChatAgent: null,
    pendingTemplate: null,
    builtinTemplates: [
      { name: 'General Assistant', description: 'A versatile conversational agent for everyday tasks.', category: 'General', profile: 'full', system_prompt: 'You are a helpful, friendly assistant. Provide clear, accurate, and concise responses. Ask clarifying questions when needed.' },
      { name: 'Code Helper', description: 'A programming-focused agent for writing, reviewing, and debugging code.', category: 'Development', profile: 'coding', system_prompt: 'You are an expert programmer. Help users write clean, efficient code. Explain your reasoning. Follow best practices and conventions for the language being used.' },
      { name: 'Researcher', description: 'An analytical agent that synthesizes information and provides cited summaries.', category: 'Research', profile: 'research', system_prompt: 'You are a research analyst. Break down complex topics into clear explanations. Provide structured analysis with key findings. Cite sources when available.' },
      { name: 'Writer', description: 'A creative writing agent for drafting, editing, and improving written content.', category: 'Writing', profile: 'full', system_prompt: 'You are a skilled writer and editor. Help users create polished content. Adapt your tone and style to match the intended audience. Offer constructive suggestions for improvement.' },
      { name: 'Data Analyst', description: 'A data-focused agent for analyzing datasets and interpreting results.', category: 'Development', profile: 'coding', system_prompt: 'You are a data analysis expert. Help users understand their data, write SQL/Python queries, and interpret results. Present findings clearly with actionable insights.' },
      { name: 'DevOps Engineer', description: 'A systems-focused agent for CI/CD, Docker, and deployment.', category: 'Development', profile: 'automation', system_prompt: 'You are a DevOps engineer. Help with CI/CD pipelines, Docker, Kubernetes, infrastructure as code, and deployment. Prioritize reliability and security.' },
      { name: 'Customer Support', description: 'A professional, empathetic agent for handling customer inquiries.', category: 'Business', profile: 'messaging', system_prompt: 'You are a professional customer support representative. Be empathetic, patient, and solution-oriented. Acknowledge concerns before offering solutions. Escalate complex issues appropriately.' },
      { name: 'Tutor', description: 'A patient educational agent that explains concepts step-by-step.', category: 'General', profile: 'full', system_prompt: 'You are a patient and encouraging tutor. Explain concepts step by step, starting from fundamentals. Use analogies and examples. Check understanding before moving on. Adapt to the learner\'s pace.' },
      { name: 'API Designer', description: 'An agent specialized in RESTful API design and integration architecture.', category: 'Development', profile: 'coding', system_prompt: 'You are an API design expert. Help users design clean, consistent RESTful APIs following best practices. Cover endpoint naming, request/response schemas, error handling, and versioning.' },
      { name: 'Meeting Notes', description: 'Summarizes meeting transcripts into structured notes with action items.', category: 'Business', profile: 'minimal', system_prompt: 'You are a meeting summarizer. When given a meeting transcript or notes, produce a structured summary with: key decisions, action items (with owners), discussion highlights, and follow-up questions.' },
      { name: 'GitHub Monitor', description: 'Autonomous code auditor — clones repos, runs tests, detects bugs, creates GitHub Issues.', category: 'Development', profile: 'coding', autonomous: true, cronExpr: '0 9 * * 1-5', cronMessage: 'Check all monitored repositories for new commits, run tests, detect bugs, and file GitHub Issues.', system_prompt: 'You are Code Analyzer — an autonomous agent that clones source code repositories, runs their test suites, performs static analysis, identifies bugs and problems, and files GitHub Issues for every finding. You run on a recurring schedule, detect new commits, avoid duplicate Issues, and automatically close Issues when bugs are fixed.\n\nPhase 0: State Recovery & Schedule Setup — detect OS, verify git and GitHub CLI are available, load previous state from memory, parse configured repo URLs.\nPhase 1: Process Repositories — for each repo: clone or pull latest, check for new commits since last run, understand project structure.\nPhase 2: Run Tests — execute language-specific test runners (Go, Rust, Node.js, Python).\nPhase 3: Static Analysis — run linting, pattern-based analysis, dependency audit, code structure review.\nPhase 4: Classify Findings — assign severity and category, check for existing GitHub Issues, close issues for fixed bugs.\nPhase 5: Create GitHub Issues — deduplicate findings, compose Issue with markdown body, create via GitHub API.\nPhase 6: Summary Report — per-repo report with findings table, issues created/closed.\nPhase 7: Save State — update aggregate metrics, persist state to memory.' },
      { name: 'Web Researcher', description: 'Autonomous deep researcher — exhaustive investigation, cross-referencing, and structured reports.', category: 'Research', profile: 'research', autonomous: true, cronExpr: '0 */6 * * *', cronMessage: 'Run your research cycle now.', system_prompt: 'You are Researcher — an autonomous deep research agent that conducts exhaustive investigations, cross-references sources, fact-checks claims, and produces comprehensive structured reports.\n\nPhase 0: Platform Detection & Context — detect OS, load previous state, read user config.\nPhase 1: Question Analysis & Decomposition — identify core question, decompose into sub-questions.\nPhase 2: Search Strategy Construction — build diverse search queries.\nPhase 3: Information Gathering — core loop: web_search, evaluate sources, web_fetch for depth.\nPhase 4: Cross-Reference & Synthesis — verify claims across multiple sources.\nPhase 5: Fact-Check Pass — verify critical claims, mark confidence levels.\nPhase 6: Report Generation — format structured report with citations and confidence scores.\nPhase 7: State & Statistics — save metrics and findings to memory.' },
      { name: 'Content Clipper', description: 'Turns long-form video into viral short clips with captions and thumbnails.', category: 'Content', profile: 'full', autonomous: true, cronExpr: '0 */4 * * *', cronMessage: 'Check for new videos to process.', system_prompt: 'You are Content Clipper — an AI-powered shorts factory that turns any video URL or file into viral short clips.\n\nPhase 0: Platform Detection — detect OS, set platform-specific commands.\nPhase 1: Intake — detect input type (URL or local file), gather metadata.\nPhase 2: Download — use yt-dlp for URLs, ffprobe for local files.\nPhase 3: Transcribe — try YouTube subtitles, then Whisper API, then local Whisper.\nPhase 4: Analyze & Pick Segments — read transcript, identify 3-5 viral segments (30-90 sec each).\nPhase 5: Extract & Process — extract clip, crop to vertical 9:16, generate SRT captions, burn captions.\nPhase 6: Publish — optional delivery via configured channels.\nPhase 7: Report — summary table with file paths, update metrics.' },
      { name: 'Lead Generator', description: 'Autonomous lead generation — discovers, enriches, and delivers qualified leads on schedule.', category: 'Business', profile: 'research', autonomous: true, cronExpr: '0 9 * * 1-5', cronMessage: 'Run lead generation cycle — discover and enrich new leads.', system_prompt: 'You are Lead Generator — an autonomous lead generation engine that discovers, enriches, and delivers qualified leads 24/7.\n\nPhase 0: Platform Detection — detect OS, verify tools.\nPhase 1: State Recovery & Schedule Setup — load previous state, create delivery schedule.\nPhase 2: Target Profile Construction — build ideal customer profile from configured settings.\nPhase 3: Lead Discovery — execute multi-query web research loop.\nPhase 4: Lead Enrichment — fetch company info, funding, news, social profiles.\nPhase 5: Deduplication & Scoring — compare against database, score leads 0-100.\nPhase 6: Report Generation — formatted lead report.\nPhase 7: State Persistence — update database and metrics.' },
      { name: 'Intel Collector', description: 'Autonomous intelligence collector — monitors targets with change detection and knowledge graphs.', category: 'Research', profile: 'research', autonomous: true, cronExpr: '0 */4 * * *', cronMessage: 'Run intelligence collection sweep on all monitored targets.', system_prompt: 'You are Intel Collector — an autonomous intelligence collector that monitors any target 24/7, building a living knowledge graph and detecting changes over time.\n\nPhase 0: Platform Detection & State Recovery — detect OS, load previous state.\nPhase 1: Schedule & Target Initialization — create schedule, identify target type, build query set.\nPhase 2: Source Discovery & Query Construction — build targeted searches based on focus area.\nPhase 3: Collection Sweep — web_search, web_fetch, extract entities, evaluate quality.\nPhase 4: Knowledge Graph Construction — add entities and relations for collected data.\nPhase 5: Change Detection & Delta Analysis — compare against previous snapshot, score changes.\nPhase 6: Report Generation — formatted report with change summary.\nPhase 7: State Persistence — save knowledge base and metrics.' },
      { name: 'Predictor', description: 'Autonomous forecasting — collects signals, builds reasoning chains, makes calibrated predictions.', category: 'Research', profile: 'research', autonomous: true, cronExpr: '0 8 * * *', cronMessage: 'Run prediction cycle — collect signals, update forecasts, score accuracy.', system_prompt: 'You are Predictor — an autonomous forecasting engine inspired by superforecasting principles. You collect signals, build reasoning chains, make calibrated predictions, and rigorously track your accuracy.\n\nPhase 0: Platform Detection & State Recovery — detect OS, load predictions and accuracy data.\nPhase 1: Schedule & Domain Setup — create report schedule, build domain-specific queries.\nPhase 2: Signal Collection — execute 20-40 targeted searches, tag signal types.\nPhase 3: Accuracy Review — score expired predictions, calculate Brier score, analyze calibration.\nPhase 4: Pattern Analysis & Reasoning Chains — gather signals, build reasoning, apply bias checks.\nPhase 5: Prediction Formulation — structure predictions with confidence, time horizon, reasoning.\nPhase 6: Report Generation — report with accuracy dashboard and active predictions.\nPhase 7: State Persistence — save predictions database and metrics.' },
      { name: 'Twitter Manager', description: 'Autonomous Twitter/X manager — content creation, scheduled posting, and engagement tracking.', category: 'Marketing', profile: 'full', autonomous: true, cronExpr: '0 */3 * * *', cronMessage: 'Run Twitter cycle — create content, check engagement, post scheduled tweets.', system_prompt: 'You are Twitter Manager — an autonomous Twitter/X content manager that creates, schedules, posts, and engages 24/7.\n\nPhase 0: Platform Detection & API Initialization — detect OS, verify Twitter API access.\nPhase 1: Schedule & Strategy Setup — create posting schedules based on configured frequency.\nPhase 2: Content Research & Trend Analysis — search trends, analyze performance.\nPhase 3: Content Generation — create tweets matching configured style.\nPhase 4: Content Queue & Posting — manage queue or direct posting based on approval mode.\nPhase 5: Engagement — check mentions, auto-reply, auto-like based on settings.\nPhase 6: Performance Tracking — check tweet metrics, analyze patterns.\nPhase 7: State Persistence — save queue and posting history.' },
      { name: 'Web Browser', description: 'Autonomous web browser — navigates sites, fills forms, clicks buttons, and completes web tasks.', category: 'Automation', profile: 'full', autonomous: true, cronExpr: '0 */6 * * *', cronMessage: 'Run scheduled browser automation tasks.', system_prompt: 'You are Web Browser — an autonomous web browser agent that interacts with real websites on behalf of the user.\n\nPhase 1: Understand the Task — parse request, plan approach.\nPhase 2: Navigate & Observe — navigate to target URL, read page content, identify interactive elements.\nPhase 3: Interact — click buttons, type in fields, read results, take screenshots.\nPhase 4: Purchase/Payment Approval — CRITICAL: always stop and ask before any purchase or payment.\nPhase 5: Report Results — summarize what was accomplished, provide screenshots of final state.' },
      { name: 'Coordinator', description: 'Meta-agent: routes user requests to the right specialist agent via agent_send.', category: 'Meta', profile: 'messaging', meta: true, system_prompt: 'You are Coordinator — a dispatcher agent. Inspect the user request, look up the pool of available agents, and delegate the task to the best-fit specialist via the agent_send tool. Wait for the specialist\'s reply and relay the final answer back to the user. Never try to solve tasks yourself when a better-suited agent exists.' },
      { name: 'Capability-Builder', description: 'Meta-agent: generates new skills on demand (python/node) and installs them via skill_install.', category: 'Meta', profile: 'coding', meta: true, system_prompt: 'You are Capability-Builder — an agent that expands RustyHand with new skills. When asked for a capability, draft a Python or Node skill that implements it, then call the skill_install tool with the generated code. Always validate the skill by running it once with representative input before reporting success.' },
      { name: 'Diagnostic', description: 'Meta-agent: read-only health auditor — inspects agent history, metrics, and API state.', category: 'Meta', profile: 'research', meta: true, system_prompt: 'You are Diagnostic — a read-only observability agent. Your job is to audit the RustyHand kernel: inspect agent history, metrics, and the local API (http://127.0.0.1:4200). Produce structured reports on errors, latency, cost, and capability usage. Never write files, never send messages to other agents, never install anything.' }
    ],

    normalizeAgentGroupLabel(group) {
      var normalized = (group || '').trim();
      return normalized || 'Ungrouped';
    },

    agentGroupKey(group) {
      var normalized = (group || '').trim();
      return normalized ? 'group:' + normalized.toLowerCase() : '__ungrouped__';
    },

    isAgentGroupCollapsed(groupKey) {
      if ((this.agentQuery || '').trim()) return false;
      return !!this.agentGroupsCollapsed[groupKey];
    },

    toggleAgentGroup(groupKey) {
      var nextState = Object.assign({}, this.agentGroupsCollapsed);
      if (nextState[groupKey]) delete nextState[groupKey];
      else nextState[groupKey] = true;
      this.agentGroupsCollapsed = nextState;
      localStorage.setItem('rusty-hand-agent-groups', JSON.stringify(nextState));
    },

    isPinned(agentId) {
      return this.pinnedAgentIds.indexOf(agentId) !== -1;
    },

    togglePin(agentId) {
      var ids = this.pinnedAgentIds.slice();
      var idx = ids.indexOf(agentId);
      if (idx === -1) ids.push(agentId);
      else ids.splice(idx, 1);
      this.pinnedAgentIds = ids;
      localStorage.setItem('rh-pinned-agents', JSON.stringify(ids));
    },

    get pinnedAgents() {
      var ids = this.pinnedAgentIds;
      if (!ids.length) return [];
      return this.agents.filter(function(a) { return ids.indexOf(a.id) !== -1; });
    },

    get filteredAgents() {
      var f = this.filterState;
      var q = (this.agentQuery || '').trim().toLowerCase();
      // If we have fresh server-side search results for this exact query,
      // use them (scales past 100+ agents without downloading everything).
      if (q && this._searchResultsFor === this.agentQuery.trim()) {
        return this.searchResults.filter(function(a) {
          if (f !== 'all' && (a.state || '').toLowerCase() !== f) return false;
          return true;
        });
      }
      // Fallback: local filter (for small installs or until server responds).
      return this.agents.filter(function(a) {
        if (f !== 'all' && a.state.toLowerCase() !== f) return false;
        if (!q) return true;
        var haystack = [
          a.name,
          a.group,
          a.model_name,
          a.model_provider,
          a.last_message_preview,
          a.id
        ]
          .join(' ')
          .toLowerCase();
        return haystack.indexOf(q) !== -1;
      }).sort(function(a, b) {
        var aTime = new Date(a.last_activity || a.created_at || 0).getTime();
        var bTime = new Date(b.last_activity || b.created_at || 0).getTime();
        if (aTime !== bTime) return bTime - aTime;
        return (a.name || '').localeCompare(b.name || '');
      });
    },

    // Stable grouped agents — NOT a computed getter to avoid Alpine reactivity thrashing.
    // Updated explicitly in _rebuildGroups() which is called from refreshAgents().
    groupedAgents: [],
    _groupedHash: '',

    _rebuildGroups() {
      var agents = this.filteredAgents;
      var hash = agents.map(function(a) { return a.id + ':' + (a.group || '') + ':' + a.state; }).join('|');
      if (hash === this._groupedHash) return;
      this._groupedHash = hash;

      var groups = {};
      agents.forEach(function(agent) {
        var rawGroup = (agent.group || '').trim();
        var key = rawGroup ? 'group:' + rawGroup.toLowerCase() : '__ungrouped__';
        if (!groups[key]) {
          groups[key] = {
            key: key,
            label: rawGroup || 'Ungrouped',
            is_ungrouped: !rawGroup,
            agent_count: 0,
            running_count: 0,
            latest_activity: 0,
            agents: []
          };
        }
        groups[key].agents.push(agent);
        groups[key].agent_count += 1;
        if (agent.state === 'running') groups[key].running_count += 1;
        groups[key].latest_activity = Math.max(
          groups[key].latest_activity,
          new Date(agent.last_activity || agent.created_at || 0).getTime()
        );
      });

      this.groupedAgents = Object.keys(groups)
        .map(function(key) {
          var group = groups[key];
          group.agents.sort(function(a, b) {
            var aTime = new Date(a.last_activity || a.created_at || 0).getTime();
            var bTime = new Date(b.last_activity || b.created_at || 0).getTime();
            if (aTime !== bTime) return bTime - aTime;
            return (a.name || '').localeCompare(b.name || '');
          });
          return group;
        })
        .sort(function(a, b) {
          if (a.is_ungrouped !== b.is_ungrouped) return a.is_ungrouped ? 1 : -1;
          return a.label.localeCompare(b.label);
        });
    },

    get runningCount() {
      return this.agents.filter(function(a) { return a.state === 'running'; }).length;
    },

    chatWithAgent(agent) {
      this.activeChatAgent = agent;
      this.pendingAgent = agent;
    },

    // Fire server-side search when the local list is big enough that
    // filtering all agents in memory is wasteful. Debounced on the caller.
    async _runServerSearch(query) {
        var q = (query || '').trim();
        if (!q) {
            this.searchResults = [];
            this._searchResultsFor = '';
            return;
        }
        try {
            var res = await RustyHandAPI.get(
                '/api/agents/search?q=' + encodeURIComponent(q) +
                '&state=' + encodeURIComponent(this.filterState === 'all' ? '' : this.filterState)
            );
            // The server returns the same shape as list_agents (agents envelope).
            var results = Array.isArray(res) ? res :
                (res && Array.isArray(res.results) ? res.results :
                (res && Array.isArray(res.agents) ? res.agents : []));
            this.searchResults = results;
            this._searchResultsFor = q;
        } catch (e) {
            // Network / 404: fall back to local filter silently (keeps UX smooth).
            console.warn('Agents search failed, using local filter:', e.message);
            this._searchResultsFor = '';
        }
    },

    // Watcher invoked from index_body.html on agentQuery changes.
    onAgentQueryChange() {
        var SERVER_SEARCH_THRESHOLD = 50;
        if (this._searchTimer) clearTimeout(this._searchTimer);
        // Local filter is instant and sufficient for small installs.
        if ((this.agents || []).length < SERVER_SEARCH_THRESHOLD) {
            this._searchResultsFor = '';
            this.searchResults = [];
            return;
        }
        var self = this;
        var q = this.agentQuery;
        this._searchTimer = setTimeout(function() {
            self._runServerSearch(q);
        }, 250);
    },

    async refreshAgents() {
      try {
        var _pingStart = performance.now();
        var requestedOffset = this.agentsOffset;
        var requestedLimit = this.agentsLimit;
        var url = '/api/agents?offset=' + requestedOffset + '&limit=' + requestedLimit;
        var fresh = await RustyHandAPI.get(url);
        this.lastPingMs = Math.round(performance.now() - _pingStart);
        // Discard stale responses: if the user clicked Next/Prev again while this
        // request was in flight, `agentsOffset` has already moved. Applying this
        // older response would overwrite the newer state and visibly "bounce" the page.
        if (this.agentsOffset !== requestedOffset) {
          return true;
        }
        // Support both paginated {agents: [...], total} and legacy array responses
        var freshList = Array.isArray(fresh) ? fresh : (fresh && fresh.agents ? fresh.agents : []);
        if (fresh && !Array.isArray(fresh)) {
          this.agentsTotal = fresh.total || freshList.length;
        } else {
          this.agentsTotal = freshList.length;
        }
        // If our offset ran past the real total (e.g. agents were deleted), snap back.
        if (this.agentsTotal > 0 && this.agentsOffset >= this.agentsTotal) {
          this.agentsOffset = Math.max(0, Math.floor((this.agentsTotal - 1) / this.agentsLimit) * this.agentsLimit);
          return this.refreshAgents();
        }
        // Update existing agents in-place to avoid flicker from full array replacement
        var existingById = {};
        for (var i = 0; i < this.agents.length; i++) {
          existingById[this.agents[i].id] = i;
        }
        var newIds = {};
        for (var j = 0; j < freshList.length; j++) {
          newIds[freshList[j].id] = true;
          var idx = existingById[freshList[j].id];
          if (idx !== undefined) {
            // Update existing entry in-place
            Object.assign(this.agents[idx], freshList[j]);
          } else {
            // New agent — append
            this.agents.push(freshList[j]);
          }
        }
        // Remove agents that no longer exist (iterate backwards)
        for (var k = this.agents.length - 1; k >= 0; k--) {
          if (!newIds[this.agents[k].id]) {
            this.agents.splice(k, 1);
          }
        }
        if (this.activeChatAgent && this.activeChatAgent.id) {
          var updatedActive = this.agents.find(function(agent) {
            return agent.id === this.activeChatAgent.id;
          }.bind(this));
          if (updatedActive) Object.assign(this.activeChatAgent, updatedActive);
        }
        if (this.pendingAgent && typeof this.pendingAgent === 'object' && this.pendingAgent.id) {
          var updatedPending = this.agents.find(function(agent) {
            return agent.id === this.pendingAgent.id;
          }.bind(this));
          if (updatedPending) Object.assign(this.pendingAgent, updatedPending);
        }
        this.agentCount = this.agents.length;
        this.agentRefreshError = '';
        this._rebuildGroups();
        return true;
      } catch(e) {
        this.agentRefreshError = e.message || 'Could not load agents.';
        console.warn('[RustyHand] Agent refresh failed:', this.agentRefreshError);
        return false;
      }
    },

    agentsPageNext() {
      var next = this.agentsOffset + this.agentsLimit;
      if (next >= this.agentsTotal) return;
      this.agentsOffset = next;
      this.refreshAgents();
    },

    agentsPagePrev() {
      if (this.agentsOffset <= 0) return;
      this.agentsOffset = Math.max(0, this.agentsOffset - this.agentsLimit);
      this.refreshAgents();
    },

    agentsPageLabel() {
      var total = this.agentsTotal || 0;
      if (total === 0) return '0 agents';
      var from = this.agentsOffset + 1;
      var to = Math.min(this.agentsOffset + this.agentsLimit, total);
      return from + '-' + to + ' of ' + total;
    },

    agentsHasNext() { return this.agentsOffset + this.agentsLimit < this.agentsTotal; },
    agentsHasPrev() { return this.agentsOffset > 0; },
    agentsIsPaginated() { return this.agentsTotal > this.agentsLimit; },

    async checkStatus() {
      try {
        var s = await RustyHandAPI.get('/api/status');
        this.connected = true;
        this.booting = false;
        this.lastError = '';
        this.version = s.version || '0.1.0';
        this.agentCount = s.agent_count || 0;
        this.defaultProvider = s.default_provider || '';
        this.defaultModel = s.default_model || '';
      } catch(e) {
        this.connected = false;
        this.lastError = e.message || 'Unknown error';
        console.warn('[RustyHand] Status check failed:', e.message);
      }
    },

    async refreshApprovals() {
      try {
        var data = await RustyHandAPI.get('/api/approvals');
        var arr = Array.isArray(data) ? data : (data.approvals || []);
        this.pendingApprovals = arr.filter(function(a) { return a.status === 'pending'; }).length;
        this.approvalsRefreshError = '';
        return true;
      } catch(e) {
        this.approvalsRefreshError = e.message || 'Could not load approvals.';
        console.warn('[RustyHand] Approval refresh failed:', this.approvalsRefreshError);
        return false;
      }
    },

    agentHue(name) {
      var h = 0;
      for (var i = 0; i < (name || '').length; i++) h = name.charCodeAt(i) + ((h << 5) - h);
      return Math.abs(h) % 360;
    }
  });
});

// Global time-ago formatter (used by sidebar, agents, automation, approvals)
function timeAgo(iso) {
  if (!iso) return '';
  var now = Date.now();
  var then = new Date(iso).getTime();
  var diff = Math.floor((now - then) / 1000);
  if (diff < 10) return 'just now';
  if (diff < 60) return diff + 's ago';
  if (diff < 3600) return Math.floor(diff / 60) + 'm ago';
  if (diff < 86400) return Math.floor(diff / 3600) + 'h ago';
  if (diff < 172800) return 'yesterday';
  return Math.floor(diff / 86400) + 'd ago';
}

// Global badge class for attention/tone items (used by approvals + settings)
function attentionBadgeClass(tone) {
  if (tone === 'danger') return 'badge-error';
  if (tone === 'warning') return 'badge-warn';
  return 'badge-info';
}

// Main app component
function app() {
  return {
    // ── Auth state ──
    authRequired: false,
    authenticated: false,
    loginToken: '',
    loginError: '',
    loginLoading: false,

    async checkAuth() {
      RustyHandAPI.initAuth();
      var needsAuth = await RustyHandAPI.checkAuthRequired();
      if (!needsAuth) {
        this.authRequired = false;
        this.authenticated = true;
        return;
      }
      this.authRequired = true;
      // Check if saved session token is still valid
      if (RustyHandAPI.getToken()) {
        var valid = await RustyHandAPI.validateToken(RustyHandAPI.getToken());
        if (valid) { this.authenticated = true; return; }
        RustyHandAPI.logout();
      }
      this.authenticated = false;
    },

    async login() {
      if (!this.loginToken.trim()) { this.loginError = 'Enter API key'; return; }
      this.loginLoading = true;
      this.loginError = '';
      var valid = await RustyHandAPI.validateToken(this.loginToken.trim());
      if (valid) {
        RustyHandAPI.setToken(this.loginToken.trim());
        this.authenticated = true;
        this.loginToken = '';
        Alpine.store('app').refreshAgents();
        Alpine.store('app').checkStatus();
      } else {
        this.loginError = 'Invalid API key';
      }
      this.loginLoading = false;
    },

    doLogout() {
      RustyHandAPI.logout();
      this.authenticated = false;
      this.loginToken = '';
      this.loginError = '';
    },

    page: 'agents',
    themeMode: localStorage.getItem('rusty-hand-theme-mode') || 'system',
    theme: (() => {
      var mode = localStorage.getItem('rusty-hand-theme-mode') || 'system';
      if (mode === 'system') return window.matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light';
      return mode;
    })(),
    sidebarCollapsed: localStorage.getItem('rusty-hand-sidebar') === 'collapsed',
    mobileMenuOpen: false,
    connected: false,
    wsConnected: false,
    version: '0.1.0',
    agentCount: 0,
    showShortcutsModal: false,
    showPalette: false,
    paletteQuery: '',
    paletteIdx: 0,
    paletteHistoryResults: [],
    _paletteHistoryFor: '',
    pageMeta: {
      agents: { title: 'Chat', section: 'Operations', description: 'Talk to running agents, inspect live context, and launch new conversations.', hotkey: '1', keywords: 'chat agents messages templates conversations' },
      approvals: { title: 'Approvals', section: 'Safety', description: 'Review sensitive actions waiting for a human decision.', hotkey: '2', keywords: 'approvals permissions review queue' },
      workflows: { title: 'Workflows', section: 'Automation', description: 'Compose multi-step pipelines and inspect execution history.', hotkey: '3', keywords: 'workflows automation runs builder' },
      automation: { title: 'Automation', section: 'Automation', description: 'Manage cron jobs, event triggers, and timed execution.', hotkey: '4', keywords: 'automation cron jobs triggers schedules timed execution' },
      channels: { title: 'Channels', section: 'Integrations', description: 'Connect messaging surfaces and verify delivery setup.', hotkey: '5', keywords: 'channels telegram slack discord integrations' },
      skills: { title: 'Skills', section: 'Capabilities', description: 'Discover and install reusable skills for specialized workflows.', hotkey: '6', keywords: 'skills install discover capabilities' },
      settings: { title: 'Settings', section: 'System', description: 'Configure providers, models, security, peers, and runtime behavior.', hotkey: '7', keywords: 'settings models providers security peers config' }
    },

    get agents() { return Alpine.store('app').agents; },

    get currentPageMeta() {
      return this.pageMeta[this.page] || {
        title: 'Workspace',
        section: 'RustyHand',
        description: 'Operate agents, tools, and automations from one control deck.',
        hotkey: '',
        keywords: ''
      };
    },

    sidebarChatWithAgent(agent) {
      Alpine.store('app').chatWithAgent(agent);
      if (this.page !== 'agents') this.navigate('agents');
    },

    sidebarNewAgent() {
      Alpine.store('app').pendingAgent = 'new';
      if (this.page !== 'agents') this.navigate('agents');
    },

    sidebarSpawnBuiltin(t) {
      Alpine.store('app').pendingTemplate = t;
      if (this.page !== 'agents') this.navigate('agents');
    },

    // Export all agents as a downloadable JSON file (server-side endpoint).
    async exportAgents() {
      try {
        var data = await RustyHandAPI.get('/api/agents/export');
        var blob = new Blob([JSON.stringify(data, null, 2)], { type: 'application/json' });
        var url = URL.createObjectURL(blob);
        var a = document.createElement('a');
        a.href = url;
        var ts = new Date().toISOString().replace(/[:.]/g, '-').slice(0, 19);
        a.download = 'rustyhand-agents-' + ts + '.json';
        document.body.appendChild(a);
        a.click();
        document.body.removeChild(a);
        URL.revokeObjectURL(url);
        RustyHandToast.success('Exported ' + (data.agent_count || 0) + ' agent(s)');
      } catch (e) {
        RustyHandToast.error('Export failed: ' + e.message);
      }
    },

    // Import agents from a user-selected JSON file. Uses /api/agents/import
    // which returns {imported, skipped, errors}.
    async importAgents(file) {
      if (!file) return;
      try {
        var text = await file.text();
        var body;
        try { body = JSON.parse(text); }
        catch (e) { throw new Error('Invalid JSON: ' + e.message); }
        var res = await RustyHandAPI.post('/api/agents/import', body);
        var msg = 'Imported ' + (res.imported || 0) + ' / ' +
                  (res.total_in_file || 0) + ' agent(s)';
        if (res.skipped) msg += ', skipped ' + res.skipped + ' duplicate(s)';
        if (res.errors && res.errors.length) {
          RustyHandToast.warn(msg + ' (' + res.errors.length + ' error' +
            (res.errors.length === 1 ? '' : 's') + ' — see console)');
          console.warn('Import errors:', res.errors);
        } else {
          RustyHandToast.success(msg);
        }
        await this.refreshAgents();
      } catch (e) {
        RustyHandToast.error('Import failed: ' + e.message);
      }
    },

    async init() {
      var self = this;

      // Check authentication before anything else
      await this.checkAuth();

      // Listen for OS theme changes (only matters when mode is 'system')
      window.matchMedia('(prefers-color-scheme: dark)').addEventListener('change', function(e) {
        if (self.themeMode === 'system') {
          self.theme = e.matches ? 'dark' : 'light';
        }
      });

      // Hash routing
      var validPages = ['agents','approvals','workflows','automation','channels','skills','settings','analytics','knowledge'];
      var pageRedirects = {
        'chat': 'agents',
        'templates': 'agents',
        'overview': 'agents',
        'wizard': 'agents',
        'sessions': 'agents',
        'memory': 'agents',
        'triggers': 'workflows',
        'cron': 'automation',
        'schedules': 'automation',
        'scheduler': 'automation',
        'usage': 'analytics',
        'logs': 'settings',
        'audit': 'settings',
        'security': 'settings',
        'peers': 'settings',
        'migration': 'settings',
        'approval': 'approvals'
      };
      function handleHash() {
        var hash = window.location.hash.replace('#', '') || 'agents';
        if (pageRedirects[hash]) {
          hash = pageRedirects[hash];
          window.location.hash = hash;
        }
        if (validPages.indexOf(hash) >= 0) self.page = hash;
      }
      window.addEventListener('hashchange', handleHash);
      handleHash();

      // Keyboard shortcuts
      var pageOrder = ['agents','approvals','workflows','automation','channels','skills','settings'];
      document.addEventListener('keydown', function(e) {
        var tag = (e.target.tagName || '').toLowerCase();
        var inInput = (tag === 'input' || tag === 'textarea' || tag === 'select' || e.target.isContentEditable);

        // Ctrl+K — open command palette
        if ((e.ctrlKey || e.metaKey) && e.key === 'k') {
          e.preventDefault();
          self.openPalette();
          return;
        }
        // Ctrl+N — create new agent
        if ((e.ctrlKey || e.metaKey) && e.key === 'n' && !e.shiftKey) {
          e.preventDefault();
          Alpine.store('app').pendingAgent = 'new';
          self.navigate('agents');
        }
        // Ctrl+, — settings
        if ((e.ctrlKey || e.metaKey) && e.key === ',') {
          e.preventDefault();
          self.navigate('settings');
        }
        // Escape — close modals, then mobile menu
        if (e.key === 'Escape') {
          if (self.showPalette) { self.closePalette(); return; }
          if (self.showShortcutsModal) { self.showShortcutsModal = false; return; }
          self.mobileMenuOpen = false;
        }
        // ? — show shortcuts (not in input)
        if (e.key === '?' && !inInput && !e.ctrlKey && !e.metaKey) {
          self.showShortcutsModal = !self.showShortcutsModal;
        }
        // 1-9 — page jump (not in input)
        if (!inInput && !e.ctrlKey && !e.metaKey && !e.altKey && e.key >= '1' && e.key <= '9') {
          var idx = parseInt(e.key) - 1;
          if (idx < pageOrder.length) self.navigate(pageOrder[idx]);
        }
      });

      // Connection state listener
      RustyHandAPI.onConnectionChange(function(state) {
        Alpine.store('app').connectionState = state;
      });

      // Initial data load
      this.pollStatus();
      setInterval(function() { self.pollStatus(); }, 5000);
    },

    navigate(p) {
      if (this.page === p) return;
      var currentBody = document.querySelector('.page-body');
      if (currentBody && !window.matchMedia('(prefers-reduced-motion: reduce)').matches) {
        currentBody.classList.add('is-leaving');
        var self = this;
        setTimeout(function() { self.page = p; window.location.hash = p; self.mobileMenuOpen = false; }, 120);
      } else {
        this.page = p;
        window.location.hash = p;
        this.mobileMenuOpen = false;
      }
    },

    async refreshWorkspace(silent) {
      await this.pollStatus();
      if (silent || typeof RustyHandToast === 'undefined') return;
      if (Alpine.store('app').connected) {
          RustyHandToast.success('Workspace refreshed');
      } else {
        RustyHandToast.error(Alpine.store('app').lastError || 'Workspace is offline');
      }
    },

    setTheme(mode) {
      this.themeMode = mode;
      localStorage.setItem('rusty-hand-theme-mode', mode);
      if (mode === 'system') {
        this.theme = window.matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light';
      } else {
        this.theme = mode;
      }
    },

    toggleTheme() {
      var modes = ['light', 'system', 'dark'];
      var next = modes[(modes.indexOf(this.themeMode) + 1) % modes.length];
      this.setTheme(next);
    },

    toggleSidebar() {
      this.sidebarCollapsed = !this.sidebarCollapsed;
      localStorage.setItem('rusty-hand-sidebar', this.sidebarCollapsed ? 'collapsed' : 'expanded');
    },

    // Shortcuts data
    shortcuts: [
      { category: 'Navigation', desc: 'Command palette', keys: ['Ctrl', 'K'] },
      { category: 'Navigation', desc: 'Go to page 1-9', keys: ['1-9'] },
      { category: 'Navigation', desc: 'Settings', keys: ['Ctrl', ','] },
      { category: 'Agents', desc: 'New agent', keys: ['Ctrl', 'N'] },
      { category: 'Chat', desc: 'Focus chat input', keys: ['Ctrl', '/'] },
      { category: 'Chat', desc: 'Send message', keys: ['Enter'] },
      { category: 'Chat', desc: 'New line', keys: ['Shift', 'Enter'] },
      { category: 'General', desc: 'Close / dismiss', keys: ['Esc'] },
      { category: 'General', desc: 'Show shortcuts', keys: ['?'] }
    ],

    get shortcutCategories() {
      var cats = [];
      var seen = {};
      for (var i = 0; i < this.shortcuts.length; i++) {
        var s = this.shortcuts[i];
        if (!seen[s.category]) { seen[s.category] = []; cats.push({ name: s.category, items: seen[s.category] }); }
        seen[s.category].push(s);
      }
      return cats;
    },

    // Command palette
    get paletteItems() {
      var raw = (this.paletteQuery || '').trim();
      // History search mode: prefix ">" triggers /api/search
      if (raw.startsWith('>')) {
        return this.paletteHistoryResults;
      }
      var q = raw.toLowerCase();
      var pages = Object.keys(this.pageMeta).map(function(pageKey) {
        var meta = this.pageMeta[pageKey];
        return {
          type: 'page',
          label: meta.title,
          sublabel: meta.description,
          hint: meta.hotkey ? meta.hotkey : meta.section,
          page: pageKey,
          keywords: meta.keywords
        };
      }, this);
      var actions = [
        { type: 'action', label: 'New Agent', sublabel: 'Spawn and open a fresh agent', hint: 'Ctrl N', action: 'newAgent', keywords: 'create spawn new agent' },
        { type: 'action', label: 'Refresh Workspace', sublabel: 'Re-check kernel, agents, and approvals', hint: 'Live', action: 'refreshWorkspace', keywords: 'refresh reload sync reconnect status' },
        { type: 'action', label: 'Focus Agent Finder', sublabel: 'Jump to the sidebar search box', hint: 'Find', action: 'focusAgentFinder', keywords: 'agent search filter finder sidebar' },
        { type: 'action', label: 'Toggle Theme', sublabel: 'Rotate light, system, and dark modes', hint: this.themeMode, action: 'toggleTheme', keywords: 'theme dark light system appearance' },
        { type: 'action', label: 'Toggle Sidebar', sublabel: 'Collapse or expand the navigation rail', hint: 'Layout', action: 'toggleSidebar', keywords: 'sidebar collapse expand layout' },
        { type: 'action', label: 'Show Shortcuts', sublabel: 'Open the keyboard cheat sheet', hint: '?', action: 'showShortcuts', keywords: 'shortcuts keyboard help' }
      ];
      var agents = (Alpine.store('app').agents || []).map(function(a) {
        return {
          type: 'agent',
          label: a.name,
          sublabel: (a.model_provider || '?') + ':' + (a.model_name || '?'),
          hint: a.state || '',
          agentId: a.id,
          keywords: [a.last_message_preview, a.id, a.model_provider, a.model_name].join(' ')
        };
      });
      var all = pages.concat(actions).concat(agents);
      if (q) {
        all = all.filter(function(it) {
          return it.label.toLowerCase().indexOf(q) >= 0 ||
            (it.sublabel && it.sublabel.toLowerCase().indexOf(q) >= 0) ||
            (it.keywords && it.keywords.toLowerCase().indexOf(q) >= 0);
        });
      }
      return all.slice(0, 18);
    },

    openPalette(initialQuery) {
      this.showPalette = true;
      this.paletteQuery = initialQuery || '';
      this.paletteIdx = 0;
      this.$nextTick(function() {
        var inp = document.querySelector('.palette-input');
        if (inp) inp.focus();
      });
    },

    closePalette() {
      this.showPalette = false;
      this.paletteQuery = '';
      this.paletteHistoryResults = [];
      this._paletteHistoryFor = '';
    },

    async _paletteHistorySearch(q) {
      if (q.length < 2) { this.paletteHistoryResults = []; return; }
      if (this._paletteHistoryFor === q) return;
      this._paletteHistoryFor = q;
      try {
        var data = await RustyHandAPI.get('/api/search?q=' + encodeURIComponent(q) + '&limit=12');
        if (this._paletteHistoryFor !== q) return; // stale
        this.paletteHistoryResults = (data.results || []).map(function(r, i) {
          return {
            type: 'history',
            label: r.excerpt || '(no preview)',
            sublabel: (r.label || r.session_id.substring(0, 8)) + ' · ' + r.role,
            hint: 'Chat',
            agentId: r.agent_id,
            sessionId: r.session_id,
            keywords: r.excerpt,
          };
        });
      } catch (e) { this.paletteHistoryResults = []; }
    },

    executePaletteItem(item) {
      this.closePalette();
      if (item.type === 'page') { this.navigate(item.page); }
      else if (item.type === 'agent') {
        var agentObj = (Alpine.store('app').agents || []).find(function(a) { return a.id === item.agentId; });
        Alpine.store('app').chatWithAgent(agentObj || { id: item.agentId, name: item.label });
        this.navigate('agents');
      }
      else if (item.type === 'history') {
        var agentObj = (Alpine.store('app').agents || []).find(function(a) { return a.id === item.agentId; });
        Alpine.store('app').chatWithAgent(agentObj || { id: item.agentId, name: item.label });
        Alpine.store('app').pendingSession = item.sessionId;
        this.navigate('agents');
      }
      else if (item.type === 'action') {
        if (item.action === 'newAgent') { Alpine.store('app').pendingAgent = 'new'; this.navigate('agents'); }
        else if (item.action === 'refreshWorkspace') { this.refreshWorkspace(); }
        else if (item.action === 'focusAgentFinder') {
          this.mobileMenuOpen = true;
          var self = this;
          this.$nextTick(function() {
            if (self.$refs && self.$refs.sidebarAgentSearch) self.$refs.sidebarAgentSearch.focus();
          });
        }
        else if (item.action === 'toggleTheme') { this.toggleTheme(); }
        else if (item.action === 'toggleSidebar') { this.toggleSidebar(); }
        else if (item.action === 'showShortcuts') { this.showShortcutsModal = true; }
      }
    },

    paletteKeydown(e) {
      var items = this.paletteItems;
      if (e.key === 'ArrowDown') { e.preventDefault(); this.paletteIdx = Math.min(items.length - 1, this.paletteIdx + 1); }
      else if (e.key === 'ArrowUp') { e.preventDefault(); this.paletteIdx = Math.max(0, this.paletteIdx - 1); }
      else if (e.key === 'Enter' && items.length > 0) { e.preventDefault(); this.executePaletteItem(items[this.paletteIdx]); }
      else if (e.key === 'Escape') { this.closePalette(); }
    },

    async pollStatus() {
      var store = Alpine.store('app');
      await Promise.allSettled([
        store.checkStatus(),
        store.refreshAgents(),
        store.refreshApprovals()
      ]);
      this.connected = store.connected;
      this.version = store.version;
      this.agentCount = store.agentCount;
      this.wsConnected = RustyHandAPI.isWsConnected();
    }
  };
}
