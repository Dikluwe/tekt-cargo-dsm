// Crystalline Lineage @prompt lab/dsm-explicavel/CONTRATO-DOXYGEN.md
'use strict';
const $ = id => document.getElementById(id);
// Conteúdo externo nunca passa por innerHTML, templates HTML ou atributos-evento.
function el(tag, text, cls) {
  const node = document.createElement(tag);
  if (text !== undefined) node.textContent = text;
  if (cls) node.className = cls;
  return node;
}
let data, evidenceByIndex, cells, modules, selectedEvidence = [], shown = 0, excluded = [], excludedShown = 0;
let requestVersion = 0;
const describe = p => {
  switch (p.code) {
    case 'sem_modulo': return 'sem módulo contenedor';
    case 'no_ausente': return `nó ausente #${p.id}`;
    case 'multiplos_pais': return `múltiplos pais do item #${p.item}: ${p.pais.join(', ')}`;
    case 'ciclo_owns': return `ciclo Owns: ${p.membros.join(', ')}`;
    default: return 'diagnóstico não reconhecido';
  }
};
function reason(r) {
  const d = r.destination;
  if (d.kind === 'internal') return `Relação interna absorvida no módulo #${d.module}.`;
  if (d.reason === 'owns_context') return 'Owns: contenção estrutural, não compõe o peso.';
  return `Origem: ${(d.source_problems || []).map(describe).join('; ') || 'válida'}. Destino: ${(d.target_problems || []).map(describe).join('; ') || 'válido'}.`;
}
function evidenceCard(r, canQuery) {
  const card = el('article', undefined, 'evidence');
  card.append(el('code', `Registro #${r.snapshot_index} · ${r.relation} · ${r.subtype || 'subtipo não informado'}`));
  card.append(el('p', `${r.from} → ${r.to}`));
  card.append(el('p', `IDs no snapshot: ${r.from_id} → ${r.to_id}`, 'hint'));
  card.append(el('p', 'Local de uso: não disponível. A declaração do nó não localiza esta referência.', 'hint'));
  if (data.documentation) {
    for (const id of [r.from_id, r.to_id]) {
      const button = el('button', `Documentação #${id}`, 'secondary');
      button.addEventListener('click', () => { selectDocument(String(id)); $('documentation-panel').scrollIntoView({block:'start'}); });
      card.append(button);
    }
  }
  if (canQuery) {
    const button = el('button', 'Investigar cadeia destes itens', 'secondary');
    button.addEventListener('click', () => {
      $('consumer').value = String(r.from_id); $('changed').value = String(r.to_id);
      $('query-button').focus(); $('query').requestSubmit();
    });
    card.append(button);
  } else card.append(el('p', reason(r)));
  return card;
}
function showMore() {
  for (const index of selectedEvidence.slice(shown, shown + 20)) $('evidence-list').append(evidenceCard(evidenceByIndex.get(index), true));
  shown += 20; $('more').hidden = shown >= selectedEvidence.length;
}
function selectCell(from, to, button) {
  document.querySelectorAll('.cell').forEach(b => b.setAttribute('aria-pressed', String(b === button)));
  const cell = cells.get(`${from.id}:${to.id}`);
  $('selection').replaceChildren(el('p', `${from.path} → ${to.path}`, 'selected-path'));
  if (cell) {
    $('selection').append(el('p', `${cell.weight} relações`, 'weight'), el('p', 'Peso = registros Uses, incluindo paralelas. Quantidade de locais de uso: desconhecida.'));
  } else $('selection').append(el('p', from.id === to.id ? 'Diagonal: relações internas são absorvidas. Consulte as exclusões abaixo.' : 'Nenhuma relação agregada nesta direção no snapshot. Isso não comprova ausência no sistema.'));
  if (data.documentation) selectDocument(String(to.id));
  selectedEvidence = cell?.evidence_indices || []; shown = 0;
  $('evidence-list').replaceChildren(); showMore();
}
function matrix() {
  const container = $('matrix-scroll');
  if (!modules.length) {
    container.append(el('p', 'Nenhum módulo disponível neste snapshot.', 'empty'));
    $('selection').textContent = 'Sem células para selecionar neste snapshot.';
    return;
  }
  const table = el('table'); const caption = el('caption', 'DSM: origem nas linhas; destino nas colunas.');
  caption.className = 'hint'; table.append(caption);
  const head = el('thead'), tr = el('tr'); tr.append(el('th', 'Consome ↓ / fornece →'));
  modules.forEach(m => { const th = el('th', m.path); th.scope = 'col'; tr.append(th); }); head.append(tr); table.append(head);
  const body = el('tbody'); const buttons = [];
  modules.forEach((from, row) => {
    const tr = el('tr'), th = el('th', from.path); th.scope = 'row'; tr.append(th);
    modules.forEach((to, col) => {
      const td = el('td'), c = cells.get(`${from.id}:${to.id}`), b = el('button', c ? String(c.weight) : '—', 'cell');
      b.type = 'button'; b.dataset.weight = c ? (c.weight > 1 ? 'many' : '1') : '0';
      b.setAttribute('aria-label', `${from.path} depende de ${to.path}: ${c ? c.weight + ' relações' : 'sem relação agregada'}`);
      b.setAttribute('aria-pressed', 'false'); b.tabIndex = row === 0 && col === 0 ? 0 : -1;
      b.addEventListener('focus', () => { buttons.forEach(x => { x.tabIndex = x === b ? 0 : -1; }); });
      b.addEventListener('click', () => selectCell(from, to, b));
      b.addEventListener('keydown', e => {
        let r = row, c = col;
        switch (e.key) {
          case 'ArrowUp': r--; break; case 'ArrowDown': r++; break;
          case 'ArrowLeft': c--; break; case 'ArrowRight': c++; break;
          case 'Home': c = 0; break; case 'End': c = modules.length - 1; break;
          default: return;
        }
        e.preventDefault();
        buttons[Math.max(0, Math.min(modules.length - 1, r)) * modules.length + Math.max(0, Math.min(modules.length - 1, c))].focus();
      });
      buttons.push(b); td.append(b); tr.append(td);
    }); body.append(tr);
  }); table.append(body); container.append(table);
}
function showExcluded() {
  for (const entry of excluded.slice(excludedShown, excludedShown + 20)) {
    if (entry.type === 'item') {
      const card = el('article', undefined, 'evidence');
      card.append(el('p', `Item #${entry.value.id}: ${entry.value.path}`), el('p', entry.value.problems.map(describe).join('; ')));
      $('excluded-list').append(card);
    } else $('excluded-list').append(evidenceCard(entry.value, false));
  }
  excludedShown += 20; $('excluded-more').hidden = excludedShown >= excluded.length;
  if (!excluded.length) $('excluded-list').replaceChildren(el('p', 'Nenhum item ou relação ficou fora da agregação neste snapshot.'));
}
async function query(e) {
  e.preventDefault(); const version = ++requestVersion;
  $('query-button').disabled = true; $('path-result').textContent = 'Consultando o componente de caminho mínimo…';
  try {
    const response = await fetch('/api/path', { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify({ consumer: Number($('consumer').value), changed: Number($('changed').value) }) });
    const result = await response.json(); if (version !== requestVersion) return;
    if (!response.ok) throw new Error(result.error || 'Consulta indisponível');
    $('path-result').replaceChildren();
    if (result.status === 'no_path') $('path-result').append(el('p', 'Os itens existem, mas não há caminho Uses nesta direção no snapshot. Em análise parcial, isso não prova ausência de dependência.'));
    else {
      $('path-result').append(el('strong', `${result.hops} saltos · consumidor → item alterado`));
      const list = el('ol', undefined, 'chain'); list.setAttribute('aria-label', 'Cadeia de dependências');
      result.path.forEach(p => list.append(el('li', p))); $('path-result').append(list);
      if (result.hops === 0) $('path-result').append(el('p', 'Mesmo endpoint: consulta reflexiva, não autoimpacto.'));
    }
    $('path-result').append(el('p', 'Alcance estrutural não comprova quebra comportamental.'));
  } catch (error) { if (version === requestVersion) $('path-result').textContent = `Consulta não concluída: ${error.message}`; }
  finally { if (version === requestVersion) $('query-button').disabled = !data.items.length; }
}
function selectDocument(id) {
  const p = data.documentation;
  const entry = p.entries.find(e => e.node_id === id);
  if (!entry) return;
  $('documentation-node').value = id;
  const body = $('documentation-content'); body.replaceChildren();
  const labels = {matched:'Associação confirmada', ambiguous:'Associação ambígua', unmatched:'Sem associação confirmada'};
  body.append(el('h3', `${labels[entry.status]} · #${id}`));
  body.append(el('p', 'Evidências: ' + entry.evidence.join(', ')));
  if (entry.document) {
    const d = entry.document.symbol;
    body.append(el('h4', d.qualified_name || d.name));
    body.append(el('p', d.brief || 'Sem resumo documental informado.'));
    body.append(el('pre', d.details || 'Sem detalhes documentais informados.'));
  }
  if (entry.status !== 'matched' && entry.candidates.length) {
    body.append(el('h4', 'Candidatos — não são documentação confirmada'));
    entry.candidates.forEach(c => body.append(el('pre', JSON.stringify(c, null, 2))));
  }
  const ids = new Set(entry.candidates.map(c => c.id));
  if (entry.document) ids.add(entry.document.symbol.id);
  const pending = p.pending_relations.filter(r => ids.has(r.from) || ids.has(r.to));
  body.append(el('h4', 'Referências documentais pendentes relevantes'));
  body.append(el('pre', pending.length ? JSON.stringify(pending, null, 2) : 'Nenhuma pendência vinculada aos documentos deste nó.'));
}
function initDocumentation() {
  const p = data.documentation; if (!p) return;
  $('documentation-panel').hidden = false;
  $('documentation-coverage').textContent = `Cobertura documental: ${p.coverage}. Vínculo com os bytes do grafo verificado. Documentação não altera impacto estrutural.`;
  p.entries.forEach(e => { const o=el('option', `#${e.node_id} · ${e.qualified_name || 'nome qualificado desconhecido'}`); o.value=e.node_id; $('documentation-node').append(o); });
  $('documentation-diagnostics').textContent = JSON.stringify({producer:p.producer, graph_digest:p.graph_digest, inventory_digest:p.inventory_digest, catalog_digest:p.catalog_digest, matches_digest:p.matches_digest, inventory:p.inventory_diagnostics, catalog:p.catalog_diagnostics, matches:p.match_diagnostics, pending_relations:p.pending_relations}, null, 2);
  $('documentation-node').addEventListener('change', e => selectDocument(e.target.value));
  if (p.entries.length) selectDocument(p.entries[0].node_id);
}
async function init() {
  try {
    const response = await fetch('/api/data'); if (!response.ok) throw new Error('Pacote indisponível');
    data = await response.json(); if (data.format_version !== 1) throw new Error('Versão de pacote incompatível');
    modules = data.aggregated_graph.nodes;
    cells = new Map(data.cells.map(c => [`${c.from}:${c.to}`, c]));
    evidenceByIndex = new Map(data.relations.map(r => [r.snapshot_index, r]));
    const m = data.metadata;
    for (const [label, value] of [['Provedor', m.provider], ['Linguagem / granularidade', `${m.language} / ${m.granularity}`], ['Configuração', m.configuration], ['Execução', ({partial:'Parcial',complete:'Completa',failed:'Falhou',unknown:'Não informada'})[m.execution]]]) {
      const item = el('div', undefined, 'meta'); item.append(el('span', label), el('strong', value)); $('analysis').append(item);
    }
    const state = ({partial:'Análise parcial',complete:'Execução completa; cobertura semântica não presumida',failed:'Execução com falha',unknown:'Completude não informada'})[m.execution];
    $('state').textContent = `${state}. Cobertura semântica: ${m.semantic_coverage}`;
    $('count').textContent = `${modules.length} módulos · ${data.cells.length} dependências`;
    matrix();
    initDocumentation();
    for (const n of data.items) for (const id of ['consumer', 'changed']) {
      const option = el('option', n.path); option.value = n.id; $(id).append(option);
    }
    if (!data.items.length) {
      $('consumer').disabled = $('changed').disabled = $('query-button').disabled = true;
      $('path-result').textContent = 'Nenhum item disponível para consultar uma cadeia.';
    }
    const internal = data.relations.filter(r => r.destination.kind === 'internal');
    const outside = data.relations.filter(r => r.destination.kind === 'excluded');
    $('excluded-summary').textContent = `${internal.length} relações internas absorvidas · ${outside.length} relações não agregáveis (inclui Owns) · ${data.excluded_items.length} itens sem contenedor`;
    data.limitations.forEach(text => $('limitations').append(el('li', text)));
    (m.diagnostics.length ? m.diagnostics : ['Nenhum diagnóstico adicional informado.']).forEach(text => $('diagnostic-list').append(el('li', typeof text === 'string' ? text : JSON.stringify(text))));
    excluded = [...data.excluded_items.map(value => ({type:'item',value})), ...[...internal,...outside].map(value => ({type:'relation',value}))];
    $('excluded-details').addEventListener('toggle', () => { if ($('excluded-details').open && !excludedShown) showExcluded(); });
    $('excluded-more').addEventListener('click', showExcluded);
    $('more').addEventListener('click', showMore); $('query').addEventListener('submit', query);
  } catch (error) { $('state').textContent = `Não foi possível abrir a análise: ${error.message}`; $('query-button').disabled = true; }
}
init();
