// grammex demo — ES6 module
// Imports will resolve once wasm-pack builds the pkg/ directory.

let wasm = null;

const NODE_COLORS = {
    entrance: '#27ae60',
    room:     '#3498db',
    lock:     '#e74c3c',
    key:      '#f1c40f',
    boss:     '#9b59b6',
    treasure: '#e67e22',
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
        const color = NODE_COLORS[node.kind] || '#888';
        ctx.beginPath();
        ctx.arc(node.x, node.y, NODE_RADIUS, 0, Math.PI * 2);
        ctx.fillStyle = color;
        ctx.fill();
        ctx.strokeStyle = '#fff';
        ctx.lineWidth = 2;
        ctx.stroke();

        // Label
        ctx.fillStyle = '#fff';
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

// ---------- Force-Directed Layout (placeholder) ----------

function layoutGraph() {
    const w = canvas.clientWidth;
    const h = canvas.clientHeight;
    const n = graphData.nodes.length;
    if (n === 0) return;

    // Simple circular layout as placeholder
    const cx = w / 2;
    const cy = h / 2;
    const radius = Math.min(w, h) * 0.35;

    for (let i = 0; i < n; i++) {
        const angle = (2 * Math.PI * i) / n - Math.PI / 2;
        graphData.nodes[i].x = cx + radius * Math.cos(angle);
        graphData.nodes[i].y = cy + radius * Math.sin(angle);
    }
}

// ---------- Placeholder Data Generation ----------

function generatePlaceholder() {
    const seed = parseInt(document.getElementById('seed').value) || 0;
    const mode = document.getElementById('mode').value;
    const maxSteps = parseInt(document.getElementById('max-steps').value) || 100;

    // Simple procedural placeholder based on seed
    const rng = mulberry32(seed);
    const nodeCount = 5 + Math.floor(rng() * Math.min(maxSteps / 10, 15));
    const kinds = mode === 'quest'
        ? ['start', 'task', 'reward', 'boss']
        : ['entrance', 'room', 'key', 'lock', 'boss', 'treasure'];

    const nodes = [];
    nodes.push({ kind: kinds[0], label: kinds[0] });
    for (let i = 1; i < nodeCount; i++) {
        const kind = kinds[1 + Math.floor(rng() * (kinds.length - 1))];
        nodes.push({ kind, label: kind });
    }

    const edges = [];
    // Ensure connectivity: chain
    for (let i = 1; i < nodeCount; i++) {
        edges.push({ source: i - 1, target: i });
    }
    // Add some random edges
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
        cycles: edges.length - nodeCount + 1,
    };

    layoutGraph();
}

// Simple seeded PRNG
function mulberry32(a) {
    return function() {
        let t = a += 0x6D2B79F5;
        t = Math.imul(t ^ t >>> 15, t | 1);
        t ^= t + Math.imul(t ^ t >>> 7, t | 61);
        return ((t ^ t >>> 14) >>> 0) / 4294967296;
    };
}

// ---------- Actions ----------

function doGenerate() {
    stopAuto();

    if (wasm) {
        const config = JSON.stringify({
            seed: parseInt(document.getElementById('seed').value) || 0,
            mode: document.getElementById('mode').value,
            max_steps: parseInt(document.getElementById('max-steps').value) || 100,
            strategy: document.getElementById('strategy').value,
        });
        const result = wasm.generate_demo(config);
        const parsed = JSON.parse(result);
        if (parsed.status === 'not_implemented') {
            generatePlaceholder();
            showMessage('Using placeholder (WASM not yet implemented).', 'info');
        } else {
            graphData = parsed;
            layoutGraph();
        }
    } else {
        generatePlaceholder();
        showMessage('Using placeholder data.', 'info');
    }
    render();
}

function doStep() {
    if (!stepRewriter && wasm) {
        const config = JSON.stringify({
            seed: parseInt(document.getElementById('seed').value) || 0,
            mode: document.getElementById('mode').value,
        });
        try {
            stepRewriter = new wasm.StepRewriter(config);
        } catch (e) {
            showMessage('Step rewriter not available.', 'error');
            return;
        }
    }

    if (stepRewriter) {
        const event = JSON.parse(stepRewriter.step());
        if (event.type === 'not_implemented') {
            showMessage('Step not yet implemented in WASM.', 'info');
        }
        const json = JSON.parse(stepRewriter.graph_json());
        graphData = json;
        layoutGraph();
        stats.steps++;
    } else {
        showMessage('WASM not loaded. Use Generate for placeholder.', 'info');
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
