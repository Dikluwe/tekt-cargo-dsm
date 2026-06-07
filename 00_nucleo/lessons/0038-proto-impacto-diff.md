# Laudo de Execução — Prompt 0038 (Protótipo do impacto de um diff)

**Camada**: L5 (laudo — registro de Arena)
**Data**: 2026-06-05
**Prompt executado**: `00_nucleo/prompt/0038-proto-impacto-diff.md`
**Tipo**: Arena visual descartável (`lab/proto-impacto-diff/`); padrão dos
laudos 0029 (`proto-ui`) e 0036 (`proto-dsm`) — **bruto em `lab/`,
registro aqui**.
**Estado**: `EXECUTADO` — pipeline + UI funcionam contra dado real do
próprio repo. Suíte de produção intacta (213 verdes + 22 ignored,
mesma do laudo 0037).

---

## O que rodou

Arena em `lab/proto-impacto-diff/`: binário Rust + UI HTML/JS estática.
Pipeline: **extrai grafo do crate alvo (com `position` do laudo 0037) →
lê `git diff` (stdin ou invocando `git diff HEAD`) → parseia hunks →
relativiza path → casa diff↔`position` → calcula raio por nó → emite
JSON**. UI consome JSON e mostra **em camadas** (`<details>` aninhado:
arquivo → nó tocado → amostra do montante).

Material da medição (contra modificações não-comitadas reais do laudo
0037):
- `lente_core`: 119 nós, 119 com `position`, 4 colisões de path.
- 10 arquivos no `git diff`; 2 com nó tocado.
- 3 nós tocados em `01_core/src/entities/grafo.rs`: módulo `grafo`
  (Folha, 0 transitivos), `Posicao` (Intermediário, 15) — a struct
  que adicionei — e `No` (Intermediário, 11).

---

## Resposta da decisão pendente (stdin vs git)

**Os conjuntos de caminhos são IGUAIS pelos dois caminhos** — para
arquivos rastreados (tracked) modificados.

```json
"comparacao": { "iguais": true, "so_em_stdin": [], "so_em_git": [] }
```

**Diferença descoberta na exploração** (registrada no `relatorio.md`):
nenhum dos dois vê **untracked** (arquivos novos sem `git add`). Para
o modo `--diff` do produto cobrir "qualquer mudança não-revisada"
incluindo arquivos novos, terá que tratar isso explicitamente —
`git ls-files --others --exclude-standard` + sintetizar hunks. Decisão
de produto, não da Arena.

Recomendação para o produto: default invocar `git diff HEAD`
(cômodo, sem pipe); manter opção `--diff-stdin` para colar diff de
PR/branch.

---

## Achados secundários

1. **Relativização sem fricção** no monorepo da lente: `position.file`
   absoluto + `git rev-parse --show-toplevel` + `strip_prefix` casa
   limpo. Sem descasamentos. Crates fora do repo / paths com symlinks
   ficam como bug latente potencial para o produto.

2. **Camadas funcionam** — o "aprofundar para ver mais fino" mostra
   detalhe útil (`Posicao` 15 transitivos vs módulo `grafo` 0
   transitivos), não ruído. Teste dos ~10s: passa neste tamanho de
   crate.

3. **Função do fork = código de release**, não testes inline. O ripple
   de `position: None` nos 9 helpers de teste (laudo 0037) **não**
   aparece como nó marcado — porque `#[cfg(test)] mod tests` fica
   fora do grafo. Coerente com a natureza estrutural da lente.

4. **Sem dedup necessário** para o caso testado — `BTreeSet<id>`
   já evita marcar o mesmo nó duas vezes quando duas faixas tocam
   nele.

5. **Honestidade declarada** na UI: nota amarela no topo dizendo
   "impacto estrutural, não comportamental". Atende o requisito da
   proposta sem impedir confusão por leitor distraído — para o
   produto, talvez valha o rótulo também na linha de cada nó.

---

## Decisões

- **Rust gera JSON, HTML lê** — espelha `proto-ui`/`proto-dsm`.
  Alternativa rejeitada: `eframe`/`egui` no mesmo binário (compilação
  pesada, pouco ganho para UI estática).
- **Grafo cru, sem `lente_resolve`** — primeira iteração; as 4 colisões
  do `lente_core` ficam registradas mas não corrigidas (raio dos 4
  paths colididos pode estar impreciso; nenhum deles foi tocado neste
  experimento).
- **`BTreeSet`/`BTreeMap` para determinismo** — protótipo emite JSON
  estável entre rodadas.
- **`ends_with` como fallback** para `position.file` que não bate a
  raiz do repo (defesa para casos exóticos não exercitados).

---

## Estado da suíte

| Item | Resultado |
|------|-----------|
| `cargo test --workspace` | **213 verdes + 22 ignored** — idêntica ao laudo 0037 |
| Crates de produção tocados | **Zero** — Arena pura |
| `Cargo.toml` raiz | intocado — `lab/proto-impacto-diff` tem `[workspace]` próprio |
| Subprocessos do cargo (invariante 0023) | dois únicos, intocados |

---

## Conteúdo bruto

```
lab/proto-impacto-diff/
├── Cargo.toml             # bin; deps lente_core + lente_infra
├── src/main.rs            # ~440 linhas; pipeline completo (CLI + parser + mapeamento + JSON)
├── index.html             # ~7 KB; UI em camadas, sem CDN
├── dados/
│   ├── impacto-ambos.json   # ~19 KB
│   ├── impacto-git.json     # ~9 KB
│   └── impacto-stdin.json   # ~11 KB
└── relatorio.md           # detalhe denso (perguntas do prompt + decisões + bordas)
```

O `relatorio.md` carrega o conteúdo denso (perguntas do prompt
respondidas em detalhe, decisões D1–D4, bordas exercitadas e
não-exercitadas). Este laudo é o **registro de que rodou**, com sumário
e ponteiro — convenção do laudo 0021.

---

## Para a próxima rodada

| Pendência | Estado |
|---|---|
| Modo `--diff` no produto | **Aberto** — Arena entrega material para decidir a forma (input default = `git diff HEAD` invocado; flag para stdin; tratamento de untracked) |
| Resolução de colisões antes do raio | **Aberto** — refinamento; 4 colisões no `lente_core` registradas |
| Macro call-site (briefing §5) | **Não exercitado** — sem edição em macro neste diff |
| Crate fora do repo / symlinks no `position.file` | **Não exercitado** — relativização pode falhar; bug latente |
| Casca MCP (Ponte 2 da trilha local) | **Aberta** — etapa posterior |

---

## Histórico de Revisões

| Data | Motivo | Arquivos afetados |
|------|--------|-------------------|
| 2026-06-05 | Arena `lab/proto-impacto-diff/`: binário Rust + UI HTML/JS que mapeiam um `git diff` aos nós tocados do grafo do crate alvo (usando `No.position` do laudo 0037), com cadeia de contenção e raio por camada. Comparação dos dois caminhos de input (stdin vs `git diff HEAD` invocado) **iguais para arquivos rastreados**; untracked é cego nos dois (achado para o produto). Vista visual em camadas com `<details>`. Mede a ideia da trilha local sobre o próprio repo da lente. Zero toque em produção. | `lab/proto-impacto-diff/{Cargo.toml,src/main.rs,index.html,dados/*.json,relatorio.md}`, `00_nucleo/lessons/0038-proto-impacto-diff.md` |
