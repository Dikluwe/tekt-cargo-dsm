# Prompt: as três vistas de texto do `--diff` (`--vista`) — fecha o modo (L2)

**Camada**: L2 — Shell (`lente_cli`)
**Criado em**: 2026-06-06
**Estado**: `EXECUTADO` (laudo `00_nucleo/lessons/0048-vistas_texto_diff.md`)
**Decisões de origem**: decisão do autor — dado separado da vista; o `ResultadoDiff`
(0047) é completo e view-agnóstico, o JSON o emite cru, e **as três vistas são
renderizadores** sobre ele, o visualizador (ou o `--vista`) decide. Laudo 0043 (os
4 itens; a ênfase se adapta — arquivo novo destaca jusante, modificação destaca
montante; o sinal do solto é de primeira classe).
**Pré-requisito**: 0047 (`ResultadoDiff`, o modo `--diff`, o JSON).
**Arquivos afetados**: `02_shell/cli/src/{args.rs,saida.rs}`, `02_shell/catalogo/src/lib.rs`,
testes.

---

## Contexto e escopo

O 0047 entregou o `ResultadoDiff` completo (tocados com raio, raio combinado,
censo do untracked, fantasmas) e a CLI `--diff` emitindo o **JSON**. Este prompt
adiciona as **três vistas de texto** como renderizadores sobre esse mesmo
resultado, selecionadas por `--vista`. Fecha o modo `--diff` (e a trilha local).

**É só L2.** O dado já está completo — paths, `classificacao` (de cada raio),
contagens de montante/jusante, o combinado com profundidade, o censo, os fantasmas.
**Nada** em L1 nem L4 muda. As vistas leem o `ResultadoDiff` e formatam.

---

## Restrições estruturais

- **L2 — formatação pura sobre o `ResultadoDiff`.** As vistas não recomputam nada;
  derivam tudo do resultado (o crate de um nó = 1º segmento do path).
- **Catálogo (ADR-0002)** para todo texto ao usuário — nada hardcoded.
- **Retrocompat dura**: o modo **global** da CLI e o **JSON** do `--diff` (0047)
  ficam **inalterados**. O `--vista` é aditivo; sem ele, `--diff` segue emitindo
  JSON.

---

## O `--vista`

`--vista <resumo|item|camadas>` — seleciona uma vista de texto. **Ausente** →
JSON (o padrão do 0047, intocado). Só vale com `--diff`.

---

## As três vistas (estrutura; o executor refina o layout exato, com o catálogo)

Cenário ilustrativo: o diff tocou `lente_core::No` e criou `cli_novo.rs` sem `mod`.

### A — `resumo` (curto, foco no impacto)

```
diff: 2 tocados em 1 crate
  pode quebrar (montante): 44 — lente_infra 9 · lente_wiring 14 · lente_resolve 7 · …
  depende de (jusante): 3 — lente_core 3
  untracked: 3 compilados · 1 sem mod · 15 não-fonte
    sem mod (não compilado): cli_novo.rs
```

- contagens: tocados (e nº de crates distintos, dos paths), `montante`/`jusante`
  do `combinado`, agrupados por crate (1º segmento do path), com a contagem por
  crate.
- **Ênfase adaptativa (0043)**: se o `combinado.montante` é vazio **e** há
  `ligados` (diff só de arquivo novo), liderar com o **jusante** (o que o código
  novo passa a usar); senão, liderar com o **montante**.
- censo numa linha; **solto listado** (sinal acionável).
- `fantasmas`: só aparece se > 0 (esperado 0, laudo 0041).

### B — `item` (por item tocado)

```
2 tocados:
  lente_core::No  [Base]
    pode quebrar: 44   depende de: —
  lente_core::entities::grafo  [Intermediario]
    pode quebrar: 44   depende de: 3
untracked: 3 compilados · 1 sem mod · 15 não-fonte
  sem mod: cli_novo.rs
```

- um bloco por tocado: o path, a `classificacao` (do raio do tocado) e as
  contagens de montante/jusante daquele nó.
- o censo + os soltos listados; fantasmas se > 0.

### C — `camadas` (estrutural, por crate)

```
tocados por crate:
  lente_core
    No [Base], entities::grafo [Intermediario]
  pode quebrar, por crate: lente_infra 9 · lente_wiring 14 · lente_resolve 7 · …
untracked: 3 compilados · 1 sem mod · 15 não-fonte
  sem mod: cli_novo.rs
```

- agrupa os tocados **por crate** (do path), e mostra o **impacto cross-crate** (o
  `combinado.montante` agrupado por crate — quem, em cada crate, depende dos
  tocados). Ordem sensata por crate (decisão do gerador; o nº de diretório ou o
  nome serve). É a versão prática de "camadas" — derivada dos paths, **sem** puxar
  o `lente_estrutura` (09). Uma vista de camadas fiel ao layering do 09 pode vir
  depois, se pedida; **não** trazer aqui salvo se for trivial.
- censo + soltos; fantasmas se > 0.

As três compartilham o rodapé do censo e o realce do solto; diferem em como
arranjam os tocados e o impacto.

---

## O que NÃO muda

- O `ResultadoDiff` (L1, 0047), o `analisar_diff` (L4, 0047), o **JSON** do `--diff`
  (0047) — intocados; as vistas só leem o resultado.
- O modo **global** da CLI — intocado.

---

## Critérios de Verificação

```
Dado um ResultadoDiff forjado (2 tocados com raio, censo com 1 solto, 0 fantasmas)
Quando renderizar a vista resumo
Então o texto traz a contagem de tocados/crates, o montante e o jusante por
crate, o censo, e lista o solto

Quando renderizar a vista item
Então um bloco por tocado com path, classificacao e contagens

Quando renderizar a vista camadas
Então os tocados agrupados por crate e o impacto cross-crate por crate

Dado --diff --vista resumo|item|camadas
Então sai a vista de texto correspondente

Dado --diff sem --vista
Então sai o JSON (0047, inalterado)

Dado um ResultadoDiff de diff só-arquivo-novo (montante vazio, ligados presentes)
Quando renderizar a vista resumo
Então lidera com o jusante (ênfase adaptativa, 0043)

Dado um ResultadoDiff com 1 solto
Então todas as vistas listam o solto (sinal acionável)

Dado fantasmas > 0
Então as vistas o sinalizam (e some quando 0)

Dado o mesmo ResultadoDiff
Quando renderizar duas vezes
Então o mesmo texto (determinístico)

Dado o modo global da CLI e o JSON do --diff
Quando rodar seus testes existentes
Então todos passam (--vista é aditivo)
```

Casos: as três vistas (estrutura de cada uma); o roteamento do `--vista`; o JSON
ainda como padrão sem `--vista`; a ênfase adaptativa; o solto listado; o fantasma
sinalizado só se > 0; determinismo; não-regressão do global e do JSON. As vistas
testáveis sobre um `ResultadoDiff` forjado, **sem git/fork**.

---

## Resultado esperado

- Três renderizadores de texto (resumo / item / camadas) sobre o `ResultadoDiff`,
  na L2; a flag `--vista`; o JSON padrão **inalterado**.
- Strings no catálogo (ADR-0002).
- Testes: cada vista (estrutura), o roteamento `--vista`, o JSON-sem-flag, a
  ênfase adaptativa, o solto listado, o fantasma condicional, determinismo,
  não-regressão.
- **Laudo** em `00_nucleo/lessons/0048-…`:
  - O layout de cada vista (com um exemplo real do repo, via o binário).
  - O agrupamento por crate (derivado do path).
  - A ênfase adaptativa (como detecta diff só-arquivo-novo).
  - Se a vista `camadas` ficou por-crate (esperado) ou puxou o `lente_estrutura`
    (e por quê).
  - Que o JSON e o modo global seguem inalterados.
  - Contagem da suíte (era 265 verdes + 28 ignored no laudo 0047).

---

## Cuidados

- **Só L2.** Nada em L1/L4 muda; as vistas leem o `ResultadoDiff`. Se uma vista
  parecer precisar de algo que não está no resultado (ex.: `kind` por nó), **parar
  e registrar** — não enriquecer L1/L4 por conta (a `classificacao` do raio cobre
  o rótulo dos nós; `kind` é enriquecimento futuro, fora deste prompt).
- **Sinal do solto de primeira classe (0043)**: listar os soltos ("sem mod, não
  compilado"), em todas as vistas — é o caso acionável, não um aviso escondido.
- **Ênfase adaptativa (0043)**: arquivo novo (montante vazio + ligados) destaca o
  jusante; modificação destaca o montante.
- **Vista `camadas` leve**: agrupar por crate dos paths; **não** trazer o
  `lente_estrutura` (09) salvo se trivial — a vista fiel ao layering fica para
  depois, se pedida.
- **Catálogo (ADR-0002)** para todo texto; nada hardcoded.
- **JSON padrão intocado** (0047): sem `--vista`, `--diff` emite JSON.
- **Determinismo**: as vistas ordenam o que iteram (tocados por path, crates por
  ordem fixa).

---

## Histórico de Revisões

| Data | Motivo | Arquivos afetados |
|------|--------|-------------------|
| 2026-06-06 | Fecha o modo `--diff` (e a trilha local) com as três vistas de texto, na L2, como renderizadores sobre o `ResultadoDiff` (0047): **resumo** (curto — contagem de tocados/crates, montante/jusante por crate, censo, solto listado; ênfase adaptativa do 0043: arquivo novo destaca jusante, modificação destaca montante), **item** (um bloco por tocado: path + `classificacao` do raio + contagens), **camadas** (tocados agrupados por crate + impacto cross-crate por crate — versão leve, derivada dos paths, **sem** puxar o `lente_estrutura`). Flag `--vista <resumo\|item\|camadas>`; **ausente → JSON** (padrão do 0047, intocado). Só L2: nada em L1/L4 muda; as vistas leem o resultado (a `classificacao` cobre o rótulo dos nós — `kind` não está no dado, fica para depois). Strings no catálogo (ADR-0002). Modo global e JSON do `--diff` inalterados. Testes sobre `ResultadoDiff` forjado (sem git/fork): cada vista, roteamento do `--vista`, JSON-sem-flag, ênfase adaptativa, solto listado, fantasma condicional, determinismo, não-regressão. Suíte era 265+28. | `02_shell/cli/src/{args.rs,saida.rs}`, `02_shell/catalogo/src/lib.rs`, `00_nucleo/lessons/0048-...` |
