# Laudo de Execução — Prompt 0042 (`lente_resolve` — escada `trait_` → `trait_ref` → contador)

**Camada**: L1 — Núcleo (`lente_resolve`)
**Data**: 2026-06-05
**Prompt executado**: `00_nucleo/prompt/0042-resolve_escada_trait_ref.md`
**Estado**: `EXECUTADO` — escada implementada, ADR-0006 emendada,
testes adicionados, caso real fechado, suíte verde.

---

## A correção em uma sentença

A regra de nomeação do `lente_resolve` (ADR-0006) passa de `trait_` único
para uma **escada**: degrau 1 = `<trait_>`; degrau 2 = `<trait_ref>` se
2+ nós ficam com o mesmo nome no degrau 1; degrau 3 = contador `#N` como
piso. Resultado: o invariante "paths únicos após resolução" volta a valer
para impls genéricos do mesmo trait — `Path::from` (2× `From<T>`) e
`ErroLente::from` (4× `From<T>`) agora ganham paths únicos via
`trait_ref`.

---

## O que mudou

### Em `06_resolve/src/lib.rs`

`aplicar_distintos(grafo, colisao, ids_colidentes) -> Grafo` agora
implementa a escada:

```text
1. Para cada id: tentar nome "Tipo::<trait_>::metodo". Nó sem trait_
   vai direto ao contador (degrau 3).
2. Agrupar os nomes do degrau 1. Para cada grupo de tamanho 1: fica.
   Para cada grupo de tamanho ≥ 2: reescrever esses ids pelo
   `<trait_ref>`; nó sem `trait_ref` cai no contador. Se o degrau 2
   ainda colide entre dois ids (patológico), também → contador.
3. Pendentes: contador `#N`, N = posição do id no `ids` sortido + 1
   (preserva semântica do laudo 0010 D9 — id=1 com `<Display>`, id=2
   sem trait → `#2`, não `#1`).
```

O `trait_ref` é lido do `No.trait_ref` (cascata do descritor; laudos
0012/0013). O fork já o emite — o laudo 0041 leu os valores reais
(`From<&str>`, `From<String>`, etc.).

**O resto não mudou**: redistribuição de arestas por `id_from`/`id_to`;
`MesmoItem`; `NaoDeterminado`; erros (`ColisaoInexistente`, etc.);
pureza L1.

### Em `00_nucleo/adr/0006-nomeacao-trait-padrao.md`

**Emenda** (não nova ADR — caso M3 granular). Acrescenta seção "Ajuste
(laudo 0042) — escada `trait_` → `trait_ref` → contador" com: contexto
(o buraco silencioso), decisão (a escada de 3 degraus), porquê emenda
(refina o "trait" sem mudar fundo), verificação no caso real,
contagem da suíte.

A regra de fundo (trait por padrão, contador como piso, sem flag) **não
muda**. A escada só amplia o "trait" para uma cascata que sempre
converge.

---

## A confirmação no caso real

Pré-condição: `lente_resolve` reescreve as 10 colisões do workspace da
lente, executado via `lab/proto-impacto-diff` (Arena do laudo 0041).

| Métrica | Antes (laudo 0041) | Depois (laudo 0042) |
|---|---:|---:|
| Colisões totais | 10 | 10 |
| Distintos limpos | 8 | **10** |
| `DistintosPosRegraColide` | 2 | **0** |
| `NaoDeterminado` | 0 | 0 |
| `MesmoItem` | 0 | 0 |

Paths gerados para os dois casos que antes colidiam:

**`lente_core::entities::grafo::Path::from`** (2 cópias):
- `lente_core::entities::grafo::Path::<From<&str>>::from`
- `lente_core::entities::grafo::Path::<From<String>>::from`

**`lente_wiring::ErroLente::from`** (4 cópias):
- `lente_wiring::ErroLente::<From<ErroFork>>::from`
- `lente_wiring::ErroLente::<From<ErroAdaptador>>::from`
- `lente_wiring::ErroLente::<From<ErroResolve>>::from`
- `lente_wiring::ErroLente::<From<ErroRaio>>::from`

Os 8 casos `Display + Debug` continuam virando `<Display>::fmt` /
`<Debug>::fmt` (degrau 1 já distingue — escada não escala).

---

## Testes adicionados (7 novos)

Em `06_resolve/src/lib.rs`, `mod tests`:

1. `escada_d2_path_from_dois_impl_genericos_se_distinguem_por_trait_ref`
   — caso real do laudo 0041 (2 cópias).
2. `escada_d2_erro_lente_from_quatro_impl_genericos_se_distinguem`
   — caso real do laudo 0041 (4 cópias).
3. `escada_d1_display_debug_nao_escala_para_d2_nem_d3`
   — não-regressão do caso canônico (`<Display>` / `<Debug>`).
4. `escada_d3_trait_ref_ausente_no_grupo_colidindo_cai_no_contador`
   — degrau 3 ativado por `trait_ref = None` num grupo `trait_`
   colidindo.
5. `escada_d3_patologico_trait_ref_identicos_cai_no_contador`
   — piso pegando o caso impossível-no-Rust mas construível em teste.
6. `escada_mistura_d1_ok_e_d2_resolvendo`
   — um nó `Display` resolve no d1, dois `From<X>`/`From<Y>` escalam para
   o d2.
7. `escada_determinismo_aplicar_duas_vezes`
   — `HashMap` interno não vaza ordem; aplicar 2× dá grafos iguais.

Helper novo: `no_com_trait_e_ref(id, path, trait_, trait_ref)` para
construir nós com ambos campos.

---

## Contagem da suíte (reconciliação)

| Crate | Antes (laudo 0041) | Depois (laudo 0042) | Δ |
|---|---:|---:|---:|
| `lente_resolve` | 11 verdes | **18 verdes** | +7 (escada) |
| Outros 9 crates | 202 verdes + 22 ignored | 202 verdes + 22 ignored | 0 |
| **Workspace** | **213 + 22 ignored** | **220 + 22 ignored** | **+7** |

Nenhum teste anterior precisou ser modificado. Os 11 testes pré-0042 do
`lente_resolve` continuam passando — a escada **estende** a regra, não
substitui o caminho que eles cobrem.

---

## Pureza L1 preservada

```
$ cargo tree -p lente_resolve
lente_resolve v0.0.0
└── lente_core v0.0.0
```

Zero deps externas. `trait_ref` lido diretamente do `No.trait_ref` (já
existia desde o laudo 0012, cascata do descritor) — não precisou de
campo novo, nem de leitura de fontes (E2 segue em quarentena).

---

## Decisões registradas

### D16 — Emenda à ADR-0006, não nova ADR

Caso M3 (granular): a regra de fundo (trait por padrão, contador como
piso, sem flag) **não muda**; a escada é refinamento. Coerente com
laudo 0008 (emenda) ao invés de superseção.

### D17 — Contador com índice global, não local

`#N` usa a posição do id em `ids` (sortido), não em `pendentes_contador`.
Preserva o teste antigo `distintos_mistura_trait_e_contador` (id 2 sem
trait num conjunto onde id 1 tem `<Display>` → `#2`, não `#1`). Coerente
com a interpretação do laudo 0010 D9.

### D18 — `trait_ref = None` ⇒ contador, não inventar nome

Se um nó cujo `trait_` colidiu não tem `trait_ref`, vai para o piso
(degrau 3). Não tenta heurísticas (parser de fontes, etc.) — segue a
política "L1 puro: o nome vem do nó ou do contador".

---

## Não-regressão coordenada

A correção fecha uma **violação latente do invariante "paths únicos"**
que os testes unitários (Display+Debug) não pegavam, mas que aparecia em
**execuções reais** do `lente_resolve` sobre o grafo do próprio repo.
Verifiquei se algum teste de integração / E2E mudaria de comportamento:

- E2E ignored `lente_wiring::tests::e2e_lente_core_renomeia_erro_raio_fmt`
  (requer fork instalado) — não afetado: o alvo do teste é
  `lente_core::domain::raio::Raio` (não-colidente). Pipeline funciona
  porque as colisões internas agora resolvem limpo (antes eram
  "Distintos com paths colidindo entre si" — passavam no `Veredito`
  mas violavam o invariante).
- Restante da suíte: idêntico.

Em uso real (via `cargo run -- --pacote lente_wiring --ranking`, etc.),
as colisões pré-0042 produziam grafos com paths repetidos
silenciosamente — o invariante de unicidade era um contrato **não
testado**. A escada o restaura.

---

## Histórico de Revisões

| Data | Motivo | Arquivos afetados |
|------|--------|-------------------|
| 2026-06-05 | `aplicar_distintos` do `lente_resolve` passa a escada `<trait_>` → `<trait_ref>` → `#N`: degrau 1 inalterado (`<Display>`/`<Debug>` continuam); degrau 2 novo, ativado quando 2+ nós do conjunto ficam com mesmo nome no d1 (mesmo `trait_`); degrau 3 é o piso (contador `#N`, índice no `ids` sortido global, preservando D9 do laudo 0010). `trait_ref` lido de `No.trait_ref` (já existia, cascata do descritor — zero campo novo). Pureza L1 preservada (`cargo tree -p lente_resolve` só `lente_core`). 7 testes novos (`From<T>` 2 e 4 cópias = caso real do laudo 0041; `Display+Debug` não-regressão; `trait_ref = None` cai no contador; patológico `trait_ref` idênticos no contador; mistura d1+d2; determinismo). 11 testes pré-0042 inalterados, todos passando. Suíte workspace: 213 + 22 ignored → **220 + 22 ignored** (+7). Caso real (lab/proto-impacto-diff/ Arena do laudo 0041): de 8 limpos + 2 `DistintosPosRegraColide` para **10 limpos + 0 colisões pós-regra**. ADR-0006 emendada (caso M3 granular — refina "trait" para escada, fundo inalterado). | `06_resolve/src/lib.rs`, `00_nucleo/adr/0006-nomeacao-trait-padrao.md`, `00_nucleo/lessons/0042-resolve-escada-trait-ref.md` |
