# Laudo de Execução — Prompt 0043 (Arena — validar untracked no protótipo de impacto de diff)

**Camada**: L5 (laudo — registro de Arena)
**Data**: 2026-06-05
**Prompt executado**: `00_nucleo/prompt/0043-arena_untracked_validacao.md`
**Tipo**: Arena visual, extensão do `lab/proto-impacto-diff/` (rodadas 0038→0042) —
bruto em `lab/`, registro aqui (padrão dos laudos 0021, 0029, 0036, 0038, 0040).
**Estado**: `EXECUTADO` — detecção de untracked + corte ligado-vs-solto +
hunks sintéticos + observação montante/jusante + interação com cache, validados
no repo real. Cenários ligado/solto rodados e **limpos ao final** (`git status`
de produção idêntico ao baseline). Zero toque em produção.

---

## A resposta em uma sentença

O corte **untracked ∩ fontes-que-o-cargo-compila** funciona exatamente como o
prompt previu: o `git ls-files --others` pega arquivo novo ligado **e** solto;
a interseção com as `position.file` do grafo separa os dois sem ambiguidade; o
ligado mapeia ao grafo (montante ≈ vazio, **jusante é o valor**) e o solto vira
sinal acionável ("presente, não compilado") em vez de omissão silenciosa ou
panic.

---

## Como rodar

```bash
cd lab/proto-impacto-diff
REPO="$(cd ../../ && pwd)"

# Estado limpo (nenhum .rs novo): untracked .rs em crate = 0
cargo run --release -- --repo "$REPO" --input git --out dados/untracked-baseline.json

# Cenário LIGADO: criar 01_core/src/entities/novo_ligado.rs + `pub mod novo_ligado;`
# em entities/mod.rs (edição não-comitada), depois rodar:
cargo run --release -- --repo "$REPO" --input git --out dados/untracked-ligado.json

# Cenário SOLTO: criar 01_core/src/entities/novo_solto.rs SEM `mod`, depois rodar:
cargo run --release -- --repo "$REPO" --input git --out dados/untracked-solto.json
```

A detecção de untracked é **sempre** executada (uma chamada a `git ls-files`,
custo desprezível); o campo `untracked` do JSON traz o censo
ligados/soltos/não-fonte e o impacto (com jusante) dos ligados.

---

## Respostas às perguntas A–E

### A. Detecção — **sim, limpa**

`git ls-files --others --exclude-standard` lista os untracked corretamente,
**respeitando `.gitignore`** (o `target/` do lab fica de fora — confirmado por
`git check-ignore`). No estado limpo: 56 untracked, dos quais o único `.rs` é o
próprio `lab/proto-impacto-diff/src/main.rs` — que **não** é membro do
workspace principal (lab tem `[workspace]` próprio), logo cai em "não-fonte". O
protótipo sintetiza um hunk "tudo adicionado" (`Faixa { inicio: 1, fim:
n_linhas }`) lendo o arquivo do disco — entra no mesmo mapeamento por linha dos
rastreados.

### B. Arquivo novo LIGADO — **grafo passa a tê-lo; jusante carrega o valor**

`01_core/src/entities/novo_ligado.rs` (21 linhas, 2 `fn` usando `&No`/`&Path`)
+ `pub mod novo_ligado;` como edição não-comitada:

- **Cache erra → re-extrai**: `cenário=morno-1, extraídos=1` (lente_core),
  3.47 s. A união sobe de **363 → 366 nós** (3 novos: o módulo + as 2 `fn`).
- **Hunks sintetizados mapeiam aos nós por `position`**: 3 nós tocados, casados
  pela reconciliação absoluto↔relativo já existente (laudo 0038).
- **Impacto** (confirma a tese do prompt):

  | nó | kind | montante (quem sente) | jusante (do que depende) |
  |---|---|---|---|
  | `…::novo_ligado` (mód.) | Mod | **dir=0 trans=0** `[]` | dir=2 trans=8 `No, Path, Kind, Posicao, …` |
  | `…::novo_ligado::descrever_no` | Fn | **dir=0 trans=0** `[]` | dir=2 trans=8 `No, Path, …` |
  | `…::novo_ligado::path_vazio` | Fn | **dir=0 trans=0** `[]` | dir=1 trans=2 `Path, String` |

  **Montante vazio em todos** (ninguém ainda depende de código novo) — exatamente
  como previsto. O `jusante` mostra o que cada item passou a usar. `delta_cross_crate
  = 0`: as dependências (`No`/`Path`) são do próprio `lente_core`; o jusante cruza só
  para `alloc`/`core` (stdlib), não para outro crate-membro.
  - Nuance Limite-4: o **módulo** tem `dir=2` por causa do `use crate::entities::grafo::{No, Path}`
    no topo (import sai do módulo); as `fn` têm `Uses` por referência. Ambos chegam a
    `trans=8` (o cluster `grafo` alcançável). Coerente com a spec.

### C. Arquivo novo SOLTO — **grafo o ignora; sinal acionável, não silêncio**

`01_core/src/entities/novo_solto.rs` (mesma forma, **sem** `mod`):

- **O grafo não o tem**: união permanece em **363 nós** (o cargo não compila o
  arquivo solto → o fork não emite nós).
- **Distinção preservada**: `0 ligados, 1 soltos`, com `sinal = "presente, não
  compilado — ligue com um `mod` no módulo-pai"`. O baseline (sem `.rs` novo) dá
  `0 soltos` — logo "arquivo novo presente, não compilado" ≠ "nenhum arquivo
  novo". Sem panic, sem omissão.

### D. Interação com o cache — **re-extração dispara nos DOIS casos (a do solto é espúria)**

A chave de cache do protótipo enumera fontes por **glob de filesystem**
(`coletar_fontes` percorre `src/**.rs`), **não** pela lista do cargo. Consequência
medida:

| caso | hash muda? | re-extrai? | nós novos? | veredito |
|---|---|---|---|---|
| ligado | sim (arquivo + `mod`) | sim, 3.47 s | **+3** | re-extração **necessária** |
| solto | sim (só o arquivo) | sim, 2.93 s | **+0** | re-extração **espúria** |

O glob inclui o arquivo solto → o hash muda → re-extrai → mas o cargo continua
ignorando o solto → **nada se ganha**. Se a chave enumerasse pela lista do cargo
(que pula os soltos), não haveria erro de cache espúrio — ao custo de a própria
enumeração precisar saber a ligação `mod`. Para os ligados, a re-extração via
glob aparece e os nós surgem após ela (coluna "+3").

### E. Quadro completo — **sim: rastreados ∪ untracked-ligado cobre a árvore de trabalho**

`git diff HEAD` (rastreados editados) ∪ hunks sintéticos (untracked ligados) dão
o quadro completo das **mudanças que viram nós**. O que "escapa" escapa por
construção, não por bug: o arquivo **solto** não está no grafo porque o cargo não
o compila — e é reportado à parte como sinal, não perdido. No cenário ligado, a
edição do `mod` em `entities/mod.rs` (arquivo rastreado) aparece no `git diff` e o
arquivo novo aparece via untracked: as duas pontas da mesma mudança ficam visíveis
em trilhas separadas, e juntas formam o quadro.

---

## Confirmações do corte ligado vs solto

- **O cargo de fato ignora o solto**: união 363 (solto) vs 366 (ligado).
- **A lista do git pega os dois**: `git ls-files --others` listou tanto
  `novo_ligado.rs` quanto `novo_solto.rs` (cada um no seu cenário).
- **A interseção decide**: usar as `position.file` dos nós do grafo unido como
  "verdade-de-campo do que o cargo compilou" é o critério mais barato e direto —
  não exige uma segunda consulta ao cargo; o grafo já é a resposta.

---

## Decisões

- **Interseção via `position.file` do grafo unido** (não uma chamada separada a
  `cargo build --message-format=json` para listar fontes). O grafo já carrega a
  verdade do que foi compilado; reusá-lo é zero custo extra.
- **`RaioResumo` ganhou jusante** (`diretos_saida`, `transitivos_jusante`,
  `amostra_jusante`) — o `lente_core::raio::Raio` já calculava `jusante`; só não
  estava exposto no JSON. Para arquivo novo, é o campo que importa.
- **Detecção sempre-ligada** (sem flag): uma chamada a `git ls-files` é barata; o
  censo `untracked` entra em todo run.
- **Não-fonte só contagem + amostra** (`nao_fonte_total` + 10) — evita inflar o
  JSON com os 50+ untracked de docs/lab.

---

## Recomendação para o modo `--diff` (L2) de produção

**O que vale nuclear:**

1. **Incluir untracked por padrão** no modo `--diff` (a pergunta "o que quebra se
   eu mexer aqui?" inclui arquivo que acabei de criar). `git diff HEAD` ∪
   `git ls-files --others --exclude-standard`, com hunks sintéticos "tudo
   adicionado" para os untracked.
2. **Corte ligado vs solto** via interseção com as fontes compiladas (as
   `position.file` do grafo). É a peça nova; nuclear na infra L3 (que monta o
   grafo de workspace) + L2 (que consome).
3. **Sinal do solto** como saída de primeira classe ("presente, não compilado —
   ligue com um `mod`"), não warning escondido. É acionável e evita o pior caso
   (silêncio que parece "sem impacto").
4. **Para arquivo novo, destacar o jusante** na vista, não o montante. O montante
   de código novo é ~vazio por natureza; o valor da lente para arquivo novo está
   em "o que ele passa a usar / acoplar".

**O que descartar / decidir depois:**

- **Enumeração da chave de cache (glob vs cargo)**: a re-extração espúria do solto
  custa **uma** rodada do fork (~3 s) e é auto-corrigida. Recomendação: **manter o
  glob** (simples, robusto) e aceitar o custo, OU — se o custo incomodar — derivar
  a lista de fontes do `cargo` para a chave. Não é bloqueante; registrar como
  ajuste fino, não requisito.
- O quadro combinado é completo para o propósito da lente (mudanças estruturais);
  nada a nuclear além dos itens 1–4.

---

## Estado da suíte / produção

| Item | Resultado |
|------|-----------|
| Crates de produção tocados | **Zero residual** — `git status` de produção idêntico ao baseline (26 entradas) |
| Arquivos de teste | `novo_ligado.rs` + `novo_solto.rs` criados e **removidos**; `mod` revertido (`git checkout`) |
| `Cargo.toml` raiz | intocado — `lab/proto-impacto-diff` tem `[workspace]` próprio |
| Fork tocado | **Não** — só invocado |
| Cache do lab | refrescado ao estado limpo ao final (próximo run = quente) |

---

## Conteúdo bruto

```
lab/proto-impacto-diff/
├── src/main.rs            # +~140 linhas: ler_untracked, fontes_compiladas,
│                          #   contar_linhas, UntrackedResumo/Ligado/Solto,
│                          #   RaioResumo com jusante, bloco 7.3 no main
├── dados/
│   ├── untracked-baseline.json  # estado limpo (0 .rs em crate)
│   ├── untracked-ligado.json    # 366 nós; 1 ligado, 3 nós tocados, jusante
│   └── untracked-solto.json     # 363 nós; 0 ligados, 1 solto (sinal)
└── relatorio.md           # seção 0043 anexada (perguntas A–E em detalhe)
```

---

## Para a próxima rodada

| Item | Estado |
|---|---|
| Untracked (achado do 0038) | **Coberto** — ligado/solto, hunks sintéticos, sinal |
| Jusante na vista (arquivo novo) | **Coberto** — exposto no `RaioResumo` |
| Cache key: glob vs cargo p/ soltos | **Documentado** — espúria mas barata; ajuste fino |
| Nucleação do motor (infra L3 + `--diff` L2) | **Aberto** — agora informada por este laudo |
| Cache key inclui `Cargo.toml` | **Aberto** (herdado do 0040) |
| Casca MCP | **Aberto** |

---

## Histórico de Revisões

| Data | Motivo | Arquivos afetados |
|------|--------|-------------------|
| 2026-06-05 | Validação na Arena do tratamento de untracked antes de nuclear o motor. Estendeu `lab/proto-impacto-diff/` com `git ls-files --others --exclude-standard`, corte ligado-vs-solto (interseção com as `position.file` do grafo = fontes que o cargo compilou), hunks sintéticos "tudo adicionado" para os ligados, e exposição do jusante no `RaioResumo`. Cenários no repo real: **ligado** (união 363→366, montante vazio, jusante carrega `No`/`Path`, cache re-extrai necessariamente) e **solto** (união fica 363, sinal "presente, não compilado", cache re-extrai **espuriamente** porque a chave usa glob de filesystem). Limpeza confirmada: `git status` de produção idêntico ao baseline. Recomendação para o `--diff` L2: incluir untracked por padrão, corte ligado/solto via fontes compiladas, sinal do solto de 1ª classe, destacar jusante para arquivo novo; manter o glob no cache (espúria barata, auto-corrigida). Zero toque em produção. | `lab/proto-impacto-diff/{src/main.rs,dados/untracked-*.json,relatorio.md}`, `00_nucleo/lessons/0043-arena_untracked_validacao.md` |
