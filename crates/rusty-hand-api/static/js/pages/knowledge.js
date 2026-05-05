/**
 * Knowledge Graph page — visualize entities and relations.
 *
 * Data source: GET /api/knowledge
 */

function knowledgePage() {
    return {
        nodes: [],
        edges: [],
        loading: true,
        error: null,
        selectedNode: null,

        async init() {
            await this.refresh();
        },

        async refresh() {
            this.loading = true;
            this.error = null;
            try {
                const data = await RustyHandAPI.get('/api/knowledge');
                this.nodes = data.nodes || [];
                this.edges = data.edges || [];
            } catch (e) {
                this.error = e.message || 'Failed to load knowledge graph';
            }
            this.loading = false;
        },

        // Group nodes by type
        nodeTypes() {
            const types = {};
            for (const n of this.nodes) {
                const t = n.type || 'unknown';
                if (!types[t]) types[t] = [];
                types[t].push(n);
            }
            return types;
        },

        // Get edges for a specific node
        nodeEdges(nodeId) {
            return this.edges.filter(e => e.source === nodeId || e.target === nodeId);
        },

        // Get connected node names for display
        connectedNames(nodeId) {
            const connected = [];
            for (const e of this.edges) {
                if (e.source === nodeId) {
                    const target = this.nodes.find(n => n.id === e.target);
                    if (target) connected.push({ relation: e.type, name: target.name, direction: 'out' });
                }
                if (e.target === nodeId) {
                    const source = this.nodes.find(n => n.id === e.source);
                    if (source) connected.push({ relation: e.type, name: source.name, direction: 'in' });
                }
            }
            return connected;
        },

        selectNode(node) {
            this.selectedNode = this.selectedNode === node ? null : node;
        },

        // Color for entity type
        typeColor(type) {
            const colors = {
                'person': '#3B82F6',
                'organization': '#8B5CF6',
                'location': '#10B981',
                'concept': '#F59E0B',
                'event': '#EF4444',
                'tool': '#EC4899',
                'project': '#06B6D4',
            };
            return colors[type?.toLowerCase()] || '#6B7280';
        },
    };
}
