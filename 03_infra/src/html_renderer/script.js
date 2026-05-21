// Crystalline DSM — vanilla ES module embutido. Sem dependências.

(function () {
  const root = document.querySelector(".dsm-root");
  const container = document.querySelector(".matrix-container");
  const canvas = document.getElementById("dsm-matrix");
  const tooltip = document.getElementById("tooltip");
  const rowLabels = document.querySelector(".row-labels");
  const colLabels = document.querySelector(".column-labels");
  const ctx = canvas.getContext("2d");

  const N = GRAPH_DATA.labels.length;

  // --- Cell size adaptativo ---
  function pickCellSize(n) {
    if (n < 50) return 24;
    if (n < 200) return 12;
    if (n < 700) return 6;
    return 3;
  }
  const cellSize = pickCellSize(N);
  container.style.setProperty("--cell-size", cellSize + "px");

  const totalPx = N * cellSize;
  canvas.width = totalPx;
  canvas.height = totalPx;
  canvas.style.width = totalPx + "px";
  canvas.style.height = totalPx + "px";
  canvas.style.marginLeft = "160px"; // alinha com row-labels width

  // --- Renderizar labels (DOM) ---
  function renderLabels() {
    const fragRow = document.createDocumentFragment();
    const fragCol = document.createDocumentFragment();
    for (let i = 0; i < N; i++) {
      const label = GRAPH_DATA.labels[i];
      const kind = GRAPH_DATA.kinds[i];
      const sccIdx = GRAPH_DATA.scc_per_position[i];
      const scc = GRAPH_DATA.sccs[sccIdx];
      const cyclic = scc.cyclic ? "true" : "false";

      const rowSpan = document.createElement("span");
      rowSpan.textContent = label;
      rowSpan.dataset.idx = String(i);
      rowSpan.dataset.kind = kind;
      rowSpan.dataset.cyclic = cyclic;
      rowSpan.dataset.match = "true";
      rowSpan.title = label;
      fragRow.appendChild(rowSpan);

      const colSpan = document.createElement("span");
      colSpan.textContent = label;
      colSpan.dataset.idx = String(i);
      colSpan.dataset.kind = kind;
      colSpan.dataset.cyclic = cyclic;
      colSpan.dataset.match = "true";
      colSpan.title = label;
      colSpan.style.left = (160 + i * cellSize + cellSize / 2) + "px";
      colSpan.style.top = "120px";
      fragCol.appendChild(colSpan);
    }
    rowLabels.appendChild(fragRow);
    colLabels.appendChild(fragCol);
  }
  renderLabels();

  // --- Índices de adjacência ---
  const pairKey = (from, to) => from + "-" + to;
  const pairToEdge = new Map();
  for (const e of GRAPH_DATA.edges) {
    pairToEdge.set(pairKey(e.from, e.to), e);
  }

  // --- Cache de cores ---
  // getComputedStyle().getPropertyValue() em custom properties retorna a
  // STRING LITERAL declarada (ex.: "light-dark(#fff, #1e293b)"), não o
  // valor resolvido. Para que o Canvas 2D receba uma cor utilizável
  // (rgb/hex), usamos um elemento sonda invisível com `color: var(--x)`
  // — aí o browser resolve light-dark() de acordo com o tema actual.
  const probe = document.createElement("span");
  probe.style.cssText = "position:absolute;visibility:hidden;pointer-events:none;";
  root.appendChild(probe);

  function resolveVar(name) {
    probe.style.color = "var(" + name + ")";
    return getComputedStyle(probe).color;
  }

  let COLORS = {};
  function refreshColors() {
    COLORS = {
      bgMatrix: resolveVar("--bg-matrix"),
      bgExtern: resolveVar("--bg-extern"),
      cellEdge1: resolveVar("--cell-edge-1"),
      cellEdge2: resolveVar("--cell-edge-2"),
      cellEdge4: resolveVar("--cell-edge-4"),
      diagonal: resolveVar("--diagonal"),
      borderScc: resolveVar("--border-scc"),
      divider: resolveVar("--divider"),
    };
  }
  refreshColors();

  function edgeColor(count) {
    if (count >= 4) return COLORS.cellEdge4;
    if (count >= 2) return COLORS.cellEdge2;
    return COLORS.cellEdge1;
  }

  // --- Desenho ---
  function shouldDrawIndex(i) {
    if (root.classList.contains("hide-externals") && GRAPH_DATA.kinds[i] !== "internal") {
      return false;
    }
    if (root.classList.contains("only-cyclic")) {
      const sccIdx = GRAPH_DATA.scc_per_position[i];
      if (!GRAPH_DATA.sccs[sccIdx].cyclic) return false;
    }
    return true;
  }

  function draw() {
    ctx.fillStyle = COLORS.bgMatrix;
    ctx.fillRect(0, 0, totalPx, totalPx);

    // Região externa: fundo levemente diferente
    const ib = GRAPH_DATA.internal_boundary;
    if (ib < N) {
      ctx.fillStyle = COLORS.bgExtern;
      const externStart = ib * cellSize;
      const externLen = (N - ib) * cellSize;
      ctx.fillRect(externStart, 0, externLen, totalPx);
      ctx.fillRect(0, externStart, totalPx, externLen);
    }

    // Arestas
    for (const e of GRAPH_DATA.edges) {
      if (!shouldDrawIndex(e.from) || !shouldDrawIndex(e.to)) continue;
      ctx.fillStyle = edgeColor(e.count);
      ctx.fillRect(e.to * cellSize, e.from * cellSize, cellSize, cellSize);
    }

    // Diagonal
    ctx.fillStyle = COLORS.diagonal;
    for (let i = 0; i < N; i++) {
      if (!shouldDrawIndex(i)) continue;
      ctx.fillRect(i * cellSize, i * cellSize, cellSize, cellSize);
    }

    // Bordas de SCCs cíclicos
    ctx.strokeStyle = COLORS.borderScc;
    ctx.lineWidth = Math.max(1, Math.floor(cellSize / 3));
    for (const s of GRAPH_DATA.sccs) {
      if (!s.cyclic) continue;
      const x = s.start * cellSize;
      const len = (s.end - s.start) * cellSize;
      ctx.strokeRect(x + 0.5, x + 0.5, len, len);
    }

    // Linha divisória interno/externo
    if (ib > 0 && ib < N) {
      ctx.strokeStyle = COLORS.divider;
      ctx.lineWidth = 1;
      const pos = ib * cellSize + 0.5;
      ctx.beginPath();
      ctx.moveTo(pos, 0);
      ctx.lineTo(pos, totalPx);
      ctx.moveTo(0, pos);
      ctx.lineTo(totalPx, pos);
      ctx.stroke();
    }
  }
  draw();

  // --- Listeners ---
  let lastHoveredRow = -1;
  let lastHoveredCol = -1;
  const rowSpans = rowLabels.querySelectorAll("span");
  const colSpans = colLabels.querySelectorAll("span");

  function setHover(row, col) {
    if (lastHoveredRow >= 0) rowSpans[lastHoveredRow]?.classList.remove("is-hovered");
    if (lastHoveredCol >= 0) colSpans[lastHoveredCol]?.classList.remove("is-hovered");
    if (row >= 0 && row < N) rowSpans[row]?.classList.add("is-hovered");
    if (col >= 0 && col < N) colSpans[col]?.classList.add("is-hovered");
    lastHoveredRow = row;
    lastHoveredCol = col;
  }

  canvas.addEventListener("mousemove", (event) => {
    const rect = canvas.getBoundingClientRect();
    const x = event.clientX - rect.left;
    const y = event.clientY - rect.top;
    const col = Math.floor(x / cellSize);
    const row = Math.floor(y / cellSize);
    if (row < 0 || row >= N || col < 0 || col >= N) {
      tooltip.hidePopover();
      setHover(-1, -1);
      return;
    }

    setHover(row, col);

    const edge = pairToEdge.get(pairKey(row, col));
    let lines = [];
    lines.push("Row: " + GRAPH_DATA.labels[row]);
    lines.push("Col: " + GRAPH_DATA.labels[col]);
    if (edge) {
      lines.push("Edges: " + edge.count);
      if (edge.items && edge.items.length > 0) {
        const more = edge.truncated ? " (+more)" : "";
        lines.push("Items: " + edge.items.join(", ") + more);
      }
      if (edge.has_glob) lines.push("(includes glob)");
    } else if (row === col) {
      lines.push("(diagonal)");
    } else {
      lines.push("(no edge)");
    }
    tooltip.textContent = lines.join("\n");
    tooltip.style.whiteSpace = "pre-line";

    // Fallback de posicionamento: nem todos os browsers suportam
    // CSS Anchor Positioning de forma completa.
    tooltip.style.position = "fixed";
    tooltip.style.left = (event.clientX + 12) + "px";
    tooltip.style.top = (event.clientY + 12) + "px";

    try {
      tooltip.showPopover();
    } catch (_e) {
      tooltip.style.display = "block";
    }
  });

  canvas.addEventListener("mouseleave", () => {
    try { tooltip.hidePopover(); } catch (_e) { tooltip.style.display = "none"; }
    setHover(-1, -1);
  });

  canvas.addEventListener("click", (event) => {
    const rect = canvas.getBoundingClientRect();
    const x = event.clientX - rect.left;
    const y = event.clientY - rect.top;
    const col = Math.floor(x / cellSize);
    const row = Math.floor(y / cellSize);
    if (row < 0 || row >= N || col < 0 || col >= N) return;
    rowSpans[row]?.classList.toggle("is-pinned");
    colSpans[col]?.classList.toggle("is-pinned");
  });

  // Botões de filtro: alternar classe no body e redesenhar
  document.getElementById("toggle-external")?.addEventListener("click", () => {
    root.classList.toggle("hide-externals");
    draw();
  });
  document.getElementById("toggle-trivial")?.addEventListener("click", () => {
    root.classList.toggle("only-cyclic");
    draw();
  });

  // Busca: marca data-match nos labels
  document.getElementById("search")?.addEventListener("input", (event) => {
    const q = event.target.value.toLowerCase().trim();
    for (let i = 0; i < N; i++) {
      const matches = q === "" || GRAPH_DATA.labels[i].toLowerCase().includes(q);
      const v = matches ? "true" : "false";
      rowSpans[i].dataset.match = v;
      colSpans[i].dataset.match = v;
    }
  });

  // Mudanças de tema OS: recalcula cores e redesenha
  if (window.matchMedia) {
    const mql = window.matchMedia("(prefers-color-scheme: dark)");
    const onChange = () => { refreshColors(); draw(); };
    if (mql.addEventListener) mql.addEventListener("change", onChange);
    else if (mql.addListener) mql.addListener(onChange);
  }
})();
