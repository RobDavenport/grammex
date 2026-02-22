// grammex demo — ES6 module
// Imports will resolve once wasm-pack builds the pkg/ directory.

let wasm = null;

const NODE_COLORS = {
    start:    '#27ae60',
    room:     '#3498db',
    corridor: '#2c3e50',
    lock:     '#e74c3c',
    key:      '#f1c40f',
    boss:     '#9b59b6',
    treasure: '#e67e22',
    exit:     '#ecf0f1',
    objective:'#27ae60',
    task:     '#3498db',
    reward:   '#f1c40f',
    prereq:   '#e74c3c',
};

const NODE_RADIUS = 18;

// ---------- State ----------

let graphData = { nodes: [], edges: [] };
let stepRewriter = null;
let autoInterval = null;
let stats = { nodes: 0, edges: 0, steps: 0, rules: 0, cycles: 0 };

// ---------- DOM ----------

const canvas  = document.getElementById('canvas');
const ctx     = canvas.getContext('2d');
const msgEl   = document.getElementById('message');

const btnGenerate   = document.getElementById('btn-generate');
const btnStep       = document.getElementById('btn-step');
const btnAuto       = document.getElementById('btn-auto');
const btnReset      = document.getElementById('btn-reset');
const btnRandomSeed = document.getElementById('btn-random-seed');
const speedSlider   = document.getElementById('speed');
const speedLabel    = document.getElementById('speed-label');

const statNodes = document.getElementById('stat-nodes');
const statEdges = document.getElementById('stat-edges');
const statSteps = document.getElementById('stat-steps');
const statRules = document.getElementById('stat-rules');
const statCycles = document.getElementById('stat-cycles');

// ---------- Init ----------

async function init() {
    try {
        const mod = await import('../pkg/grammex_demo.js');
        await mod.default();
        wasm = mod;
        showMessage('WASM loaded. Ready to generate.', 'success');
    } catch (e) {
        console.warn('WASM not available (run wasm-pack build first):', e);
        showMessage('WASM not built yet. Using placeholder data.', 'info');
    }
    render();
}

// ---------- Graph Rendering ----------

function render() {
    const dpr = window.devicePixelRatio || 1;
    const w = canvas.clientWidth;
    const h = canvas.clientHeight;
    canvas.width  = w * dpr;
    canvas.height = h * dpr;
    ctx.setTransform(dpr, 0, 0, dpr, 0, 0);

    ctx.fillStyle = '#1a1a2e';
    ctx.fillRect(0, 0, w, h);

    // Draw edges
    ctx.strokeStyle = '#3a3a5a';
    ctx.lineWidth = 2;
    for (const edge of graphData.edges) {
        const src = graphData.nodes[edge.source];
        const tgt = graphData.nodes[edge.target];
        if (!src || !tgt) continue;
        drawArrow(ctx, src.x, src.y, tgt.x, tgt.y);
    }

    // Draw nodes
    for (const node of graphData.nodes) {
        if (!node) continue;
        const color = NODE_COLORS[node.kind] || '#888';
        ctx.beginPath();
        ctx.arc(node.x, node.y, NODE_RADIUS, 0, Math.PI * 2);
        ctx.fillStyle = color;
        ctx.fill();
        ctx.strokeStyle = '#fff';
        ctx.lineWidth = 2;
        ctx.stroke();

        // Label
        ctx.fillStyle = node.kind === 'key' ? '#333' : '#fff';
        ctx.font = '11px sans-serif';
        ctx.textAlign = 'center';
        ctx.textBaseline = 'middle';
        ctx.fillText(node.label || node.kind, node.x, node.y);
    }

    // Empty state message
    if (graphData.nodes.length === 0) {
        ctx.fillStyle = '#6a6a80';
        ctx.font = '16px sans-serif';
        ctx.textAlign = 'center';
        ctx.textBaseline = 'middle';
        ctx.fillText('Press "Generate" to create a graph', w / 2, h / 2);
    }

    updateStats();
}

function drawArrow(ctx, x1, y1, x2, y2) {
    const dx = x2 - x1;
    const dy = y2 - y1;
    const len = Math.sqrt(dx * dx + dy * dy);
    if (len < 1) return;
    const ux = dx / len;
    const uy = dy / len;

    // Shorten to avoid overlapping node circles
    const sx = x1 + ux * NODE_RADIUS;
    const sy = y1 + uy * NODE_RADIUS;
    const ex = x2 - ux * NODE_RADIUS;
    const ey = y2 - uy * NODE_RADIUS;

    ctx.beginPath();
    ctx.moveTo(sx, sy);
    ctx.lineTo(ex, ey);
    ctx.stroke();

    // Arrowhead
    const headLen = 10;
    const angle = Math.atan2(ey - sy, ex - sx);
    ctx.beginPath();
    ctx.moveTo(ex, ey);
    ctx.lineTo(
        ex - headLen * Math.cos(angle - Math.PI / 6),
        ey - headLen * Math.sin(angle - Math.PI / 6)
    );
    ctx.lineTo(
        ex - headLen * Math.cos(angle + Math.PI / 6),
        ey - headLen * Math.sin(angle + Math.PI / 6)
    );
    ctx.closePath();
    ctx.fillStyle = '#3a3a5a';
    ctx.fill();
}

// ---------- Placeholder Data Generation ----------

function generatePlaceholder() {
    const seed = parseInt(document.getElementById('seed').value) || 0;
    const mode = document.getElementById('mode').value;
    const maxSteps = parseInt(document.getElementById('max-steps').value) || 100;

    const rng = mulberry32(seed);
    const nodeCount = 5 + Math.floor(rng() * Math.min(maxSteps / 10, 15));
    const kinds = mode === 'quest'
        ? ['objective', 'task', 'reward', 'prereq']
        : ['start', 'room', 'corridor', 'key', 'lock', 'boss', 'treasure'];

    const w = canvas.clientWidth;
    const h = canvas.clientHeight;
    const cx = w / 2;
    const cy = h / 2;
    const radius = Math.min(w, h) * 0.35;

    const nodes = [];
    for (let i = 0; i < nodeCount; i++) {
        const kind = i === 0 ? kinds[0] : kinds[1 + Math.floor(rng() * (kinds.length - 1))];
        const angle = (2 * Math.PI * i) / nodeCount - Math.PI / 2;
        nodes.push({
            kind,
            label: kind,
            x: cx + radius * Math.cos(angle),
            y: cy + radius * Math.sin(angle),
        });
    }

    const edges = [];
    for (let i = 1; i < nodeCount; i++) {
        edges.push({ source: i - 1, target: i });
    }
    const extraEdges = Math.floor(rng() * nodeCount * 0.5);
    for (let i = 0; i < extraEdges; i++) {
        const s = Math.floor(rng() * nodeCount);
        let t = Math.floor(rng() * nodeCount);
        if (t === s) t = (t + 1) % nodeCount;
        edges.push({ source: s, target: t });
    }

    graphData = { nodes, edges };
    stats = {
        nodes: nodeCount,
        edges: edges.length,
        steps: Math.floor(rng() * maxSteps),
        rules: Math.floor(rng() * maxSteps * 0.7),
        cycles: Math.max(0, edges.length - nodeCount + 1),
    };
}

function mulberry32(a) {
    return function() {
        let t = a += 0x6D2B79F5;
        t = Math.imul(t ^ t >>> 15, t | 1);
        t ^= t + Math.imul(t ^ t >>> 7, t | 61);
        return ((t ^ t >>> 14) >>> 0) / 4294967296;
    };
}

// ---------- Config ----------

function getConfig() {
    return {
        seed: parseInt(document.getElementById('seed').value) || 0,
        mode: document.getElementById('mode').value,
        max_steps: parseInt(document.getElementById('max-steps').value) || 100,
        strategy: document.getElementById('strategy').value,
        lock_key: document.getElementById('lock-key').checked,
        reachability: document.getElementById('reachability').checked,
        acyclic: document.getElementById('acyclic').checked,
    };
}

// ---------- Actions ----------

function doGenerate() {
    stopAuto();
    stepRewriter = null;

    if (wasm) {
        try {
            const config = JSON.stringify(getConfig());
            const result = wasm.generate_demo(config);
            const parsed = JSON.parse(result);
            graphData = parsed;
            stats = {
                nodes: parsed.nodes.length,
                edges: parsed.edges.length,
                steps: 0,
                rules: 0,
                cycles: Math.max(0, parsed.edges.length - parsed.nodes.length + 1),
            };
            showMessage(`Generated: ${parsed.nodes.length} nodes, ${parsed.edges.length} edges`, 'success');
        } catch (e) {
            console.error('Generation error:', e);
            generatePlaceholder();
            showMessage('WASM error. Using placeholder.', 'error');
        }
    } else {
        generatePlaceholder();
        showMessage('Using placeholder data (WASM not loaded).', 'info');
    }
    render();
}

function doStep() {
    if (!stepRewriter) {
        if (wasm) {
            try {
                const config = JSON.stringify(getConfig());
                stepRewriter = new wasm.StepRewriter(config);
                stats = { nodes: 1, edges: 0, steps: 0, rules: 0, cycles: 0 };
            } catch (e) {
                showMessage('Error creating step rewriter: ' + e.message, 'error');
                return;
            }
        } else {
            showMessage('WASM not loaded. Use Generate for placeholder.', 'info');
            return;
        }
    }

    try {
        const eventStr = stepRewriter.step();
        const event = JSON.parse(eventStr);

        // Update graph display
        const graphStr = stepRewriter.graph_json();
        graphData = JSON.parse(graphStr);

        // Update stats from event
        if (event.type === 'applied') {
            stats.steps++;
            stats.rules++;
            stats.nodes = graphData.nodes.length;
            stats.edges = graphData.edges.length;
            const added = event.nodes_added ? ` (+${event.nodes_added.length} nodes, +${event.edges_added.length} edges)` : '';
            showMessage(`Applied rule: ${event.rule}${added}`, 'success');
        } else if (event.type === 'constraint_violated') {
            stats.steps++;
            showMessage(`Constraint violated: ${event.rule} (rolled back)`, 'info');
        } else if (event.type === 'no_match' || event.type === 'complete') {
            stopAuto();
            stats.nodes = event.nodes || graphData.nodes.length;
            stats.edges = event.edges || graphData.edges.length;
            showMessage(`Complete: ${stats.nodes} nodes, ${stats.edges} edges, ${event.steps || stats.steps} steps`, 'success');
        }

        stats.cycles = Math.max(0, stats.edges - stats.nodes + 1);
    } catch (e) {
        console.error('Step error:', e);
        showMessage('Step error: ' + e.message, 'error');
        stopAuto();
    }

    render();
}

function toggleAuto() {
    if (autoInterval) {
        stopAuto();
    } else {
        const speed = parseInt(speedSlider.value) || 10;
        const interval = Math.max(16, Math.floor(1000 / speed));
        autoInterval = setInterval(doStep, interval);
        btnAuto.textContent = 'Stop';
        btnAuto.classList.add('active');
    }
}

function stopAuto() {
    if (autoInterval) {
        clearInterval(autoInterval);
        autoInterval = null;
    }
    btnAuto.textContent = 'Auto-play';
    btnAuto.classList.remove('active');
}

function doReset() {
    stopAuto();
    graphData = { nodes: [], edges: [] };
    stepRewriter = null;
    stats = { nodes: 0, edges: 0, steps: 0, rules: 0, cycles: 0 };
    render();
    showMessage('', 'info');
    msgEl.classList.add('hidden');
}

// ---------- UI Helpers ----------

function showMessage(text, type) {
    msgEl.textContent = text;
    msgEl.className = `message ${type}`;
    if (text) {
        msgEl.classList.remove('hidden');
    } else {
        msgEl.classList.add('hidden');
    }
}

function updateStats() {
    statNodes.textContent = stats.nodes;
    statEdges.textContent = stats.edges;
    statSteps.textContent = stats.steps;
    statRules.textContent = stats.rules;
    statCycles.textContent = Math.max(0, stats.cycles);
}

// ---------- Event Listeners ----------

btnGenerate.addEventListener('click', doGenerate);
btnStep.addEventListener('click', doStep);
btnAuto.addEventListener('click', toggleAuto);
btnReset.addEventListener('click', doReset);
btnRandomSeed.addEventListener('click', () => {
    document.getElementById('seed').value = Math.floor(Math.random() * 4294967295);
});
speedSlider.addEventListener('input', () => {
    const v = speedSlider.value;
    speedLabel.textContent = `${v} steps/s`;
    if (autoInterval) {
        stopAuto();
        toggleAuto();
    }
});

window.addEventListener('resize', render);

// ---------- Start ----------

init();
