# Prompt: Arena — validar untracked no protótipo de impacto de diff

**Camada**: Arena (`lab/proto-impacto-diff/`) — regime relaxado, descartável,
sem linhagem.
**Criado em**: 2026-06-05
**Estado**: `EXECUTADO` (laudo `00_nucleo/lessons/0043-arena_untracked_validacao.md`)
**Origem**: laudo 0038 (untracked cego — `git diff HEAD` não mostra arquivo
novo); laudo 0040 (cache por SHA dos fontes); laudos 0041/0042 (resolução por
crate, correta após a escada).
**Objetivo**: validar o tratamento de arquivos não rastreados **antes** de
nuclear o motor (a infra L3 e o modo `--diff` L2). Untracked é a única ponta da
trilha local que nem o 0038 explorou nem o cache cobriu. Saída = laudo com
achados, não código de produção.

---

## Contexto

O protótipo lê `git diff HEAD` (ou stdin), mapeia as linhas alteradas a nós por
`position` (file + faixa de linhas, laudo 0037), calcula o raio e mostra a vista
em camadas. Funciona para arquivos **rastreados** (laudo 0038), com cache (0040)
e resolução por crate (0041/0042).

O ponto cego: um arquivo **novo, não rastreado** não aparece em `git diff HEAD`
— logo o protótipo não o vê. Antes de o motor incluir untracked, é preciso saber
o que isso exige de fato.

### A distinção que decide tudo

Um arquivo `.rs` novo só vira nós no **grafo** se o `cargo` o **compila** — ou
seja, se está **ligado** à árvore de módulos (uma declaração `mod foo;` em algum
lugar, mesmo que essa declaração seja uma edição não-comitada). Então há dois
sub-casos:

- **Ligado** (`mod` declarado): o `cargo` compila → o fork emite os nós → o grafo
  os tem (com `position` no arquivo novo).
- **Solto** (sem `mod`): o `cargo` ignora → o grafo **não** tem os nós.

E o `git ls-files --others` enxerga os **dois** (é nível de filesystem/git). Isso
dá uma saída limpa: o protótipo cruza as duas listas —

```
untracked ∩ fontes-que-o-cargo-compila  = arquivo novo LIGADO   → mapeia ao grafo
untracked \ fontes-que-o-cargo-compila  = arquivo novo SOLTO    → reporta "presente, não compilado"
```

— em vez de "untracked some no grafo" (silencioso) ou panic.

---

## Perguntas a responder (com o repo real)

**A. Detecção.** `git ls-files --others --exclude-standard` lista os untracked
corretamente? O protótipo consegue sintetizar hunks "todas as linhas
adicionadas" para eles (para entrarem no mapeamento por linha)?

**B. Arquivo novo ligado.** Com um `.rs` novo ligado (arquivo + `mod` como edição
não-comitada): o grafo passa a ter seus nós (cache erra → re-extrai)? Os hunks
sintetizados mapeiam aos nós por `position`? Que impacto aparece — o `montante`
(quem depende dele; provavelmente vazio, código novo não tem dependentes ainda) e
o `jusante` (o que ele usa; deve mostrar as dependências do arquivo novo)?

**C. Arquivo novo solto.** Com um `.rs` novo **sem** `mod`: o grafo o ignora (o
`cargo` não compila)? O protótipo **distingue** "arquivo novo presente, não
compilado" de "nenhum arquivo novo" — sinal acionável ("ligue com um `mod`"), não
omissão silenciosa nem panic?

**D. Interação com o cache.** Adicionar um arquivo dispara erro de cache
(re-extração)? Por qual enumeração de fontes — a do `cargo` (pula os soltos, sem
erro de cache espúrio) ou um glob do filesystem (inclui os soltos, erro de cache
espúrio que re-extrai e não ganha nada)? Reportar qual, e se os nós do arquivo
novo ligado aparecem após a re-extração.

**E. Quadro completo.** `git diff HEAD` (rastreados) ∪ hunks sintetizados
(untracked) dão o quadro completo da árvore de trabalho (edições em rastreados +
arquivos novos), ou alguma combinação escapa?

---

## Como validar (estender o protótipo)

No `lab/proto-impacto-diff/` (Arena, pode importar os crates de produção
`lente_infra`/`lente_resolve`/`lente_core`/`lente_wiring`):

1. Acrescentar a detecção `git ls-files --others --exclude-standard`.
2. Cruzar com o conjunto de fontes que o `cargo` compila (ligado vs solto).
3. Sintetizar hunks "tudo adicionado" para os arquivos novos ligados; mapear ao
   grafo por `position` (reusar a reconciliação de caminho absoluto↔relativo que
   já existe para rastreados, laudo 0038).
4. Para os soltos, emitir o sinal "presente, não compilado".
5. Observar o `montante`/`jusante` dos nós novos e a interação com o cache.

### Cenários de teste (repo real — limpar depois)

1. **Ligado**: criar um `.rs` novo em `01_core/src/` com 1–2 itens que usam tipos
   existentes (ex.: uma `fn` que recebe `&No` ou `&Path`), e adicionar a
   declaração `mod` no módulo-raiz como edição não-comitada. Rodar o protótipo.
   Observar: inclusão no grafo, mapeamento, impacto (`montante` esperado vazio;
   `jusante` mostra os tipos usados).
2. **Solto**: criar um `.rs` novo em `01_core/src/` **sem** declaração `mod`.
   Rodar. Observar: o grafo o ignora; o protótipo reporta "presente, não
   compilado".
3. **Limpeza**: remover os arquivos de teste e reverter a edição do `mod`.
   Confirmar `git status` limpo ao final.

---

## O que reportar (laudo)

- Respostas a A–E.
- O corte ligado vs solto: funciona como descrito? O `cargo` de fato ignora o
  solto, e a lista do `git` o pega?
- O `montante`/`jusante` de um arquivo novo: confirma a observação de que código
  novo tem `montante` quase vazio (valor da trilha local para arquivo novo está no
  `jusante` — o que ele passa a usar — mais que no `montante`)?
- A enumeração de fontes do cache (cargo vs filesystem) e a consequência (erro de
  cache espúrio para soltos, ou não).
- Se o quadro combinado (rastreados ∪ untracked) é completo, ou se algo escapa.
- Qualquer surpresa.
- **Recomendação**: como o modo `--diff` (L2) de produção deve tratar untracked —
  o que vale nuclear, o que descartar.

---

## Regime da Arena

- **Relaxado**: sem cabeçalho de linhagem, sem ADR, protótipo descartável.
- **Importa de produção, produção não importa do lab** (ADR da Arena).
- **A única regra estrita**: é o **repo real**. Os arquivos de teste criados
  precisam ser **removidos** e a edição do `mod` **revertida** ao final —
  `git status` limpo. Não deixar resíduo no working tree.

---

## O que NÃO entra

- **Nenhuma mudança em produção.** Isto é validação na Arena. A nucleação do motor
  (infra L3 = grafo de workspace com extração + resolução + união + cache de chave
  completa; modo `--diff` L2 = lê diff incl. untracked, mapeia, raio, texto+JSON)
  vem **depois**, informada por este laudo.
- A chave de cache completa (Cargo.toml/lock + toolchain) — já validada como
  conceito no 0040; entra na infra L3, não aqui.

---

## Histórico de Revisões

| Data | Motivo | Arquivos afetados |
|------|--------|-------------------|
| 2026-06-05 | Validação na Arena do tratamento de untracked antes de nuclear o motor. Estende `lab/proto-impacto-diff/` com `git ls-files --others`, corte ligado-vs-solto (interseção com as fontes que o cargo compila), hunks sintéticos para arquivos novos ligados, e observação do `montante`/`jusante` + interação com o cache. Cenários no repo real (ligado, solto, limpeza com `git status` limpo ao final). Saída = laudo com achados e recomendação para o modo `--diff` L2; zero mudança em produção. | `lab/proto-impacto-diff/` (Arena, descartável) |
