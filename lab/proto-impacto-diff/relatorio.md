# proto-impacto-diff — relatório de Arena

**Data**: 2026-06-05
**Prompt**: `00_nucleo/prompt/0038-proto-impacto-diff.md`
**Tipo**: Arena visual descartável; mede a ideia da trilha local antes
de comprometer o produto.

---

## O que é

Pipeline em Rust + UI HTML/JS que faz: **extrair grafo de um crate → ler
`git diff` (stdin OU `git diff HEAD`) → mapear hunks→nós tocados via
`No.position` (laudo 0037) → calcular raio por nó → emitir JSON →
desenhar em camadas**.

A vista em camadas usa `<details>`/`<summary>` aninhados: um nível para
o arquivo, outro nível para cada nó tocado (com sua cadeia de
contenção, classificação, raio, e amostra do montante).

---

## Como rodar

Pipeline:

```bash
cd lab/proto-impacto-diff

# Modo "ambos" (compara stdin e git):
git -C ../../ diff HEAD | cargo run --release -- \
    --crate lente_core \
    --crate-path "$(cd ../../01_core && pwd)" \
    --repo "$(cd ../../ && pwd)" \
    --input ambos \
    --out dados/impacto-ambos.json
```

UI:

```bash
cd lab/proto-impacto-diff
python3 -m http.server 8080
# abrir http://localhost:8080/
```

O JSON é gerado uma vez; a UI consome estaticamente. Padrão das
Arenas visuais (laudos 0029, 0036).

---

## Material da medição

O experimento rodou contra o **próprio repo da lente**, com edições
não-comitadas reais — os arquivos do laudo 0037 (`Posicao` adicionada,
`No.position` adicionada, ripple em 9 helpers).

| Campo | Valor |
|---|---|
| Crate alvo | `lente_core` |
| `nodes_total` | 119 |
| `nodes_com_position` | 119 (100%) — fork 5ª-rodada confirmado em PATH |
| `colisoes_path` | 4 (caso típico — `ErroRaio::fmt` Display+Debug, etc.) |
| Arquivos no diff (git) | 10 |
| Arquivos no diff (stdin) | 10 |
| Arquivos com nó tocado | 2 (os do `lente_core`: `raio.rs`, `grafo.rs`) |

---

## Respostas às perguntas do prompt

### 1. Comparação `stdin` vs invocar `git diff HEAD` — IGUAIS

`comparacao.iguais = true`. O conjunto de caminhos no diff é
**idêntico** pelos dois caminhos:

```
"comparacao": { "iguais": true, "so_em_stdin": [], "so_em_git": [] }
```

**Mas** uma diferença que importa, descoberta pela exploração:

#### Untracked (arquivo novo) NÃO entra em nenhum dos dois caminhos

- `git diff HEAD` (o que o binário executa) **ignora** untracked.
- `git diff` puro (que um usuário poderia pipear no stdin) **também
  ignora** untracked.

Para um arquivo novo aparecer no diff, o usuário precisaria:
`git add --intent-to-add <arquivo>` (ou já ter commitado).

**Implicação para o produto**: se a próxima iteração do modo `--diff`
quiser "qualquer mudança não-revisada", precisará tratar untracked
**explicitamente** (via `git ls-files --others --exclude-standard` e
sintetizar hunks `+1,N` para todo o arquivo). Não é grátis — é decisão
de produto.

#### `git diff HEAD` cobre encenado **e** não-encenado, juntos

Comportamento desejável para "o que mudei desde o último commit", que
é provavelmente o caso comum. Para "só o que está encenado", seria
`git diff --cached`; para "só o não-encenado", `git diff`. Convergem
nas mesmas faixas quando não há nada encenado (caso atual do repo).

#### Recomendação para o produto

**Invocar `git` é mais cômodo** (o usuário não precisa lembrar de
pipar; o produto sabe o que rodar). **stdin é mais flexível** (aceita
diff de PR, de branch, de qualquer fonte). O produto poderia oferecer
ambos: default `git diff HEAD`, opção `--diff-stdin` para colar.

### 2. Relativização do caminho — sem mistério no monorepo

`position.file` chega do fork **absoluto**:

```
/home/dikluwe/Documentos/Antigravity/tekt-cargo-dsm/01_core/src/entities/grafo.rs
```

`git diff` usa caminhos **relativos à raiz do repo**:

```
01_core/src/entities/grafo.rs
```

O método aqui foi: descobrir a raiz do repo via `git rev-parse
--show-toplevel`, depois `strip_prefix(raiz_repo)` no `position.file`.
**Casou limpo no monorepo da lente** — zero descasamentos.

**Bug latente possível** (não exercitado neste experimento): se o
crate alvo viver fora do repo (workspace externo), ou se o fork
emitir paths com `..` ou symlinks resolvidos, o `strip_prefix` falha
silenciosamente e o `ends_with(&caminho_do_diff)` do fallback pode
casar nó errado. Para o produto, vale testar com crates fora do repo
e registrar.

### 3. As camadas — leem bem em ~10s

Vista por camadas:

```
01_core/src/entities/grafo.rs (3 nós tocados)
  + lente_core › entities › grafo                Folha   0 diretos · 0 transitivos    L1–588
  + lente_core › entities › grafo › Posicao      Intermediário   2 · 15                L218–234
  + lente_core › entities › grafo › No           Intermediário   2 · 11                L236–272
```

- O **arquivo** é a camada de cima (clique para abrir).
- Cada **nó tocado** é uma sub-linha colapsada. Aprofundar mostra
  path completo e amostra do montante.
- O nó-pai (`...::grafo`, o módulo todo) tem `Folha 0/0` porque ninguém
  depende do **módulo como tal** no grafo de itens — toda dep aponta
  para tipos/itens internos. Faz sentido estruturalmente; mostra
  honestamente o que o grafo do fork oferece.
- Os nós-filhos (`Posicao`, `No`) têm raio real (`Intermediário 11/15`).
  Aprofundar uma camada acrescenta detalhe **útil**, não ruído — pelo
  menos neste tamanho de crate (119 nós).

**Teste dos ~10s**: olhando o arquivo `grafo.rs`, vejo "3 nós tocados,
`No` com 11 transitivos". Conclusão arquitetural rápida: "mexer no
`No` toca 11 itens". Confirma o teste do prompt.

### 4. O mapeamento em si — acertos

Edição conhecida: adicionei `pub struct Posicao { ... }` em `grafo.rs`
(linhas 218–234). O protótipo achou:

- Nó: `lente_core::entities::grafo::Posicao` (kind=Struct, L218–234).
- Cadeia: `lente_core › entities › grafo › Posicao`.
- Raio: Intermediário, 2 diretos, 15 transitivos.

Casou **exato** — começo e fim do span batem a faixa do diff. ✓

Outras edições (adicionei `position: None,` em 9 helpers de teste): o
binário **não** as marca, **porque os helpers de teste em `#[cfg(test)]`
não aparecem no grafo do fork** (o fork analisa só código de release;
testes inline ficam de fora). Isso é coerente com a natureza estrutural
da lente — testes não fazem parte do "raio" do crate consumidor.

### 5. Bordas

| Caso | Comportamento |
|------|---------------|
| Edição **dentro** de uma função | Achei a função (nó mais interno). ✓ |
| Edição **entre itens** (linhas em branco do módulo) | Cai sob o nó-módulo (que cobre L1–N do arquivo). A cadeia mostra só o módulo. ✓ |
| Edição cruza **vários** itens | Ambos os itens são marcados (sem dedup). ✓ |
| Função **nova** (linhas adicionadas) | Não aparece como nó próprio (o fork ainda não a viu — extraio o grafo **antes** de aplicar o diff). **Importante**: o protótipo extrai o grafo da árvore de trabalho **atual** (que já tem a função nova), então ela está no grafo. ✓ Se o produto extrair do `HEAD` e diferir contra o working tree, isso muda. |
| Macro call-site (laudo 0037) | Não exercitado; espera-se que macro-gerados peguem o call-site, conforme o briefing. |
| Arquivo **novo** (untracked) | NÃO aparece (vê seção 1). |
| Arquivo **deletado** | Linha `+++ /dev/null` no diff → parser ignora; nada marcado (correto). ✓ |
| Arquivo **renomeado** | Não exercitado. O parser usa o `+++ b/<novo>`, então deve casar o nó na posição nova. |

### 6. Honestidade visual

A faixa amarela no topo da página diz literalmente:

> **Honestidade.** Esta vista mostra impacto **estrutural** (quem
> depende dos nós tocados via arestas `Uses` no grafo), **não
> comportamental** — não diz o que quebra em runtime; diz quem está
> no raio.

Atende o critério da proposta. Para um leitor distraído, "Intermediário
11 transitivos" pode parecer "11 coisas vão quebrar". O rótulo
mitiga, mas a UI não impede confusão — só sinaliza. Para o produto,
talvez valha uma palavra na própria linha do nó ("impacto estrutural:
11 nós no raio") em vez de só no cabeçalho.

### 7. Colisões de path no `lente_core`

`colisoes_path = 4`. Esperado (`ErroRaio::fmt` Display+Debug,
`Classificacao::fmt`, etc.) — laudo 0006/0031. O protótipo usa o grafo
**cru** (sem `lente_resolve`); o raio dos 4 paths colididos pode estar
impreciso. Para o experimento, é tolerável (3 nós tocados, nenhum
deles é dos 4 colididos: `Posicao`, `No`, módulo `grafo`). Para um
produto, resolver é refinamento de próxima rodada.

---

## Decisões da Arena

### D1 — Rust gera JSON, HTML lê

Espelha `lab/proto-dsm`/`lab/proto-ui`. Alternativa: `eframe`/`egui` no
mesmo binário. Razões para HTML:

- Compilação **rápida** (sem `eframe`). ~5s.
- UI estática trivial via `<details>`. Quase nada de JS.
- Padrão consistente com as Arenas visuais existentes.

O prompt sugeria `eframe`; a substituição por HTML é coerente com
"qualidade de protótipo" e mantém o foco no pipeline.

### D2 — Sem dedup entre arquivos da mesma faixa

Se um diff toca o mesmo nó por duas faixas no mesmo arquivo, o
protótipo dedup'a por `id` (usando `BTreeSet`). Mas se duas faixas em
arquivos **diferentes** atingem o mesmo módulo conceitual (raro), são
duas entradas. Aceito como simplicidade da Arena.

### D3 — `BTreeSet`/`BTreeMap` para determinismo

Ordens determinísticas por construção. `HashMap` no `Raio.montante`
fica fora do controle aqui (vem do `lente_core`), mas o protótipo
ordena alfabeticamente os 10 da amostra.

### D4 — `position.file` casado por `==` ou `ends_with` fallback

O fluxo principal usa o mapa `nos_por_arquivo[relativizado]`. Se um nó
tem `position.file` que **não** começa com a raiz do repo (cenário
estranho: rustc src cache, deps), ele cai no fallback `ends_with` —
casa pelo final do path. Robusto o suficiente para o experimento; no
produto, valeria registrar o número de fallbacks usados.

---

## Pendências cobertas / abertas

| Item | Estado |
|------|--------|
| Pipeline diff→nós com cadeia de Owns e raio por camada | **Coberto** — protótipo |
| Comparação stdin vs git | **Coberta** — iguais para tracked; untracked é cego nos dois |
| Vista visual em camadas | **Coberta** — `<details>` aninhados |
| Honestidade visual (estrutural ≠ comportamental) | **Coberta** — nota amarela no topo |
| Resolução de colisões antes do raio | **Aberta** — Arena usa grafo cru; 4 colisões registradas |
| Untracked tratado no produto | **Aberto** — decisão de produto, não da Arena |
| Bordas: macro call-site, rename, crate fora do repo | **Não exercitadas** — registradas |
| Modo `--diff` no produto / casca MCP | **Abertos** — próximas pontes da trilha local |

---

## Arquivos

```
lab/proto-impacto-diff/
├── Cargo.toml             # bin Rust; deps lente_core + lente_infra
├── src/main.rs            # pipeline (CLI args + parser + mapeamento + JSON)
├── index.html             # vista em camadas (DOM + <details>; sem CDN)
├── dados/
│   ├── impacto-ambos.json   # stdin + git lado a lado (~19 KB)
│   ├── impacto-git.json     # só git diff HEAD (~9 KB)
│   └── impacto-stdin.json   # só stdin (~11 KB)
└── relatorio.md           # este arquivo
```

---

## Convenção de aposentadoria

Padrão da Arena: a tela de produção (se nascer) vive em outro lugar;
este protótipo fica como registro do experimento. Se a próxima
iteração der à luz o modo `--diff` na CLI ou uma tela DSM enriquecida,
atualizar o laudo 0038 indicando qual componente nasceu; manter este
protótipo intocado.

---

# Segunda rodada — laudo 0039 (multi-crate)

**Data**: 2026-06-05
**Prompt**: `00_nucleo/prompt/0039-proto-impacto-diff-multicrate.md`

A primeira rodada (0038) provou a vista em camadas sobre **um** crate.
Esta segunda rodada estende para **multi-crate**: o mesmo binário agora
descobre todos os crates do workspace via `cargo metadata`, mapeia
arquivo→crate, extrai cada crate tocado (e o restante para a união),
e exibe o impacto **atravessando crates** (abordagem B do prompt:
união por **path**, porque os `id`s do petgraph são instáveis entre
extrações).

## A pergunta central: o impacto cruza crates?

Mediu, e a resposta é **alto** para tipos públicos do `lente_core`:

| Nó tocado | Raio local (no crate) | Raio workspace (união) | Δ cross-crate |
|---|---:|---:|---:|
| `lente_core::entities::grafo::Posicao` | 15 | **48** | **+33** |
| `lente_core::entities::grafo::No` | 11 | **44** | **+33** |
| `lente_infra::dto::NoDTO` | 7 | 7 | +0 |
| `lente_infra::dto::PositionDTO` | 10 | 10 | +0 |

Leitura: mexer em `No` ou `Posicao` no `lente_core` reverbera em **3×
mais nós** quando se conta o resto do workspace. Os DTOs do
`lente_infra` ficam contidos (são internos, ninguém de fora usa).

Sem este multi-crate, o protótipo do 0038 reportaria `transitivos=11`
para `No` — escondendo 33 dependentes reais. A trilha local **precisa**
de visão de workspace para responder honestamente.

## Confirmações do prompt 0039

### 1. O 0038 perdeu `lente_infra` — agora não mais

O diff atual toca `01_core/src/entities/grafo.rs` (Posicao, No),
`03_infra/src/dto.rs` (PositionDTO no DTO), `03_infra/src/traducao.rs`
(propagação), e helpers em `05_investiga`, `06_resolve`, `07_filtro`,
`08_ranking`, `09_estrutura`. O 0038, limitado a `lente_core`, mapeava
só os 3 nós em `grafo.rs`. O multi-crate agora encontra:

```
[lente_core]       4 nós tocados
[lente_estrutura]  1 nó (módulo)
[lente_filtro]     1 nó (módulo)
[lente_infra]      7 nós tocados (dto.rs e traducao.rs)
[lente_investiga]  2 nós (módulos)
[lente_ranking]    1 nó (módulo)
[lente_resolve]    1 nó (módulo)
```

Os helpers de teste (`mod tests`) caem sob o nó-módulo do crate
porque `cfg(test)` itens não entram no grafo de release — coerente.
O 0038 não erra; o `mod tests` é coberto pelo módulo dono.

### 2. Comparação stdin vs git — IGUAIS no multi-crate também

`comparacao.iguais = true`, `so_em_stdin = []`, `so_em_git = []`. O
resultado do 0038 vale aqui sem mudança.

### 3. Abordagem A vs B

Medi A (extrair `lente_wiring`, que depende de todos os L1) **antes**
de implementar B:

- `lente_wiring` extraído: 68 nós, dos quais 37 do próprio wiring e
  31 dos outros crates (`lente_core` 7, `lente_estrutura` 5,
  `lente_investiga` 3, etc.). Surpresa: esses 31 **vêm com `position`
  preenchida** (o `cargo modules` lê o source via `cargo metadata`,
  não emite "nó leve"). 21 arestas `lente_wiring → lente_core`.
- **MAS** o conjunto de `lente_core` no grafo do wiring é **subset**
  do `lente_core` real — só os itens que o wiring usa. Para "todos
  os dependentes de `No`", isso é insuficiente.

Abordagem B (união de todas as extrações por path):

| Métrica | Valor |
|---|---|
| Crates extraídos | 10 (workspace inteiro) |
| Tempo por crate | ~3.1–3.6 s |
| Tempo total | ~33 s |
| Nós únicos por path | 351 |
| Arestas | 1 148 |
| **Arestas soltas após união** | **0** |

Casamento por path **funcionou perfeitamente** no monorepo da lente.
Zero soltas — todo `to`/`from` de aresta cross-crate referencia path
que existe em alguma extração. Como o `id` muda entre extrações, o
casamento por path é a única opção viável.

**Para o produto**: B é o caminho. A só economizaria tempo se o
"grafo de workspace" pudesse ser obtido com **uma** extração; o
cargo modules **não** dá isso (extrai por pacote). Logo a única
otimização é **cache** das extrações por commit-hash, fora do
escopo deste prompt.

### 4. Sysroot e `position` — a dúvida do 0038 fechada

O `lente_infra::extrair_grafo` força `--sysroot` (laudo 0023, política
da lente). Mesmo assim, **100% dos nós em todos os crates extraídos
têm `position`** — incluindo os de stdlib (`core::*`, `alloc::*`,
`std::*`). Explicação verificada: o fork lê o source da rustc via
`cargo metadata` e atribui `position` aos itens da stdlib apontando
para os fontes em `~/.rustup/toolchains/.../lib/rustlib/src/rust/...`.

Pratica: o `position.file` desses nós **não existe no repo da lente**,
então a `relativizar` devolve `None`, e o `ends_with` do fallback não
casa (porque um diff do repo nunca toca `clone.rs` da stdlib). Logo
nós de stdlib **não geram falso-positivo** na vista. Bom.

### 5. Macro / `#[derive(...)]` — surpresa de honestidade

A `Posicao` (struct nova) deriva `Debug, Clone, PartialEq, Eq`. O
fork gera nós para os métodos derivados:

| Nó | `position.file` (resumido) | linhas |
|---|---|---:|
| `Posicao` (struct) | `grafo.rs` | 218–234 |
| `Posicao::clone` (gerado por `#[derive(Clone)]`) | `.../rustlib/.../clone.rs` | 195–236 |
| `Posicao::eq` (gerado por `#[derive(PartialEq)]`) | `.../rustlib/.../cmp.rs` | 252–256 |
| `Posicao::fmt` (gerado por `#[derive(Debug)]`) | `.../rustlib/.../fmt/mod.rs` | 874–904 |

O fork aponta para a **definição original do trait na stdlib**, NÃO
para a linha do `#[derive(...)]` no meu código.

Implicações:

- **Honestidade**: o protótipo **não** reporta falso-positivo. Adicionar
  uma struct nova com `#[derive(Clone)]` não marca `Clone::clone` no
  diff como "tocado", porque a `position` de `Clone::clone` aponta
  para a stdlib.
- **Subreporte deliberado**: se o usuário edita o `#[derive(...)]`
  (ex.: remove `Clone`), o protótipo marca a **struct** (que cobre
  a linha do derive no diff) mas **não** os métodos derivados — porque
  estão "fora" do arquivo. Os usuários de `Posicao::clone` ficam
  invisíveis.
- **Diferença do call-site previsto pelo briefing §5**: o briefing
  dizia "macro call-site para itens gerados". Para `#[derive(...)]`,
  o fork não faz isso — aponta para a definição do trait. É um detalhe
  do fork; uma macro **proc-macro própria** do projeto poderia se
  comportar diferente (não exercitado — a lente não tem proc-macros
  próprias).

Registrado como achado importante para o produto: a vista é
**conservadora** quanto a derive (não inventa marca), e o usuário
precisa saber disso para interpretar bem.

### 6. Camadas em escala

10 crates × média de 50 nós tocáveis. A UI agrupa em três níveis:

```
crate (chip dourado)
  └─ arquivo
       └─ nó tocado
            └─ amostra do montante
```

Cada nível colapsa por default exceto onde tem 1+ nó tocado (abre
automático). Para o diff atual (7 crates tocados, mais ~17 nós no
total), a vista cabe na primeira tela sem rolagem. Para diffs muito
maiores (refactor de 30+ arquivos), provavelmente vira ruído —
seria preciso filtrar (ex.: `--so-com-delta-cross-crate`) ou
ordenar por impacto.

## Decisões adicionais da segunda rodada

### D5 — União sempre sobre o workspace inteiro

Default: extrair todos os 10 crates, mesmo que só 2 estejam tocados.
Custo: ~33s. Benefício: o `raio_workspace` reflete o cross-crate
**completo**, não só os crates tocados. Opção `--so-tocados` reduz
extração ao subconjunto tocado, sacrificando a parte do raio que
viria de crates não-tocados.

### D6 — Casar arestas por path, não por id

Implementado em `unir_grafos`: ids reatribuídos sequencialmente a
partir do mapa `path → id_global`. Cada aresta usa o `path` como
chave (já está em `from`/`to`); ids antigos são descartados. Zero
arestas soltas no experimento confirma a viabilidade do casamento
por path no monorepo da lente.

### D7 — Detectar e relatar nós-leves (referências)

Helper `nos_leves` no Rust: conta nós sem `position` + sem campos
de descritor (uma marca dos nós-referência do `cargo modules`).
Resultado: **0 leves em todos os crates**. O fork emite nós completos
para deps no workspace — descobre via `cargo metadata` que o source
está disponível. Para deps externas (`clap`, `serde`), provavelmente
viriam leves; não exercitado.

## Tempo total e custo no produto

| Fase | Custo |
|---|---|
| `cargo metadata --no-deps` | <100 ms |
| 10× extração de fork | ~33 s (cada uma cold-start do rust-analyzer) |
| União por path | <100 ms |
| Mapeamento diff→nós | <100 ms |
| Cálculo de raio (~17 nós tocados) | <100 ms |
| **Total para um diff típico** | **~33 s** |

Para um produto na CI ou agente reativo, **inviável sem cache**. O
caminho previsto: cachear o grafo por crate por commit-hash; recompor
a união só do que mudou. Cache: outro prompt.

Para uso **manual de desenvolvedor** ("o que minha mudança toca?"),
~33s é tolerável — ainda mais barato que esperar CI rodar.

## Tabela final de respostas (perguntas do prompt 0039)

| Pergunta | Resposta |
|---|---|
| Mapeamento multi-crate casa nós certos? | **Sim** — 7 crates tocados pelo diff, todos com seus nós identificados. O `lente_infra` que o 0038 perdeu agora aparece com 7 nós. |
| Impacto cross-crate aparece? | **Sim, e é grande** — `No` 11→44 (Δ+33); `Posicao` 15→48 (Δ+33) ao incluir o workspace. |
| Abordagem A ou B? | **B** (união por path). A traz subset insuficiente. |
| Arestas soltas após B? | **Zero** no monorepo da lente. Casamento por path é robusto. |
| Sysroot ligado, e nós sem `position`? | Sysroot ligado (política do produto); **todos** os nós têm `position`, inclusive stdlib (fork lê source da rustc). Não gera falso-positivo porque a `relativizar` filtra fora-do-repo. |
| Macro/derive comportamento? | **Conservador**: derive aponta para stdlib, não para o `#[derive]` no código. Não inventa marca; subreporta quando usuário mexe no derive. |
| Vista em camadas em escala? | Boa para o diff atual (7 crates, ~17 nós). Diffs grandes pedem filtro. |

## Estado pós-0039

- O protótipo do 0038 foi **substituído** pelo multi-crate no mesmo
  diretório. Os dumps antigos (`impacto-ambos.json` etc.) ficam para
  comparação histórica via seletor "dump" da UI.
- Novo dump principal: `impacto-multi-ambos.json` (60 KB).
- Suíte de produção intacta (213 verdes + 22 ignored).

**Próximos passos sugeridos** (não decididos aqui):

1. Modo `--diff` na CLI com **cache de extrações** (para o tempo
   ficar abaixo de 1s na 2ª invocação).
2. Casca MCP (Ponte 2 da trilha local).
3. Tratamento de untracked (achado do 0038, ainda em aberto).
4. Filtro/ordenação para diffs grandes (Achado 6).

---

# Terceira rodada — laudo 0040 (cache + incremental)

**Data**: 2026-06-05
**Prompt**: `00_nucleo/prompt/0040-proto-impacto-diff-cache.md`

A segunda rodada (0039) provou multi-crate, mas extrair 10 crates
custou ~33 s — proibitivo para uso reativo. Esta terceira rodada
adiciona **cache do JSON cru por crate** (chave = SHA-256 dos
fontes) e **extração incremental** (re-extrai só o que mudou).

## Tabela de tempos (a resposta principal)

Sobre o monorepo da lente, 10 crates-membros, em todos os cenários
o diff é o atual (laudos 0037 + alteração de comentário). Tempo
total do binário (`t_main`):

| Cenário | Extraídas | Reusadas | Fork (s) | Cache I/O | desser+união+mapa | **Total** |
|---|---:|---:|---:|---:|---:|---:|
| **Cold** (cache vazio) | 10 | 0 | 31.68 | 0 | 9 ms | **31.76 s** |
| **Morno-1** (1 crate invalidado) | 1 | 9 | 2.95 | <1 ms | 9 ms | **3.02 s** |
| **Morno-3** (3 crates invalidados) | 3 | 7 | 9.69 | <1 ms | 9 ms | **9.77 s** |
| **Cache quente** (nada mudou) | 0 | 10 | 0.00 | <1 ms | 9 ms | **0.07 s** |
| **Renomeação** (cache stale, sem mudança no fonte) | 0 | 10 | 0.00 | <1 ms | 9 ms | **0.07 s** + 1 fantasma |

Em todos os cenários: `cargo metadata --no-deps` ≈ 55 ms;
`desserializar+unir+mapear` < 10 ms para 10 crates × ~30 nós cada.

### Veredito de viabilidade reativa

| Frequência típica do cenário | Tempo |
|---|---|
| Cache quente (consulta repetida, sem edição) | **70 ms** ✓ instantâneo |
| Morno-1 (edição em 1 crate — caso mais comum) | **~3 s** ✓ interativo |
| Morno-3 (refactor em 2-3 crates) | **~10 s** ⚠ tolerável |
| Cold (primeira execução) | **~32 s** ✗ inevitável, 1 vez por sessão |

**A trilha local é viável no uso reativo.** O caminho típico
(edição em 1 crate → consulta) está abaixo dos ~3 s — equivalente
ao tempo de uma rodada de `cargo check`. O cold-start (~32 s) é
**inevitável**: precisa rodar o fork em cada crate uma vez para
popular o cache. Pago uma vez por sessão.

A parte sem-fork (desserializar + unir + mapear) é **< 10 ms** para
o workspace inteiro. Nem chega perto de virar gargalo — a otimização
de "cachear a união já montada" do prompt §3 **não se justifica**.

## Chave de cache: SHA-256 dos fontes

Implementação em `coletar_fontes` + `hash_fontes`:
- Lista recursiva de `.rs` sob `<crate>/src/`, em ordem
  alfabética determinística.
- Para cada arquivo: hash do `path-relativo + 0x00 + len(content) + 0x00
  + content + 0x00`.
- SHA-256 (não `DefaultHasher` da stdlib — esse muda seed entre
  execuções; cache precisa ser estável entre rodadas).

**Pega edições não-comitadas?** **Sim.** Editei um comentário em
`01_core/src/entities/grafo.rs`, rodei — o hash de `lente_core`
mudou e a re-extração disparou. Coerente com o requisito do prompt
(uso reativo).

**Não invalida em excesso?** Não pega mudanças em `Cargo.toml`
(deps externas mudam → comportamento do fork pode mudar). Caso de
borda baixo no monorepo da lente; registrar como limitação para o
produto.

## Cenário de renomeação: fantasmas como sinal

O prompt §4 antecipou que renomeação geraria **arestas soltas** (o
cache de B aponta para `A::No`, A re-extraído não tem mais `No`).

**Achado contra-intuitivo**: no monorepo da lente, a renomeação
**NÃO produz arestas soltas**. Razão: o `cargo modules` extrai cada
crate independentemente, e cada extração **inclui nós-referência dos
crates dependentes** (com `position` própria do `cargo metadata`).
Logo o cache de `lente_infra` tem o próprio nó `lente_core::No` em
sua lista (não apenas a aresta para ele). Após eu editar só o cache
de `lente_core` removendo `No`, o **nó** `No` continua vivo na união
(vindo dos caches dos dependentes), e as arestas casam.

**Mas o sinal está lá — em outra forma**: implementei a detecção de
**nós fantasma** = "path cujo primeiro segmento é um crate do
workspace, mas que **não** está entre as origens (crates cujos
caches o produziram)".

Para a renomeação simulada (`lente_core::entities::grafo::No` →
`NoRenomeado`, só no cache do `lente_core`):

```
↳ fantasmas (sinal de cache stale / renomeação):
    lente_core::entities::grafo::No
      esperado em: lente_core
      vem de:      [lente_estrutura, lente_infra, lente_investiga, lente_resolve]
```

**A lista de "vem de" é EXATAMENTE a lista de crates impactados pela
renomeação.** É o sinal certo de impacto.

Para o produto: a vista do `--diff` pode usar esta detecção para
**alertar**: "este path sumiu do crate dono; os crates X, Y, Z
referenciam — provável que quebrem".

## Robustez da chave de cache

Testes (manuais) feitos:

| Mudança | Cache invalida? |
|---|---|
| Editar conteúdo de `.rs` em `<crate>/src/` | **Sim** ✓ |
| Não tocar nada | Não ✓ |
| Reverter para o conteúdo idêntico | Não ✓ (mesmo hash) |
| Adicionar `.rs` novo em `src/` | Sim ✓ (entra na lista, muda hash) |
| Renomear arquivo em `src/` | Sim ✓ (path relativo entra no hash) |
| Editar `Cargo.toml` (mudar dep ou feature) | **Não** ⚠ — limitação |
| Mudar versão do `rustc` / fork | Não ⚠ — limitação |

Para um produto sério: a chave precisaria incluir
`Cargo.toml`/`Cargo.lock` (e idealmente versão do fork). Fora do
escopo desta Arena.

## Decisões adicionais da terceira rodada

### D8 — Cache do JSON cru, não do `Grafo`

`lente_core::Grafo` é puro (sem `serde`). Cachear o JSON cru evita
mexer no produto (regra da Arena). O custo extra: desserializar a
cada execução. Medido: ~1 ms por crate × 10 crates = ~9 ms total.
**Insignificante.**

### D9 — `--invalidar <c1,c2,...>` para cronometrar morno-N sem editar repo

A Arena não pode "editar o repo para invalidar 3 crates". A flag
`--invalidar` apaga manualmente os arquivos `.hash` dos crates
listados, forçando re-extração no próximo run — **simula** o efeito
de edição naqueles crates, com tempo equivalente.

### D10 — Detecção de fantasmas é o sinal real de renomeação

O prompt §4 esperava arestas soltas; o achado real é mais sutil
(nós-referência mantêm o nó "vivo"). A detecção implementada usa o
critério "primeiro segmento de path bate crate do workspace, mas
origens não incluem o crate dono". É o sinal certo, e a lista de
origens nomeia os afetados.

### D11 — Cache vai em `lab/proto-impacto-diff/cache/` (default)

Não em `~/.cache` ou `target/`. Razão: Arena, autocontido. Para
o produto, viraria um `~/.cache/lente/<repo-hash>/`.

## Tabela final de respostas (perguntas do prompt 0040)

| Pergunta | Resposta |
|---|---|
| O caminho morno é rápido o bastante? | **Sim**. Morno-1 ~3 s (≈ uma extração); morno-3 ~10 s; quente 70 ms. |
| A parte sem-fork é sub-segundo? | **Sim**, ~9 ms. Não vira gargalo. Cachear a união não se justifica. |
| Cold é aceitável? | **~32 s, inevitável, 1 vez por sessão.** Sim para uso humano; precisa pré-aquecimento para CI/agente reativo. |
| Chave de cache (SHA-256 dos fontes) é robusta? | **Pega edições não-comitadas. Não pega Cargo.toml** — limitação registrada. |
| Renomeação produz arestas soltas? | **Não** (no monorepo da lente, caches stale dos dependentes carregam nós-referência). O **sinal correto** é "nó fantasma": o path existe na união mas não está nas origens do crate-dono. **A lista de origens nomeia os crates afetados.** |
| Veredito: incremental + cache torna o uso reativo viável? | **Sim**. Cold 1×/sessão; depois ~3 s por edição de 1 crate, ~70 ms se nada mudou. |
| Custo residual? | Cold (~32 s) e mudança de `Cargo.toml` (não detectada pelo hash). |

---

# Quarta rodada — laudo 0041 (colisões na união)

**Data**: 2026-06-05
**Prompt**: `00_nucleo/prompt/0041-proto-impacto-diff-colisoes.md`

A união por path do 0039/0040 **funde** silenciosamente colisões
intra-crate (4 no `lente_core`, 10 no workspace). Esta rodada **aplica
a cascata da lente** (`lente_investiga` E1 + `lente_resolve` ADR-0006)
em cada crate **antes** de unir, e mede: censo, cobertura, custo,
correção do raio, e órfãos cross-crate.

## Censo de colisões no workspace

| Crate | Colisões | Vereditos (E1) |
|---|---:|---|
| `lente_core` | 4 | 4× `Distintos` (Display+Debug em `fmt`; um `Path::from`) |
| `lente_infra` | 3 | 3× `Distintos` (Display+Debug em 3 `ErroXxx::fmt`) |
| `lente_wiring` | 2 | 2× `Distintos` (`ErroLente::fmt`; `ErroLente::from`) |
| `lente_resolve` | 1 | 1× `Distintos` (`ErroResolve::fmt`) |
| **Total** | **10** | **10× `Distintos` · 0× `MesmoItem` · 0× `NaoDeterminado`** |

**E1 (vizinhança) resolve 100% das colisões do monorepo da lente.**
Zero `NaoDeterminado` — coerente com o achado do laudo 0021
(typst+egui: zero `NaoDeterminado` também). Confirma que, em código
real bem-organizado, a vizinhança no grafo basta.

**E2 (em quarentena) seria necessária? Não.** Nenhuma colisão do
monorepo da lente exigiria a Estratégia 2 (parser de fontes). A E2
permanece justificadamente fora do caminho.

### Padrão dominante: `Display + Debug` em `impl fmt::*::fmt`

8 das 10 colisões são o caso clássico: dois `impl` (Display e Debug)
no mesmo `Tipo`, cada um declarando `fn fmt(...)`. A regra ADR-0006
do `lente_resolve` distingue lindo: `T::<Display>::fmt` vs
`T::<Debug>::fmt`.

### Achado novo da Arena — `<Trait>::metodo` insuficiente para impls genéricos

**2 das 10** colisões NÃO resolvem limpo:

- `lente_core::entities::grafo::Path::from` — 2 cópias, ambas com
  `trait = "From"`. `lente_resolve` produz `Path::<From>::from` ×2 —
  paths novos **colidem entre si**. `trait_ref` real (`From<&str>`,
  `From<String>`) distinguiria; a regra ADR-0006 não o usa.
- `lente_wiring::ErroLente::from` — 4 cópias, todas `From`.
  `lente_resolve` produz `ErroLente::<From>::from` ×4 — colisão
  remanescente. `trait_ref` real:
  `From<ErroFork>`, `From<ErroAdaptador>`, `From<ErroResolve>`,
  `From<ErroRaio>` — únicos.

**Detectei isto na Arena como `DistintosPosRegraColide`** (campo
`distintos_mas_colidem_pos_regra` no censo). Não toca a regra; marca
para a UI como **raio impreciso**.

**Recomendação para o produto**: a regra ADR-0006 deveria usar
`trait_ref` (não `trait`) quando todas as cópias compartilham o
mesmo `trait` mas têm `trait_ref` distinto. O fork já emite o
campo (laudo 0012), só não está sendo lido aqui.

## União limpa: zero fusão indevida (das resolvidas)

Sem resolução: união tem **351 nós** (workspace, cache quente).
Com resolução: união tem **359 nós** (+8) — uma entrada por cópia
distinta das 8 colisões limpas. Os 2 `<From>::from` colididos
continuam fundidos, mas **marcados** como imprecisos.

Total de arestas: 1148 cru-fundidas → **1188 resolvidas** (+40 —
`id_from`/`id_to` agora apontam para nós distintos em vez de fundir
no blob).

## Antes/depois num path colidido tocado

**Esperado pelo prompt §4**: tocar um path colidido e mostrar o raio
errado (cru-fundido) vs o(s) raio(s) correto(s) (cópias distintas).

**Realidade do monorepo da lente**: as 10 colisões são TODAS em
métodos `fmt` ou `from` — **folhas comportamentais** (laudo 0021,
~18.5% dos nós). O fork `cargo-modules` não captura chamadas via
`format!`/`println!` (macro) nem via `?`/`From::from` — então `fmt` e
`from` aparecem como folhas com raio 0.

| Path colidido tocado | Cru-fundido | Resolvidos | Δ |
|---|:---:|:---:|:---:|
| `lente_core::…::ErroRaio::fmt` (2 cópias) | raio=0 (Folha) | 2× raio=0 (Folha) | 0 |
| `lente_wiring::ErroLente::from` (4 cópias) | raio=0 (Folha) | 4× raio=0 (Folha) | 0 |

**Conclusão honesta**: no monorepo da lente, **o efeito da resolução
no número do raio é zero para o diff atual** — porque todas as
colisões são folhas. O efeito é em **honestidade da contagem de
cópias** (4 entradas para `ErroLente::from`, não 1) e em
**correção futura** (quando houver um tipo colidido **estrutural**,
tocá-lo dará raio correto).

**Limite herdado, não-novo**: o fork não vê chamadas de macro/From —
laudos 0021 e 0038 já registraram. Esta rodada apenas confirma que
o efeito coincide com folhas-comportamentais para o monorepo.

Para o produto: o caso real do delta apareceria quando um tipo
colidido for um **struct/enum não-comportamental** referenciado por
outros nós — não exercitado no diff atual.

## Custo da resolução: desprezível

Hipótese §5: "desprezível, pois é L1 puro sem fork novo." Confirmada.

| Cenário | Total | Fork | **Resolução** | % do total |
|---|---:|---:|---:|---:|
| Cold (resolver) | 31.46 s | 31.38 s | **0.76 ms** | 0.0024% |
| Cold (sem-resolução) | 31.35 s | 31.27 s | — | — |
| Quente (resolver) | 0.07 s | 0 s | **0.63 ms** | 0.86% |
| Quente (sem-resolução) | 0.08 s | 0 s | — | — |

10 colisões resolvidas em **~0.8 ms** → ~0.08 ms por colisão. Abaixo
da granularidade da medição total em segundos. Pode entrar no
caminho morno sem custo perceptível.

## Órfãos cross-crate da resolução: predição confirmada (0 no monorepo)

**Predição do prompt §7**: raro/nenhum — paths colididos costumam
ser métodos internos de impl que outros crates não referenciam pelo
path colidido.

**Confirmação empírica no monorepo da lente**:

- `lente_core::…::ErroRaio::fmt` (colidido) aparece em cache de
  **só `lente_core`**. Nenhum outro crate o referencia → resolução
  intra-crate de `lente_core` não cria fantasma.
- `lente_wiring::ErroLente::from` (colidido) aparece em cache de
  **só `lente_wiring`**. Idem.
- **Verificado para todas as 10 colisões**: 0 fantasmas após
  resolução. A renomeação intra-crate é invisível aos demais
  crates do workspace.

**Razão estrutural**: as colisões são impls (`fmt`, `from`), chamadas
via macro/`?`. Nenhum outro crate as referencia **por path** — só
chama o tipo dono, deixando o `impl` resolver. Esses impls são, em
prática, "internos" cross-crate.

**Distinção fantasma-de-resolução vs fantasma-de-edição**:
preservada por construção. A simulação `--simular-renomeacao` do
0040 continua produzindo seu fantasma certo (`lente_core::…::No`
com origens `[lente_estrutura, lente_infra, lente_investiga,
lente_resolve]`). A resolução adiciona 0 fantasmas. As duas
detecções convivem sem ruído.

## Decisões da quarta rodada

### D12 — Replicar laço do wiring na Arena, não importar L4

`detectar_colisoes_grafo`, `construir_vizinhanca`, `resolver_grafo`
são funções da Arena, copiadas em estrutura do
`lente_wiring::obter_grafo_resolvido` (laudo 0019). Razão: o wiring
**falha** em `NaoDeterminado`; a Arena precisa **continuar** (manter
blob marcado). Importar e adaptar adicionaria fronteira que não vale
para Arena.

### D13 — `NaoDeterminado` ⇒ blob fundido marcado, não erro

O wiring vira `ColisaoNaoResolvida` em erro. A Arena **registra
diagnóstico** e mantém o blob — o nó tocado fica com
`raio_impreciso = true`, a UI desenha alerta. Coerente com o objetivo
("avisar"). Sem `NaoDeterminado` no monorepo, este caminho ficou
não-exercitado em dados reais; testado por construção (estrutura
existe e marca; sem foto).

### D14 — `DistintosPosRegraColide` é uma 4ª categoria

A regra ADR-0006 pode produzir nomes novos que **continuam
colidindo entre si** (impls genéricos do mesmo trait). Sai de
`Distintos` (que prometeria paths únicos) — fica numa categoria
nova `DistintosPosRegraColide`, somada em `paths_imprecisos`.
**É um achado** sobre a regra; não corrige na Arena.

### D15 — Pré-resolução guarda os grafos crus

Para o `--simular-tocar-colidido`, comparar o raio cru-fundido (na
união ingênua dos grafos crus) com o raio resolvido. O `main` mantém
`grafos_por_crate_cru` antes da etapa 5.5; a função de antes/depois
faz **outra união** sobre os crus quando solicitada. Custo: uma
clonagem de `BTreeMap<String, Grafo>` (poucos KB; trivial).

## Tabela de respostas (perguntas do prompt 0041)

| Pergunta | Resposta |
|---|---|
| Censo: quantas colisões? | **10**: 4 `lente_core`, 3 `lente_infra`, 2 `lente_wiring`, 1 `lente_resolve`. |
| Cobertura E1? | **100% — todas decidem com vizinhança disjunta**. |
| `NaoDeterminado`? | **Zero** no monorepo da lente. |
| Alguma exigiria E2? | **Não.** E2 segue justificadamente em quarentena. |
| `MesmoItem`? | **Zero** — coerente com laudo 0021. |
| União limpa após resolver? | **Sim** para 8 das 10. As 2 que colidem-pós-regra (impls de `From<T>` distintos) precisariam usar `trait_ref` na ADR-0006. |
| Correção do raio (antes/depois)? | **Sem efeito numérico no monorepo da lente** — todas as colisões são folhas comportamentais (`fmt`/`from`). Efeito real é honestidade da contagem (n cópias). |
| Custo da resolução? | **~0.8 ms** para 10 colisões — 0.002% do cold, 0.86% do quente. Desprezível, hipótese confirmada. |
| Órfãos cross-crate da resolução? | **Zero** no monorepo. Razão: as colisões são impls internas (`fmt`/`from`), não referenciadas por path em outros crates. Predição §7 confirmada. |
| Distinção fantasma-de-resolução vs fantasma-de-edição? | **Preservada**. Renomeação de 0040 continua sinalizada (lista de origens); resolução adiciona 0 fantasmas. |
| **Veredito para o produto**: resolver por crate antes de unir? | **Sim, sem reservas.** Cobertura E1 alta; custo desprezível; honestidade ganha (cópias distintas viram entradas separadas); órfãos não-criados. **Mas atualizar a ADR-0006 para usar `trait_ref` quando `trait` colide pós-regra.** |



---

# Rodada 0043 — Untracked (arquivos novos não rastreados)

Extensão do protótipo para o ponto cego dos laudos 0038/0040: `git diff HEAD`
não vê arquivo novo sem `git add`. Detecção via
`git ls-files --others --exclude-standard`; corte **ligado vs solto** pela
interseção com as fontes que o cargo compilou (= `position.file` dos nós do
grafo unido).

## Tabela de respostas (perguntas do prompt 0043)

| Pergunta | Resposta |
|---|---|
| **A. Detecção** | `git ls-files --others --exclude-standard` lista limpo, respeitando `.gitignore` (target/ do lab fora). Hunks "tudo adicionado" sintetizados lendo o arquivo (`Faixa { 1, n_linhas }`). |
| **B. Ligado** | União 363→**366 nós** (módulo + 2 `fn`). Cache **erra → re-extrai** (morno-1, 3.47 s). Hunks mapeiam por `position` (3 nós tocados). **Montante = vazio** nos 3; **jusante** mostra `No, Path, …` (dir=2 trans=8 nas `fn` que usam ambos; trans=2 na que usa só `Path`). `delta_cross_crate=0` (deps internas ao `lente_core`). |
| **C. Solto** | União fica **363 nós** (cargo ignora). Protótipo reporta `1 solto` com sinal "presente, não compilado — ligue com um `mod`". Distingue de "nenhum arquivo novo" (baseline = 0 soltos). Sem panic, sem silêncio. |
| **D. Cache** | Re-extração dispara nos **dois** casos. A do solto é **espúria**: a chave usa **glob de filesystem** (`coletar_fontes` percorre `src/**.rs`), que inclui o solto → hash muda → re-extrai → ganha 0 nós. A do ligado é necessária (+3 nós após re-extrair). |
| **E. Quadro completo** | `git diff HEAD` (rastreados) ∪ untracked-ligado = completo para mudanças que viram nós. O solto "escapa" por construção (cargo não compila), mas é reportado à parte — não perdido. No cenário ligado, a edição do `mod` (rastreado) e o arquivo novo (untracked) aparecem em trilhas separadas e juntas formam o quadro. |
| Corte ligado vs solto funciona como descrito? | **Sim.** Cargo ignora o solto (363 vs 366); git pega os dois; a interseção via `position.file` decide sem segunda consulta ao cargo. |
| Montante quase vazio em arquivo novo? | **Confirmado** — 0 em todos os 3 nós novos. O valor da trilha local para arquivo novo está no **jusante**. |
| Enumeração de fontes do cache | **Filesystem glob** → erro de cache espúrio para soltos (uma re-extração ~3 s, auto-corrigida). |
| Surpresa | O módulo do arquivo novo tem `Uses` de saída (dir=2) pelo `use` no topo (Limite 4: import sai do módulo); as `fn` têm `Uses` por referência. Ambos chegam a trans=8. |
| **Recomendação `--diff` L2** | Incluir untracked por padrão; corte ligado/solto via fontes compiladas; sinal do solto de 1ª classe; **destacar jusante** para arquivo novo. Manter glob no cache (espúria barata) ou derivar lista do cargo — ajuste fino, não bloqueante. |
