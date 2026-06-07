# Laudo de Execução — Prompt 0041 (Protótipo do impacto de um diff — colisões na união)

**Camada**: L5 (laudo — registro de Arena)
**Data**: 2026-06-05
**Prompt executado**: `00_nucleo/prompt/0041-proto-impacto-diff-colisoes.md`
**Tipo**: Arena visual, quarta rodada dos laudos 0038/0039/0040 —
bruto em `lab/proto-impacto-diff/`, registro aqui (padrão Arena dos
laudos 0021, 0029, 0036, 0038, 0039, 0040).
**Estado**: `EXECUTADO` — resolução por crate antes da união
funcionando contra o monorepo da lente. 10 colisões censadas e
processadas; 8 resolvidas limpo; 2 com colisão remanescente da regra
ADR-0006 (achado novo). Suíte de produção intacta (**213 verdes +
22 ignored**, idêntica aos laudos 0037–0040).

---

## A resposta da pergunta central

**Sim, o produto deve resolver por crate antes de unir** — sem
reservas para Distintos/MesmoItem; com aviso quando
`NaoDeterminado` ou `DistintosPosRegraColide`.

**Censo do workspace da lente**:

| Crate | Colisões | Distintos | Pós-regra colide |
|---|---:|---:|---:|
| `lente_core` | 4 | 3 | 1 (`Path::from`) |
| `lente_infra` | 3 | 3 | 0 |
| `lente_wiring` | 2 | 1 | 1 (`ErroLente::from`, 4× `From<T>`) |
| `lente_resolve` | 1 | 1 | 0 |
| **Total** | **10** | **8** | **2** |

E1 (vizinhança) **resolve 100%** das colisões em `Veredito`.
`NaoDeterminado` = 0; `MesmoItem` = 0 (coerente com laudo 0021).
**E2 não seria necessária no monorepo — segue justificadamente em
quarentena.**

**Custo: ~0.8 ms para 10 colisões** (0.002% do cold; 0.86% do quente).
Hipótese §5 do prompt ("desprezível") confirmada com folga.

---

## Achado novo: regra ADR-0006 insuficiente para impls genéricos

A regra do `lente_resolve` (ADR-0006) usa o campo `trait` para
nomear cópias distintas (`Tipo::<Trait>::metodo`). Quando todas as
cópias compartilham o **mesmo trait** mas têm **`trait_ref`
distintos** (impls genéricos), os novos paths colidem entre si:

- `lente_core::…::Path::from`: 2 cópias, ambas `trait = "From"` —
  viram `Path::<From>::from` ×2.
  `trait_ref` real: `From<&str>`, `From<String>`.
- `lente_wiring::ErroLente::from`: 4 cópias, todas `trait = "From"`
  — viram `ErroLente::<From>::from` ×4.
  `trait_ref` real: `From<ErroFork>`, `From<ErroAdaptador>`,
  `From<ErroResolve>`, `From<ErroRaio>`.

Detectado na Arena como categoria `DistintosPosRegraColide`. **Não
toca a regra** (zero produto); recomendação registrada para
atualizar a ADR-0006 a usar `trait_ref` quando `trait` é
insuficiente.

---

## Como rodar

```bash
cd lab/proto-impacto-diff

# Resolver (default) — censo + união com paths únicos:
cargo run --release -- --repo "$(cd ../../ && pwd)" --input git \
    --out dados/colisoes-quente-resolver.json

# Sem resolução (baseline cru-fundido):
cargo run --release -- --repo "$(cd ../../ && pwd)" --input git \
    --sem-resolucao --out dados/colisoes-quente-sem-resolver.json

# Antes/depois num path colidido tocado:
cargo run --release -- --repo "$(cd ../../ && pwd)" --input git \
    --simular-tocar-colidido "lente_core::domain::raio::ErroRaio::fmt" \
    --out dados/colisoes-antes-depois.json

python3 -m http.server 8080  # abrir http://localhost:8080/
```

---

## Pipeline implementado (extensão dos 0039/0040)

1–5. Idênticos ao 0040 (metadata → cache → desserializar). **NOVO**:
   `grafos_por_crate_cru` clonado antes da resolução, para
   antes/depois.
6. **NOVO — etapa 5.5 (resolução por crate)**: para cada grafo de
   crate, `detectar_colisoes_grafo` → para cada path colidido,
   `construir_vizinhanca` + `lente_investiga::investigar` (E1) +
   `lente_resolve::aplicar`. Em `NaoDeterminado`, **registra
   diagnóstico** e mantém blob (não falha como o wiring faz). Em
   `Distintos` cujos novos paths colidem entre si (impls
   genéricos), marca como `DistintosPosRegraColide`.
7. União por path (0039): agora encontra paths intra-crate únicos
   para os Distintos limpos.
8. Mapeamento diff→nós + raio (0039). Nós tocados sob path em
   `paths_imprecisos` ganham `raio_impreciso = true` no JSON.
9. **NOVO — antes/depois** (`--simular-tocar-colidido`): refaz união
   sobre `grafos_por_crate_cru` (sem resolver), calcula raio do
   path colidido, e compara com a soma dos raios das cópias
   resolvidas.

---

## Confirmações principais (detalhe no `relatorio.md`)

### 1. E1 cobre 100% do monorepo

Zero `NaoDeterminado`, zero `MesmoItem`. O padrão dominante é
`Display + Debug` em `impl fmt::*::fmt` (8 casos) — vizinhança
disjunta clássica. Confirma o achado do laudo 0021 (typst+egui, 29
crates) em código próprio.

### 2. `Path::from` e `ErroLente::from` expõem buraco da ADR-0006

A regra `Tipo::<Trait>::metodo` colapsa impls de mesmo trait com
argumentos genéricos diferentes. Achado novo, registrado, com
recomendação concreta (usar `trait_ref`). Não corrige na Arena.

### 3. Antes/depois: efeito no raio é zero no monorepo da lente

Todas as 10 colisões são em `fmt`/`from` — folhas comportamentais
(laudo 0021, ~18.5% dos nós). O fork não captura chamadas via
macro/`?`. Logo `raio = 0` nos dois lados. **O efeito real da
resolução é em honestidade**: 4 entradas para `ErroLente::from`
contra 1 entrada na vista cru-fundida.

Para o produto: o caso real do delta apareceria com tipos
colididos **estruturais** (referenciados por path em outros nós) —
não exercitado no diff atual.

### 4. Custo da resolução é insignificante

| Cenário | Total | Resolução | % |
|---|---:|---:|---:|
| Cold | 31.46 s | 0.76 ms | 0.002% |
| Quente | 0.07 s | 0.63 ms | 0.86% |

~0.08 ms por colisão. Cabe no caminho morno do 0040 (~3 s) sem
custo perceptível.

### 5. Resolução não cria fantasmas cross-crate no monorepo

Predição §7 confirmada empiricamente. Razão: as 10 colisões são
todas `fmt`/`from` — impls internos. Nenhum outro crate referencia
o path colidido pelo nome do método (chamada vem de macro/`?`, que
o fork não captura). Logo a renomeação intra-crate é invisível aos
demais. Verificado caso a caso.

### 6. Distinção fantasma-de-resolução vs fantasma-de-edição preservada

A simulação `--simular-renomeacao` do 0040 continua produzindo 1
fantasma esperado para `lente_core::…::No → NoRenomeado` (origens
nomeiam os 4 crates afetados). A resolução adiciona 0 fantasmas.
As duas detecções convivem sem ruído.

---

## Decisões

- **Replicar o laço do wiring na Arena** (D12) — wiring falha em
  `NaoDeterminado`; Arena precisa continuar marcado.
- **`NaoDeterminado` ⇒ blob marcado, não erro** (D13) — caminho de
  "avisar" do prompt; não exercitado em dados reais (zero
  `NaoDeterminado` no monorepo).
- **`DistintosPosRegraColide` é 4ª categoria** (D14) — achado da
  Arena sobre a regra ADR-0006.
- **Pré-resolução guarda grafos crus** (D15) — para antes/depois;
  custo de uma clonagem trivial.

---

## Estado da suíte

| Item | Resultado |
|------|-----------|
| `cargo test --workspace` | **213 verdes + 22 ignored** — idêntica aos laudos 0037–0040 |
| Crates de produção tocados | **Zero** — Arena pura |
| `Cargo.toml` raiz | intocado — `lab/proto-impacto-diff` tem `[workspace]` próprio |
| Subprocessos do cargo (invariante 0023) | dois únicos, intocados |
| Fork tocado | **Não** — só invocado |
| L1/L4 (`lente_investiga`/`lente_resolve`) | tocados como **bibliotecas** (Cargo.toml da Arena), zero edição |

---

## Conteúdo bruto

```
lab/proto-impacto-diff/
├── Cargo.toml             # + lente_investiga + lente_resolve (deps)
├── src/main.rs            # ~1500 linhas: pipeline 0040 + resolução por crate
├── index.html             # UI: + 6 dumps de 0041 no selector
├── cache/<crate>.{json,hash}  # mesmo cache do 0040
├── dados/
│   ├── colisoes-quente-resolver.json       # política=resolver, censo completo
│   ├── colisoes-quente-sem-resolver.json   # baseline cru-fundido
│   ├── colisoes-cold.json                  # cold com resolução
│   ├── colisoes-cold-sem-resolver.json     # cold sem resolução
│   ├── colisoes-antes-depois.json          # ErroRaio::fmt (2 cópias)
│   ├── colisoes-antes-depois-from.json     # ErroLente::from (4 cópias)
│   ├── colisoes-renomeacao.json            # 1 fantasma + 0 da resolução
│   └── (dumps legacy 0038/0039/0040 mantidos)
└── relatorio.md           # conteúdo denso (perguntas do 0041 + D12–D15)
```

Conteúdo denso em `relatorio.md` (censo detalhado, achado da regra
ADR-0006, antes/depois, tabela de custos, decisões D12–D15).

---

## Para a próxima rodada

| Item | Estado |
|---|---|
| Resolver por crate antes de unir | **Coberto** — 100% E1, custo desprezível |
| Censo de colisões | **Coberto** — 10 no monorepo, padrão Display+Debug |
| Aviso de `NaoDeterminado` | **Implementado** — não exercitado (zero no monorepo) |
| Antes/depois num path colidido | **Coberto** — sem efeito numérico no monorepo (todas as colisões são folhas comportamentais) |
| Achado regra ADR-0006 (`trait_ref`) | **Documentado** — recomendação para o produto |
| Órfãos cross-crate da resolução | **Coberto** — 0 no monorepo (predição §7 confirmada) |
| Atualizar ADR-0006 para usar `trait_ref` | **Aberto** — primeiro prompt do produto |
| Modo `--diff` na CLI | **Aberto** — Ponte 2 da trilha local |
| Casca MCP | **Aberto** |
| Untracked (achado 0038) | **Aberto** |
| Filtros para diffs grandes | **Aberto** |
| Cache key inclui `Cargo.toml` | **Aberto** |

---

## Histórico de Revisões

| Data | Motivo | Arquivos afetados |
|------|--------|-------------------|
| 2026-06-05 | Quarta rodada da Arena `lab/proto-impacto-diff/` — fecha gap de colisões na união multi-crate: censo por crate, resolução via `lente_investiga`(E1) + `lente_resolve` como bibliotecas (zero toque em L1/L4), **antes** da união por path. Censo: 10 colisões no workspace, todas `Distintos` por vizinhança; 8 resolvem limpo, 2 colidem pós-regra (achado novo — ADR-0006 usa `trait` mas não distingue impls de `From<T>` genéricos; recomendação registrada para usar `trait_ref`). Zero `NaoDeterminado` / `MesmoItem` no monorepo — E2 (em quarentena) **não** seria necessária. Custo da resolução: ~0.8 ms para 10 colisões (0.002% do cold). Antes/depois num path colidido tocado: sem efeito numérico no monorepo (colisões são folhas comportamentais `fmt`/`from` — fork não captura chamadas via macro/`?`); efeito real é honestidade da contagem de cópias. Predição §7 confirmada: 0 fantasmas cross-crate criados pela resolução, porque colisões são impls internas. Distinção fantasma-de-resolução vs fantasma-de-edição preservada. Veredito: o produto deve resolver por crate antes de unir; e atualizar ADR-0006 para usar `trait_ref` quando aplicável. Zero toque no produto. | `lab/proto-impacto-diff/{Cargo.toml,src/main.rs,index.html,dados/colisoes-*.json,relatorio.md}`, `00_nucleo/lessons/0041-proto-impacto-diff-colisoes.md` |
