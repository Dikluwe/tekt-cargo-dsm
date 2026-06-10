# Prompt: Medição em Arena — discriminância das chaves de identidade de item (par typst)

**Camada**: Arena (`lab/`) — medição descartável. **Não entrega solução; entrega
números.** Nenhum código de produto é tocado.
**Criado em**: 2026-06-10
**Estado**: `EXECUTADO` (laudo `00_nucleo/lessons/0077-medicao_chave_item.md`)
**Número**: confirmar na Fase 1 o próximo número livre em `00_nucleo/prompt/`
(o 0077 está reservado para a tela lado a lado; se este tomar o 0077, a tela
desliza — seguir a convenção que o 0075/0076 já aplicaram).
**Decisões de origem**: laudo 0076 — o pareamento por path tem sinal **zero**
no par typst (0 pareados, censo member-only). A trilha seguinte é o pareamento
por **identidade de item**, e ela tem uma decisão de desenho aberta: **a
chave**. As candidatas variam em quanto censo mantêm e quanta ambiguidade
deixam — e isso só o dado real responde. Escolher a chave no escuro repetiria
o erro que os laudos 0012/0013 ensinaram a não cometer.
**Pré-requisito**: 0076 (censo member-only; cache do par typst morno);
0045/0075 (`montar_grafo_workspace` + extração resiliente); o padrão de Arena
dos laudos 0021/0032/0038 (programa isolado em `lab/`, portão de sanidade,
relatório sem conclusão).
**Posição**: a verificação que alimenta o desenho do pareamento por
identidade de item. O prompt do produto vem **depois** dela, moldado pelos
números.
**Arquivos afetados**: `lab/medicao-chave-item/{Cargo.toml,src/main.rs,relatorio.md}`,
laudo em `00_nucleo/lessons/` (convenção: experimentos de Arena também ganham
entrada em lessons — laudo 0021).

---

## Pergunta única

Para o par typst (vanilla member-only × cristalino member-only), no nível de
**item**: quanta discriminância cada chave candidata tem — quantos itens
pareariam 1:1, quantos cairiam em ambíguo, e onde as colisões se concentram?

---

## Restrições estruturais (Arena)

- Programa isolado em `lab/medicao-chave-item/`, com `[workspace]` vazio
  próprio (fora do workspace da lente — padrão do 0032).
- Consome os crates da lente **como biblioteca**, por dependência de path
  (`lente_wiring`, `lente_filtro`, `lente_core`) — o mesmo modo do oráculo.
  **Usa, não modifica**: zero toques em código de produto, `Cargo.toml` raiz
  intocado.
- Descartável: sem testes de produto, sem catálogo, sem linhagem L1.
- A saída é `relatorio.md` na Arena + o laudo em lessons. **Sem conclusão**
  no relatório — números e amostras; a decisão da chave fica com o autor,
  na conversa ("dados primeiro, conclusão por quem decide").

---

## Preparação (o par typst)

1. O lado vanilla precisa do symlink temporário
   (`ln -s Cargo.toml.original lab/typst-original/Cargo.toml` no repo do
   typst-crystalline), removido ao final — mesmo cuidado dos laudos
   0075/0076, inclusive em caso de falha.
2. Os caminhos das duas raízes entram por argumento de CLI do programa de
   medição (não hardcoded), para a Arena ser reutilizável noutro par.
3. Cache morno do 0076: a montagem dos dois grafos deve ficar na ordem de
   segundos. Registrar o tempo.

---

## Método

### Montagem dos dois grafos (igual ao caminho de produção)

Para cada raiz: `montar_grafo_workspace` → `filtrar_stdlib` →
`filtrar_nao_membros` (com os membros de `enumerar_membros`, normalização
`'-'→'_'` do 0076). O censo da medição tem que ser **o mesmo** que o produto
usaria — senão os números não transferem.

**Representantes de fantasma**: excluir do censo de itens os nós cujo path
está em `GrafoWorkspace.fantasmas`, e **reportar a contagem excluída por
lado**. Razão: representantes são stubs de referência, não definições; no
nível de item seriam ruído. (No vanilla, esperados ~448, quase todos
`typst_macros::*` — os itens reais desse crate estão ausentes pela falha de
extração do 0075; declarar essa lacuna no relatório como limitação conhecida
do lado antes.)

### Portão de sanidade (obrigatório, antes de qualquer número novo)

Reproduzir contra o laudo 0076: a contagem de nós pós-filtro por lado, a
contagem de fantasmas (448 / 0) e a de third-party removido (434 / 40) têm
que bater. Se não bater, **parar** — qualquer número da medição seria
suspeito.

### O censo de itens

Itens = nós com `kind` ∈ {struct, enum, union, trait, fn, type, const,
static, macro, variant} — **excluindo** `mod` e `crate` (o nível de módulo
já foi medido: é o 0 pareados do 0076). Reportar a distribuição por `kind`
por lado.

### As chaves candidatas (medir todas sobre o mesmo censo)

| | Chave | Definição operacional |
|---|---|---|
| K1 | `(kind, nome)` | censo completo |
| K2 | `(kind, nome)` sem folhas de impl-de-trait | excluir nós com `trait_` ou `trait_ref` preenchido (a família `fmt`/`from`/`clone` do laudo 0021); reportar quantos saíram por lado |
| K3 | `(kind, pai-tipo::nome)` | qualificar com o **pai** via aresta `Owns` **apenas quando o pai é um tipo** (struct/enum/trait/union) — `Counter::get`; itens cujo pai é módulo ficam só com `nome` (qualificar com módulo reintroduziria a dependência de path que zerou o 0076) |
| K4 | `(kind, trait_, pai-tipo::nome)` | K3 + o trait na chave (separa `Counter::<Display>::fmt` de `Counter::<Debug>::fmt`) |

Se a Fase de leitura do dado revelar que alguma definição operacional não é
computável com o que o grafo carrega (ex.: aresta `Owns` ausente para algum
kind), registrar e medir as chaves que sobram — não inventar dado.

### As contagens por chave (o produto da medição)

Para cada chave K, sobre os dois lados:

- itens no censo de K por lado (K2 reduz o censo; as outras não);
- chaves distintas por lado;
- **pareáveis 1:1**: chaves com exatamente 1 item em cada lado;
- **ambíguos**: chaves presentes nos dois lados com >1 item em pelo menos um
  (reportar quantas chaves e quantos itens elas cobrem);
- **sem-par antes / sem-par depois** (chaves de um lado só);
- **top-10 colisões**: as chaves com mais itens, com contagem por lado e 2–3
  paths de amostra de cada lado — para ver **onde** a ambiguidade mora
  (é `new`? é `fmt`? é variante de enum?).

E uma tabela-síntese final: K1–K4 lado a lado com pareáveis / ambíguos /
sem-par, para a comparação entre chaves caber numa olhada.

### Determinismo

Rodar duas vezes; os números têm que ser idênticos (ordenar onde iterar).

---

## O que NÃO entra

- **Implementar o pareamento no produto** — é o prompt seguinte, moldado por
  estes números.
- **Similaridade estrutural / vizinhança** (a trilha pesada) — fora; esta
  medição é só de chaves exatas.
- **Consertar typst-macros / resolvedor de colisão** — a lacuna é declarada,
  não consertada.
- **Tocar produto, fork, spec, ADRs.**

---

## Resultado esperado

- `lab/medicao-chave-item/` com o programa e o `relatorio.md` (números e
  amostras, sem conclusão).
- **Laudo** em `00_nucleo/lessons/`:
  - O portão de sanidade (bateu com o 0076).
  - A tabela-síntese K1–K4 e os top-10 de colisão de cada chave.
  - A contagem de representantes excluídos por lado e a declaração da lacuna
    do typst-macros.
  - A distribuição de itens por kind por lado.
  - Tempo de montagem (cache morno) e o determinismo verificado.
  - O que a medição **não** decide (a escolha da chave — fica com o autor).

---

## Observação metodológica

Mesma forma dos laudos 0021/0032: a Arena existe para mover uma decisão de
desenho de "no escuro" para "com dado", ao custo de um programa descartável.
A hipótese barata (K1 basta) e a cara (só K4 discrimina) são ambas
plausíveis; medir as quatro sobre o mesmo censo custa quase o mesmo que
medir uma. O portão de sanidade contra o 0076 é o que torna os números
confiáveis — sem ele, uma divergência de filtro ou de censo passaria
despercebida e a chave seria escolhida sobre dado errado.

---

## Histórico de Revisões

| Data | Motivo | Arquivos afetados |
|------|--------|-------------------|
| 2026-06-10 | Proposta: medição em Arena da discriminância de 4 chaves de identidade de item (K1 kind+nome; K2 sem folhas de impl-de-trait; K3 qualificada por pai-tipo; K4 com trait) sobre o par typst member-only, com portão de sanidade contra o 0076, exclusão declarada dos representantes de fantasma e relatório sem conclusão. Alimenta o desenho do pareamento por identidade de item. | `lab/medicao-chave-item/*`, laudo em `00_nucleo/lessons/` |
