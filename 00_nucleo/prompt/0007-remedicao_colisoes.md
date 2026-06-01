# Prompt: Remedição de Colisões de Path com Fork Novo

**Tipo**: Experimento de Arena (`lab/`)
**Camada**: trabalho de bancada — sem linhagem obrigatória, sem prompt L0 de
componente. Resultado é evidência empírica.
**Criado em**: 2026-05-27
**Decisões de origem**: relatório da primeira medição
(`lab/medicao-colisoes/relatorio.md`); fork atualizado com identidade-por-nó
(commit `5fbcdfe8` no fork); laudo 0006 (lente_core e lente_infra consumindo
ids).
**Pré-requisito**: fork novo instalado (`cargo install --git
https://github.com/Dikluwe/cargo-modules cargo-modules --force`); `lente_core`,
`lente_infra` e `lente_investiga` no estado pós-laudo 0006.

---

## Contexto

A primeira medição mediu colisões de path em 17 crates do typst v0.14.2 e
encontrou 384 colisões. Aplicando o `lente_investiga`, apenas 14.3% das
colisões foram decididas — todas pela Estratégia 2 (parser textual de
fontes). A Estratégia 1 (vizinhança no grafo) foi **estruturalmente
inaplicável**: o JSON do fork antigo referenciava arestas por path, sem
distinguir qual nó concreto cada extremo da aresta tocava quando havia
colisão.

O fork foi modificado para emitir `id` em cada nó e `id_from`/`id_to` em
cada aresta (commit `5fbcdfe8`). O `lente_core` e o `lente_infra` foram
ajustados para consumir essa identidade (laudo 0006). Agora a Estratégia 1
do `lente_investiga` pode finalmente operar — a vizinhança pode ser
separada por nó usando os ids.

Esta remedição responde à pergunta: **quanto a cobertura sobe agora?**

A pergunta é importante porque ela informa diretamente:

- Se vale construir o `lente_resolve` como o ADR-0004 desenha.
- Se a Estratégia 2 (parser textual) ainda tem papel ou pode ser removida.
- Se o ADR-0004 se sustenta com os números novos, precisa ser editado, ou
  pode ser superseded por um ADR-0005.

---

## Hipóteses a testar

Três hipóteses qualitativas que a medição deve confirmar ou refutar:

**H1**: A Estratégia 1 (vizinhança por id) decide a maioria das colisões em
crates Rust idiomáticos.
- Esperado: cobertura sobe de 0% (antes do `id`) para algo entre 50% e 90%.
- Se for verdade: a cascata se justifica; E2 fica como fallback.
- Se for falso: cenário inesperado, requer análise dos motivos.

**H2**: A Estratégia 2 ainda decide alguns casos que a E1 não decide.
- Esperado: contribuição da E2 cai (de 14.3% absoluto para algo menor em
  termos absolutos), mas continua não-nula nos casos onde a vizinhança é
  realmente coincidente (ex.: dois nós sem arestas, ou com arestas
  idênticas) e o trait pode ser inferido do código.
- Se for verdade: E2 tem valor reduzido mas presente.
- Se for falso (E2 nunca decide nada que E1 não decida): E2 pode ser
  removida do desenho.

**H3**: A categoria "NaoDeterminado" cai drasticamente.
- Esperado: dos 85.7% NaoDet anteriores, a grande maioria passa a ser
  decidida pela E1.
- Se for verdade: o ADR-0004 se sustenta empiricamente.
- Se for falso (NaoDet continua alto mesmo com E1 funcional): a arquitetura
  do ADR-0004 não cumpre sua função; revisão profunda necessária.

---

## Método

### Escopo

**Mesmos 17 crates** da primeira medição: typst (lib + cli), typst_bundle,
typst_eval, typst_html, typst_ide, typst_kit, typst_layout, typst_library,
typst_macros, typst_pdf, typst_realize, typst_render, typst_svg,
typst_syntax, typst_timing, typst_utils. Mesma versão do workspace (typst
v0.14.2) e mesmo caminho local (`lab/typst-original/`).

Comparabilidade direta com a primeira medição é o ponto central — qualquer
variação de escopo seria ruído na interpretação do delta.

### Ferramenta de chamada: programa Rust no `lab/`

Criar um pequeno programa Rust em `lab/medicao-colisoes/remedicao/` (subpasta
nova, não toca a pasta da primeira medição). Estrutura:

```
lab/medicao-colisoes/remedicao/
├── Cargo.toml         # crate binário, [workspace] vazio (fora do workspace pai)
├── src/
│   └── main.rs        # programa de medição
├── relatorio.md       # gerado ao fim
└── (json/ e análise — reutilizar/regerar a partir dos JSONs novos)
```

`Cargo.toml`: crate binário (`name = "remedicao"`, `[[bin]]`), edition 2024,
rust-version 1.91, com `[workspace]` (vazio) para se isolar do workspace pai.
Dependências:

- `lente_core` por path (`../../01_core` relativo ao Cargo.toml do
  remedicao).
- `lente_investiga` por path (`../../05_investiga`).
- `serde` (derive) e `serde_json` para parsear os JSONs do fork.

Não depende de `lente_infra` — esta medição parseia o JSON do fork
diretamente, sem subir o adaptador completo. Razão: queremos exercitar o
`lente_investiga` sobre dados parseados, sem o ruído de outros componentes.

### Fluxo do programa

Para cada um dos 17 crates:

1. **Gerar o JSON** com o fork novo: `cargo modules export-json --sysroot
   --compact --package <nome>` no diretório do workspace do typst. Capturar
   stdout, salvar em `lab/medicao-colisoes/remedicao/json/<crate>.json`.
   (Reaproveitar os JSONs antigos NÃO é opção — eles foram gerados com o
   fork antigo, sem `id`.)
2. **Parsear o JSON** num modelo interno do programa de medição (struct com
   `Vec<NoJson>`, `Vec<ArestaJson>` espelhando o formato). Não é o `Grafo`
   do `lente_core` — é representação intermediária do programa de medição.
3. **Detectar colisões**: agrupar nós por `path`; cada grupo com `len > 1` é
   uma colisão. Para cada colisão, extrair os ids dos nós envolvidos.
4. **Para cada colisão**, construir a `Vizinhanca` do `lente_investiga`:
   - Para cada nó colidente, filtrar as arestas do JSON cujo `id_from` ou
     `id_to` referencia esse nó.
   - Empacotar em `ArestasNo { entrando, saindo }` conforme a estrutura
     que o `lente_investiga` espera.
5. **Chamar `lente_investiga::investigar(par, vizinhanca, None)`** primeiro
   (Estratégia 1 isolada — sem fontes). Registrar o veredito.
6. **Se NaoDeterminado**, ler os arquivos `.rs` do crate-alvo (use o
   segmento de path para tentar localizar o arquivo certo; se não
   localizar, ler todos), construir `Vec<ArquivoFonte>`, e chamar
   `investigar(par, vizinhanca, Some(fontes))` (cascata completa).
   Registrar o veredito final.
7. **Coletar para o relatório**: por crate, por colisão, qual veredito veio
   de qual estratégia.

### Reproduzir o método de detecção da primeira medição

A primeira medição usou Python para detectar colisões e categorizar. Aqui o
programa Rust faz o equivalente, mas chamando o `lente_investiga` real. A
detecção em si (agrupar por path, identificar `len > 1`) é trivial.

### Estrutura do relatório

`lab/medicao-colisoes/remedicao/relatorio.md` deve conter:

**Seção 1 — Resumo executivo** (TL;DR de 5 linhas no topo): total de
colisões, % decidido por E1, % decidido por E2, % NaoDet. Comparado lado a
lado com a primeira medição.

**Seção 2 — Comparação com a primeira medição**: uma tabela lado a lado dos
agregados:

| Métrica | Antes (fork antigo) | Depois (fork novo) | Delta |
|---------|-----|------|-------|
| Total de colisões | 384 | ? | ? |
| Decididas por E1 | 0 (inaplicável) | ? | ? |
| Decididas por E2 | 55 (14.3%) | ? | ? |
| NaoDeterminado | 329 (85.7%) | ? | ? |

**Seção 3 — Resultados por crate** (tabela equivalente à da primeira
medição, agora com colunas extras para E1 vs E2):

| Crate | Colisões | Decididas (E1) | Decididas (E2) | NaoDet |
|-------|----------|----------------|----------------|--------|

**Seção 4 — Avaliação das hipóteses**: para cada hipótese (H1, H2, H3),
declarar se foi confirmada, parcialmente confirmada, ou refutada, com os
números que sustentam.

**Seção 5 — Padrões observados**: padrões interessantes que apareceram nas
decisões por E1 e nas que ainda ficaram NaoDet. Especialmente: casos onde
a vizinhança é genuinamente coincidente (provavelmente reexports ou alias),
e casos onde mesmo a cascata completa não resolve.

**Seção 6 — Avaliação contra os três cenários do ADR-0004**: repetir a
avaliação que a primeira medição fez (cenário A / B / C), agora com os
novos números.

**Seção 7 — Sugestões para a continuidade** (sem prescrever):
- Como esses números informam a decisão sobre o `lente_resolve`?
- A Estratégia 2 ainda tem valor, ou pode ser removida?
- O ADR-0004 se sustenta, precisa ser editado, ou pode ser superseded?

A decisão sobre o ADR-0004 e os próximos passos fica com o autor — o
relatório descreve, não prescreve.

**Seção 8 — Limites declarados desta medição**: mesma honestidade da
primeira (o que esta rodada não cobriu, o que ficou pendente).

---

## Restrições

- **Não modificar `lente_core`, `lente_infra`, ou `lente_investiga`.** Esta
  é medição contra o estado pós-laudo 0006.
- **Não criar `lente_resolve`.** A medição é justamente para informar a
  construção dele.
- **Não tentar resolver as colisões.** Só detectar e investigar.
- **Tudo vive em `lab/medicao-colisoes/remedicao/`**. Não toca a pasta da
  primeira medição (preserva o histórico).
- **Sem alterações no ADR-0004 ou em outros documentos L0.** Mudanças
  arquiteturais decorrentes da remedição são decisão posterior do autor,
  com base no relatório.

---

## Critérios de verificação (do programa de medição)

```
Dado o fork novo instalado e os 17 crates do typst disponíveis
Quando o programa de remedição é executado
Então gera 17 JSONs (um por crate) e um relatorio.md

Dado o JSON de um crate com colisões
Quando o programa parseia e detecta colisões
Então identifica os pares de nós com mesmo path e ids distintos

Dado uma colisão detectada
Quando lente_investiga::investigar é chamado com vizinhança separada por id
Então retorna um Veredito (Distintos / MesmoItem / NaoDeterminado)

Dado um Veredito NaoDeterminado da E1
Quando o programa lê as fontes e chama investigar com Some(fontes)
Então retorna o Veredito final da cascata completa

Dado todos os crates processados
Quando o relatório é gerado
Então contém: tabela comparativa antes/depois, resultados por crate,
avaliação das hipóteses, comparação com os três cenários do ADR-0004
```

---

## Tempos esperados

- Geração dos 17 JSONs com o fork novo: equivalente à primeira medição
  (~3-4 minutos no total).
- Análise (parsing + cascata): mais rápida que a primeira (programa Rust
  nativo vs. Python interpretado).
- Total: provavelmente entre 5 e 10 minutos.

Se algum crate falhar ao processar (erro do fork, JSON com problema, etc.),
registrar a falha no relatório como na primeira medição — falhas são dado
útil, não erro a esconder.

---

## Resultado mínimo aceitável

Um relatório em Markdown que permite ao autor do projeto **decidir
informadamente** sobre os próximos passos do ADR-0004 (sustentar, editar,
ou supersede), com base na comparação direta antes/depois do `id`.

A interpretação fica com o autor; o relatório descreve.
