// Crystalline Lineage — @prompt ../CONTRATO.md
'use strict';
const $ = id => document.getElementById(id);
const el = (tag, text, cls) => { const e = document.createElement(tag); if (text !== undefined) e.textContent = text; if (cls) e.className = cls; return e; };
const statusLabel = {matched:'matched · Confirmada', ambiguous:'ambiguous · Ambígua', unmatched:'unmatched · Sem associação'};
const locationText = loc => loc ? `${loc.file}:${loc.line}` : 'Localização não informada';
let data, selected;
function block(parent,title,text) {parent.append(el('h3',title),el('p',text,'preserve'));}
function select(id, updateHash=true) {
  selected = id;
  if (updateHash) history.replaceState(null,'',`#node=${encodeURIComponent(id)}`);
  for (const button of $('nodes').querySelectorAll('button')) button.setAttribute('aria-current',String(button.dataset.nodeId === id));
  const p = $('panel'); p.replaceChildren();
  const n = data.entries.find(entry => entry.node_id === id);
  if (!n) { p.append(el('h2','Elemento não encontrado'),el('p','Selecione um node_id presente no catálogo.')); return; }
  p.append(el('span',`NODE ${n.node_id}`,'eyebrow'),el('h2',n.qualified_name ?? 'Nome estrutural não informado'),el('p',locationText(n.location),'location'),el('span',statusLabel[n.status],`badge ${n.status}`));
  if (n.signature !== null) block(p,'Assinatura estrutural',n.signature);
  block(p,'Evidências declaradas',n.evidence.length ? n.evidence.join(' · ') : 'Nenhuma evidência informada.');
  const article=el('article'); p.append(article);
  if (n.status==='matched') {
    const d=n.document.symbol;
    article.append(el('span','DOCUMENTAÇÃO ASSOCIADA','eyebrow'),el('h3',d.qualified_name ?? d.name),el('p',`${d.kind} · ${d.id} · ${locationText(d.location)}`,'muted'));
    block(article,'Resumo',d.brief || 'Resumo não informado.');
    block(article,'Detalhes',d.details || 'Documentação detalhada não informada.');
    if (n.document.safe_url) {
      const a=el('a','Abrir documentação ↗'); a.href=n.document.safe_url; a.target='_blank'; a.rel='noopener noreferrer'; article.append(a);
    } else if (d.documentation_url) block(article,'URL não habilitada pela política HTTP/HTTPS',d.documentation_url);
  } else if (n.candidates.length) {
    article.append(el('h3',n.status==='ambiguous' ? 'Associação ambígua. Nenhum documento escolhido.' : 'Candidatos sem evidência suficiente. Nenhuma associação confirmada.'),el('p','Os candidatos abaixo são referências para inspeção. Sua documentação não é apresentada como pertencente a este nó.'));
    const list=el('ul'); for (const c of n.candidates) { const li=el('li'); li.append(el('strong',c.qualified_name ?? c.name),el('p',`${c.id} · ${locationText(c.location)}${c.signature ? ` · ${c.signature}` : ''}`)); list.append(li); } article.append(list);
  } else article.append(el('h3','Sem documentação associada'),el('p','O inventário não oferece uma correspondência confirmada para este elemento. Isso não comprova ausência de documentação no projeto.'));
  const ids=new Set(n.candidates.map(c=>c.id)); if(n.document) ids.add(n.document.symbol.id);
  const pending=data.pending_relations.filter(r=>ids.has(r.from)||ids.has(r.to));
  block(p,'Referências documentais pendentes relevantes',pending.length ? JSON.stringify(pending,null,2) : 'Nenhuma pendência vinculada.');
}
function renderList() {
  const query=$('search').value.toLocaleLowerCase();
  const entries=data.entries.filter(n => `${n.node_id} ${n.qualified_name ?? ''} ${n.location?.file ?? ''}`.toLocaleLowerCase().includes(query));
  $('count').textContent=`${entries.length} de ${data.entries.length} elementos`;
  $('nodes').replaceChildren();
  if (!entries.length) $('nodes').append(el('p','Nenhum elemento encontrado. Limpe a busca para ver o catálogo.','empty'));
  for (const n of entries) {
    const b=el('button'); b.type='button'; b.dataset.nodeId=n.node_id; b.setAttribute('aria-current',String(n.node_id===selected));
    b.append(el('strong',n.qualified_name ?? `Nó ${n.node_id}`),el('span',`#${n.node_id} · ${statusLabel[n.status]}`));
    b.addEventListener('click',()=>select(n.node_id));
    b.addEventListener('keydown',event=>{
      const buttons=[...$('nodes').querySelectorAll('button')], index=buttons.indexOf(b);
      const target={ArrowDown:Math.min(index+1,buttons.length-1),ArrowUp:Math.max(index-1,0),Home:0,End:buttons.length-1}[event.key];
      if (target!==undefined) {event.preventDefault();buttons[target].focus();}
    }); $('nodes').append(b);
  }
}
function provenance() {
  $('coverage').textContent=`Vínculos verificados · cobertura ${ {partial:'parcial',complete:'completa na entrada declarada',unknown:'desconhecida'}[data.coverage] }`;
  const p=$('provenance');
  block(p,'Produtor',`${data.producer.name} · versão ${data.producer.version ?? 'não informada'} · configuração ${data.producer.configuration_digest ?? 'não informada'}`);
  for (const key of ['inventory_digest','catalog_digest','matches_digest','graph_digest']) block(p,key,data[key]);
  p.append(el('p','O digest do grafo é apenas declarado no catálogo; o grafo não foi fornecido para verificação. Digests não autenticam o produtor.'));
  block(p,'Referências documentais pendentes',JSON.stringify(data.pending_relations,null,2));
  block(p,'Documentos sem associação confirmada',data.unmatched_document_ids.join(', ') || 'Nenhum.');
  for (const [title,ds] of [['Diagnósticos do catálogo',data.catalog_diagnostics],['Diagnósticos do inventário',data.inventory_diagnostics],['Diagnósticos das correspondências',data.match_diagnostics]]) {
    p.append(el('h3',title)); if(!ds.length) p.append(el('p','Nenhum diagnóstico informado.'));
    for (const d of ds) p.append(el('p',`${d.code}${d.symbol_id ? ` [${d.symbol_id}]` : ''}: ${d.message}`,'preserve'));
  }
}
function hashSelection() { const match=location.hash.match(/^#node=(\d+)$/);return match?.[1]; }
async function start() {
  try {
    const response=await fetch('/api/data',{cache:'no-store'});if(!response.ok) throw Error(`HTTP ${response.status}`);
    data=await response.json();provenance();selected=hashSelection() ?? data.entries[0]?.node_id;renderList();
    if (selected!==undefined) select(selected,false);else {$('panel').replaceChildren(el('h2','Catálogo vazio'),el('p','Não há elementos estruturais para selecionar.'));}
    $('search').addEventListener('input',renderList);window.addEventListener('hashchange',()=>select(hashSelection(),false));
  } catch(error) {$('coverage').textContent='Não foi possível carregar os dados';$('panel').replaceChildren(el('h2','Entradas indisponíveis'),el('p',String(error)));}
}
start();
