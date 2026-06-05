# Laudo de Execução — Prompt 0030 (Escopo como escolha do usuário)

**Camada**: L5 (laudo)
**Data**: 2026-06-03
**Prompt executado**: `00_nucleo/prompt/0030-escopo-usuario.md`
**Estado**: `EXECUTADO` — `Escopo { Completo, SeuCodigo }` é parâmetro
dos dois pipelines no `lente_wiring`; helper único `obter_grafo` decide
filtrar-ou-não; flag CLI `--filtrar-stdlib` (default ausente = Completo);
saída (JSON e texto) declara `escopo` nos dois modos. Achado 2 do laudo
0029 (classificação divergente em silêncio) **resolvido**: agora a
diferença é **rotulada**, não silenciosa. 152 verdes + 16 ignored (era
143/15); zero subprocesso novo; pureza do L1 intacta; `lente_filtro`
intacto (só passa a ser aplicado condicionalmente).

---

## Fase 1 — Confirmação do invariante central

Antes de implementar: confirmar que **filtrar stdlib não altera o
montante** de um nó do código do usuário. A razão é simples:
`filtrar_stdlib` remove nós cujo path começa em sysroot e arestas que
os tocam; o montante de um nó-alvo do código do usuário (via reverse-uses
BFS) só contém nós que **dependem** do alvo; e stdlib não depende de
código do usuário (direção é alvo→stdlib, não o contrário). Logo:

- `uses_entrada` (= "diretos") — **invariante** ao escopo.
- `montante.len()` (= "transitivos") — **invariante** ao escopo.
- `uses_saida` — **muda** (arestas saindo para stdlib somem).
- `classificacao` — **pode mudar** (depende de `uses_saida`).

Verificado pelo teste de unidade `invariante_do_montante_diretos_e_transitivos_sao_iguais_entre_escopos`
e confirmado contra dado real (`lente_core::entities::grafo::Path`):

| Métrica | Completo | SeuCodigo |
|---------|----------|-----------|
| Diretos | 19 | 19 |
| Transitivos | 39 | 39 |
| Uses_saida | 1+ (para stdlib) | 0 |
| Classificação | **Intermediário** | **Base** |

A invariância é o que torna a flag honesta: a **resposta central** da
lente ("quem quebra se eu mexer aqui?") não muda com o escopo; só a
classificação e quem povoa o ranking.

---

## Fase 2 — Implementação

### Estrutura

```
04_wiring/src/lib.rs
  + pub enum Escopo { Completo, SeuCodigo }    (Default = Completo)
  + fn obter_grafo(fonte, escopo) → Result<Grafo, ErroLente>   (helper único)
  ~ calcular_raio_de_alvo(fonte, alvo, escopo) → Result<Raio, ErroLente>
  ~ rankear_pacote(fonte, n, escopo) → Result<Vec<ItemRanking>, ErroLente>

02_shell/catalogo/src/lib.rs
  + HELP_FILTRAR_STDLIB
  + ROTULO_ESCOPO / JSON_ESCOPO
  + ESCOPO_COMPLETO ("completo") / ESCOPO_SEU_CODIGO ("seu-codigo")
  ~ RANKING_CABECALHO inclui {escopo}: "Ranking de impacto (escopo: {escopo}) — top {n}:"

02_shell/cli/src/args.rs
  + #[arg(long="filtrar-stdlib")] pub filtrar_stdlib: bool

02_shell/cli/src/saida.rs
  ~ formatar(raio, alvo_pedido, escopo, modo)        ← +1 parâmetro
  ~ formatar_ranking(itens, escopo, modo)            ← +1 parâmetro
  + escopo_texto(Escopo) → &'static str              (privado, traduz para o catálogo)
  ~ JSON do raio:    inclui "escopo":"completo"|"seu-codigo"
  ~ JSON do ranking: chave "escopo" no topo (ranking inteiro é de um escopo)
  ~ Texto do raio:   linha "Escopo:\t<valor>"
  ~ Texto do ranking: cabeçalho "Ranking de impacto (escopo: <valor>) — top N:"

02_shell/cli/src/main.rs
  + fn escolher_escopo(cli) → Escopo                 (mapeamento flag → enum)
  ~ run() / run_ranking() propagam escopo
```

### Decisão central — helper único

```rust
fn obter_grafo(fonte: FonteGrafo, escopo: Escopo) -> Result<Grafo, ErroLente> {
    let grafo = obter_grafo_resolvido(fonte)?;
    Ok(match escopo {
        Escopo::SeuCodigo => filtrar_stdlib(&grafo),
        Escopo::Completo => grafo,
    })
}
```

Os **dois** pipelines chamam `obter_grafo(fonte, escopo)`. A coerência
entre eles sai de **um único** local de decisão, não de duplicação.

### Mudança de default no ranking (deliberada, declarada)

| | Pré-0030 | Pós-0030 |
|---|---------|----------|
| Ranking sem flag | filtrava sysroot (laudo 0027) | **Completo** — sysroot no topo |
| Ranking com `--filtrar-stdlib` | (não existia a flag) | recupera o ranking do laudo 0027 |
| Raio per-nó sem flag | nunca filtrava | **Completo** (mesmo comportamento) |
| Raio per-nó com `--filtrar-stdlib` | (não existia a flag) | filtra antes de calcular |

A mudança de default do ranking **é** o conserto: agora os dois modos
têm o **mesmo** default e o **mesmo** mecanismo de filtrar. O Achado 2
do laudo 0029 (classificação divergente entre `--ranking` e `--alvo`
sem aviso) deixa de existir — em vez disso, a diferença é **rotulada
em ambas as saídas**.

---

## Verificação

### Suíte (sem ignored)

| Crate | Verdes | Δ vs 0029 |
|-------|--------|-----------|
| lente_core | 30 | 0 |
| lente_infra | 30 | 0 |
| lente_investiga | 17 | 0 |
| lente_resolve | 11 | 0 |
| lente_filtro (lib) | 10 | 0 |
| lente_ranking | 8 | 0 |
| **lente_wiring** | **12** | **+3** (rankear_completo, invariante_montante, alvo_stdlib_seu_codigo) |
| lente_catalogo | 7 | 0 |
| **lente_cli** | **27** | **+6** (ranking_filtrado, ranking_default, ranking_text_com_escopo, raio_default, raio_filtrado, +1 saída) |
| **Total** | **152** | **+9** |

### Ignored (todos passam quando rodados)

| Item | Ignored | Δ |
|------|---------|---|
| lente_infra | 8 | 0 |
| lente_filtro (E2Es em `tests/`) | 3 | 0 |
| **lente_wiring** | 3 | **+1** (`e2e_ranking_do_lente_core_completo_traz_sysroot`) |
| lente_cli | 2 | 0 (renomeado, não duplicado) |
| **Total** | **16** | **+1** |

E2Es rodados: todos verdes.

### Output real (Achado 2 ROTULADO)

`lente --pacote lente_core --ranking --top 5 --text --filtrar-stdlib`:

```
Ranking de impacto (escopo: seu-codigo) — top 5:
  #  Impacto  Classificação    Path
   1       39  Base             lente_core::entities::grafo::Path
   2       17  Base             lente_core::entities::grafo::Kind
   3       17  Base             lente_core::entities::grafo::Modificadores
   4       17  Base             lente_core::entities::grafo::Relation
   5       17  Base             lente_core::entities::grafo::Visibility
```

`lente --pacote lente_core --ranking --top 5 --text` (default Completo):

```
Ranking de impacto (escopo: completo) — top 5:
  #  Impacto  Classificação    Path
   1       62  Base             alloc::string::String
   2       39  Intermediário    lente_core::entities::grafo::Path
   3       21  Base             core::result::Result
   4       19  Base             core::option::Option
   5       17  Base             core::fmt::Error
```

`lente --pacote lente_core --alvo lente_core::entities::grafo::Path --text`
(default Completo):

```
Alvo:           lente_core::entities::grafo::Path
Escopo:         completo
Classificação:  Intermediário
Impacto direto: 19 itens
Transitivo:     39 itens
```

Adicionando `--filtrar-stdlib`:

```
Alvo:           lente_core::entities::grafo::Path
Escopo:         seu-codigo
Classificação:  Base
Impacto direto: 19 itens
Transitivo:     39 itens
```

**O que ler no contraste**:

1. **Invariante do montante confirmado contra dado real**: nos dois
   raios de `Path`, `Impacto direto = 19` e `Transitivo = 39` — iguais.
   Só `Classificação` muda (Intermediário ↔ Base).
2. **Achado 2 do laudo 0029 rotulado**: o mesmo `Path` aparece como
   `Intermediário` no ranking Completo (pos. 2) e como `Base` no
   ranking SeuCodigo (pos. 1). Antes do 0030, o leitor via "Base no
   ranking" e "Intermediário no detalhe" sem motivo aparente; agora,
   cada saída diz **qual escopo** foi calculada.
3. **Mudança de default no ranking visível**: o top-1 do default agora
   é `alloc::string::String` (62 impacto) — sysroot, como o laudo 0021
   já havia mostrado, domina o ranking quando não há filtro.

### Invariante dos dois subprocessos do cargo (laudo 0023)

```
$ grep -rn 'Command::new("cargo")' --include "*.rs"
03_infra/src/fork.rs:117      # cargo modules
03_infra/src/metadata.rs:170  # cargo metadata
```

Continua dois únicos.

### Pureza do L1

`lente_filtro` (L1) **intacto** — só passa a ser chamado
condicionalmente pelo `obter_grafo` em `04_wiring`. `cargo tree -p
lente_filtro --depth 1` continua mostrando só `lente_core`.

---

## Decisões tácitas

### D1 — Enum forte `Escopo`, default = `Completo`

Em vez de `bool filtrar_stdlib` puro nas assinaturas da fiação, enum.
Preferência declarada do projeto e instinto-bom-precedente do
`AlvoFork`/`Kind`/`Visibility` etc. Vantagens:

- Match exaustivo no `obter_grafo` (compilador garante cobertura).
- API auto-descritiva: `calcular_raio_de_alvo(fonte, alvo, Escopo::SeuCodigo)`
  é claro; com bool seria ambíguo.
- Espaço para variantes futuras (`SoMeuCrate`, etc.) sem reescrita.

`Default = Completo` casa com o que o usuário espera ler quando não
escolhe — a forma mais fiel do grafo. A flag CLI é
**presença/ausência** (idiomático clap), não `--escopo
<completo|seu-codigo>` — mais curto, suficiente para 2 valores.

### D2 — Helper `obter_grafo` privado, sobre `obter_grafo_resolvido`

`obter_grafo_resolvido` (do laudo 0027) **fica** — é o grafo cru-mas-resolvido.
`obter_grafo` é a camada acima, que decide se aplica o filtro. Dois
níveis para duas responsabilidades. Privado, sem `pub` — a fronteira
de API do crate continua sendo os dois pipelines completos. Quando um
terceiro modo aparecer, promove-se.

### D3 — Alvo de stdlib + `SeuCodigo` → `AlvoInexistente`

Caso de borda: o usuário pede `--alvo core::fmt::Display --filtrar-stdlib`.
Comportamento: o filtro remove o nó antes do `calcular_raio` → 
`ErroRaio::AlvoInexistente`. Consistente: você pediu para filtrar a
stdlib **e** consultou um nó dela; a lente diz "esse nó não está aqui",
não "esse nó é stdlib". Coerente com a regra geral
"alvo inexistente vira erro próprio". Teste
`alvo_de_stdlib_no_escopo_seu_codigo_da_alvo_inexistente` ancora.

### D4 — Catálogo: `RANKING_CABECALHO` recebe `{escopo}`

A constante mudou de `"Ranking de impacto — top {n}:"` para
`"Ranking de impacto (escopo: {escopo}) — top {n}:"`. Outros literais
ficariam mais limpos com `Template` por modo, mas adicionar **uma**
variável ao template existente custa zero código novo e mantém o
catálogo enxuto. Quem renderiza passa as duas chaves.

### D5 — `escopo_texto(Escopo) → &'static str` na `saida.rs`

Função privada no L2 que mapeia `lente_wiring::Escopo` → string do
catálogo. Alternativa rejeitada: `impl Display for Escopo` no
`lente_wiring`. Por quê não:

- A string-pública do `Escopo` é **apresentação** (vive no catálogo,
  governado pelo ADR-0002 do Tekt). Pô-la no `Display` do tipo
  amarra-se ao tipo, e seria duplicação se o catálogo um dia
  internacionalizar.
- A função-tradutora local fica testável de modo simples e ancora a
  ligação `tipo do wiring ↔ string do catálogo` num único ponto da L2.

### D6 — Default novo do ranking é declarado, não silencioso

A regra do prompt: a mudança de default (filtrava → não filtra) **é
intencional** e o laudo declara. Adicionei dois testes que ancoram:

- `rankear_completo_traz_sysroot` (unit, wiring): garante que o default
  traz sysroot.
- `e2e_ranking_do_lente_core_completo_traz_sysroot` (ignored, wiring):
  ancoragem contra dado real.

Se algum laudo futuro quiser voltar ao default filtrado, terá que tocar
estes testes — fica rastreável.

### D7 — Re-export `lente_wiring::Escopo`

A CLI consome `lente_wiring::Escopo` (igual ao padrão `ItemRanking` do
laudo 0027, D2). L2 fala com **uma** camada acima — o `lente_filtro`
e o `lente_ranking` ficam transparentes para a CLI; só o wiring é
fronteira.

### D8 — Arena `lab/proto-ui` não foi re-capturada

O prompt declara opcional re-capturar os dumps com o campo `escopo`
para o protótipo poder mostrar um selo. Decidi **não** mexer na Arena
neste laudo:

- O contrato do prompt 0030 é o sistema, não a Arena.
- O protótipo é descartável (laudo 0029); mudá-lo agora seria
  manutenção de Arena, contra o "Arena fica como registro do
  experimento".
- O dump pode ser re-capturado quando alguém for nuclear a UI — o
  próximo prompt da trilha de UI naturalmente vai precisar.

---

## Pendências cobertas / abertas

| Pendência | Estado pós-0030 |
|-----------|-----------------|
| Achado 2 do laudo 0029 (classificação divergente em silêncio) | **Coberta** — escopo rotulado em ambos os modos; default igual. |
| Achado 1 do laudo 0029 (`impactados` sem profundidade nem arestas) | **Aberta** — prompt próprio se a UI pedir. |
| Nuclear a UI | **Aberta** — depende do Achado 1 e do que a UI real vier pedir. |
| Filtro de "folhas comportamentais" (Limite 3) | **Aberta** — trilha separada. |

---

## O que NÃO mudou

- **`lente_filtro` (L1)**: zero toques. Só passou a ser chamado condicionalmente.
- **`lente_core` (L1)**: zero toques. Pureza intacta.
- **Fork (`cargo-modules`)**: zero toques.
- **ADRs**: zero toques.
- **Spec da forma** (laudo 0028): zero toques.
- **Quarentena E2**: intacta.
- **Subprocessos do cargo** (invariante 0023): dois únicos.
- **Arena `lab/proto-ui`**: intocada (decisão D8).
- **Padrão da CLI** (`--text`, `--verbose`, `--pacote`, `--grafo`,
  `--alvo`, `--alvo-id`, `--ranking`, `--top`): **inalterado**;
  `--filtrar-stdlib` é aditivo, ortogonal aos demais.

---

## Observação metodológica

O Achado 2 do laudo 0029 era uma **decisão omitida** — divergência
semântica que só apareceu ao desenhar contra dado real (o ganho lateral
da Arena). O conserto **não foi escolher uma das respostas** (filtrar
sempre ou nunca), mas **tornar a escolha explícita** e **rotular a
saída**. As duas respostas viraram **legíveis** em vez de
**contraditórias**.

É a extensão do princípio "dados primeiro, conclusão por quem decide"
do projeto, agora estendido **do autor para o usuário final**: a lente
não decide qual pergunta o usuário faz; ela responde a que ele pediu, e
**diz qual foi**.

---

## Histórico de Revisões

| Data | Motivo | Arquivos afetados |
|------|--------|-------------------|
| 2026-06-03 | Escopo do grafo como parâmetro dos dois pipelines via helper único na fiação; flag `--filtrar-stdlib` (default Completo); saída (JSON e texto) declara `escopo` nos dois modos. Conserta o Achado 2 do laudo 0029 (classificação divergente em silêncio → diferença rotulada). Default do ranking muda de filtrado para Completo (declarado, ancorado por testes). `lente_filtro` intacto. Invariante confirmado: para um nó do código do usuário, "diretos" e "transitivos" são iguais entre escopos (apenas `uses_saida` e classificação variam). 152 verdes + 16 ignored; pureza do L1 mantida; dois subprocessos do cargo (0023). | `04_wiring/src/lib.rs`, `02_shell/catalogo/src/lib.rs`, `02_shell/cli/src/{args,saida,main}.rs`, `00_nucleo/lessons/0030-escopo-usuario.md` |
