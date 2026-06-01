# Prompt: Medição de Prevalência de Colisões de Path

**Tipo**: Experimento de Arena (`lab/`)
**Camada**: trabalho de bancada — sem linhagem obrigatória, sem prompt L0 de
componente. Resultado é evidência empírica.
**Criado em**: 2026-05-27
**Decisões de origem**: ADR-0004 (declara-se revisável conforme dados);
lição L3 do `LESSONS.md` (lab como bancada)
**Depende de**: `lente_infra` e `lente_investiga` em estado funcional
(implementados, testes verdes); fork do `cargo-modules` instalado

---

## Contexto e propósito

O ADR-0004 decidiu a arquitetura de resolução de colisões (cascata
vizinhança→fonte, dois crates L1) **sem dado** sobre a prevalência real do
problema. O laudo do `lente_infra` mediu **uma** colisão concreta
(`ErroRaio::fmt` em `lente_core`); o laudo do `lente_investiga` registrou no
item "Próximos componentes" que medir contra ~20 crates reais é o passo que
informa a continuidade.

Este experimento mede:

1. **Prevalência**: quantos crates Rust idiomáticos têm colisões de path
   detectáveis pelo `lente_infra`?
2. **Abrangência das estratégias**: das colisões encontradas, quantas a
   cascata do `lente_investiga` resolve com `MesmoItem` ou `Distintos`,
   quantas ficam `NaoDeterminado`?
3. **Padrões das colisões**: o que tipicamente causa colisão? Só
   `Display+Debug`, ou outros padrões aparecem?

O resultado é **evidência para decidir**: prosseguir com o `lente_resolve`
como o ADR-0004 desenha, ou revisar o ADR-0004 com base no que a medição
mostrar.

---

## Resultado esperado

Um **relatório em Markdown** em `lab/medicao-colisoes/relatorio.md` (ou caminho
equivalente, registrar onde) contendo:

- Lista dos crates testados, com tamanho aproximado e razão da escolha.
- Por crate: quantas colisões detectadas, e para cada colisão, o veredito
  do `lente_investiga`.
- Agregados:
  - % de crates com pelo menos uma colisão.
  - Total de colisões e distribuição por tipo de veredito
    (MesmoItem / Distintos via vizinhança / Distintos via fontes /
    NaoDeterminado).
  - Padrões observados em `Distintos via fontes` (quais traits aparecem em
    impls com método de mesmo nome).
  - Casos onde a cascata falhou: descrição do que cada estratégia tentou.
- Conclusão: avaliação se os dados sustentam ou ferem o desenho do
  ADR-0004 (sem prescrição — a interpretação fica com o autor do projeto).

---

## Critérios de seleção dos crates

Escolha **12 a 20 crates** Rust reais, cobrindo quatro categorias com 3 a 5
crates em cada. Para cada crate, registrar no relatório a razão da escolha.

**Categoria 1 — Crates pequenos idiomáticos.** Bibliotecas pequenas que
seguem padrões comuns: enums de erro com `thiserror` ou `derive Debug +
impl Display`, structs com derives padrões (`Clone`, `Debug`, `PartialEq`).
São o caso "típico" do ecossistema. Exemplos válidos: `anyhow`, `thiserror`,
crates pequenos do ecossistema Rust que você conheça.

**Categoria 2 — Crates grandes e complexos.** Projetos com muito código,
possivelmente com estilos não-idiomáticos, macros complexas, genéricos
pesados. Exemplos: `serde`, `tokio`, `clap`, `regex`. Espera-se que esses
revelem padrões que os pequenos não mostram.

**Categoria 3 — Bibliotecas de produção amplamente usadas.** Representam o
que está em uso real. Exemplos: `rayon`, `hyper`, `reqwest`, `bytes`,
`tracing`.

**Categoria 4 — Crates do próprio projeto-lente.** `lente_core`,
`lente_infra`, `lente_investiga`. Já se sabe que `lente_core` colide
(`ErroRaio::fmt`), serve de baseline. Os outros dois ainda não foram
testados e provavelmente também colidem.

**Critérios para evitar**: crates que dependem de toolchain nightly não
declarada, crates abandonados há mais de 2 anos, crates que não compilam
com a toolchain do fork (rust 1.91, edition 2024). Se um crate falhar ao
compilar ou ao rodar o `cargo modules export-json`, registrar a falha e
substituir.

---

## Procedimento

Para cada crate escolhido:

1. **Obter o código-fonte**: `cargo new`, ou clonar de repositório
   conhecido, ou usar o `Cargo.toml` para baixar via `cargo fetch`. O
   importante é ter o crate compilável localmente.

2. **Rodar o fork**: executar `cargo modules export-json --sysroot --compact`
   (com `--package <nome>` se for workspace, conforme o laudo 0003 ensinou).
   Capturar o JSON.

3. **Detectar colisões**: parsear o JSON e procurar paths que aparecem em
   mais de um nó. Pode ser implementado como código Rust no `lab/` ou
   como script (ex.: `jq` ou Python). A escolha do meio fica com o gerador,
   registrar no relatório.

4. **Para cada colisão**:
   - Extrair do JSON a vizinhança dos nós colidentes (as arestas que
     entram e saem de cada um, baseado no path).
   - Tentar primeiro a Estratégia 1 (vizinhança apenas): chamar
     `lente_investiga::investigar(par, vizinhanca, None)`. Registrar o
     veredito.
   - Se o veredito for `NaoDeterminado`, tentar a Estratégia 2: ler os
     arquivos `.rs` do crate (apenas os que possam conter o `impl` do tipo
     em questão — usar o segmento de path para localizar; se não conseguir
     localizar, ler todos os `.rs` do crate como fallback). Chamar de novo
     `investigar(par, vizinhanca, Some(fontes))`. Registrar o novo veredito.

5. **Coletar para o relatório**:
   - Número de colisões no crate.
   - Para cada colisão: o path colidente, e a sequência de vereditos
     (E1 → E2 se houve).
   - Se `Distintos via fontes`: os traits encontrados (ex.: Display+Debug,
     Display+Clone, etc.).
   - Se `NaoDeterminado` ao final: o diagnóstico completo.

---

## Restrições

- **Não modificar `lente_core`, `lente_infra`, ou `lente_investiga`.** Esta
  é medição contra o estado atual, não desenvolvimento.
- **Não criar `lente_resolve`.** O experimento é justamente para informar a
  construção dele.
- **Não tentar resolver as colisões.** Só detectar e investigar.
- **Tudo vive em `lab/medicao-colisoes/`** (ou caminho equivalente). Não
  contamina os crates do sistema principal. Sem cabeçalho de linhagem
  obrigatório; é Arena.
- **Sem alterações no ADR-0004 ou em outros documentos L0.** Mudanças
  arquiteturais decorrentes da medição são decisão posterior do autor, com
  base no relatório.

---

## Interpretação esperada (referência para o relatório)

O relatório deve incluir uma seção final que apresenta — sem prescrever — o
que os números sugerem sobre cada um dos três cenários do ADR-0004:

- Se a Estratégia 1 (vizinhança) decide a maioria das colisões: a cascata
  justifica-se; a Estratégia 2 fica como fallback raro.
- Se a maioria exige a Estratégia 2 (fontes): a cascata vira só uma
  otimização menor sobre o caminho principal — defensável mas com menor
  ganho que o esperado.
- Se a Estratégia 2 raramente decide (maioria `NaoDeterminado` ao final):
  a arquitetura do ADR-0004 não cumpre sua função, e o ADR precisa de
  revisão.

O relatório descreve o que viu; a decisão sobre revisar o ADR-0004 fica
com o autor.

---

## Observações sobre o método

- **`--package` sempre presente** quando o crate-alvo é workspace
  (descoberta do laudo 0003).
- **`--sysroot` sempre presente** (política da lente, ADR-0001).
- Se algum crate falhar ao processar (erro do fork, JSON inválido, etc.),
  registrar a falha no relatório e seguir com os demais. Falhas no fork
  contra crates reais são, em si, dado útil — uma forma de saber em que
  tipos de crate a lente não consegue operar nem para detectar colisões.
- O experimento pode levar tempo (cada `cargo modules` leva segundos a
  dezenas de segundos). Reportar tempo total no relatório.
- Se aparecer um padrão de colisão novo que o `lente_investiga` não
  cobre (ex.: colisão entre dois `impl` do mesmo trait, ou colisão por
  reexport), descrever esse padrão e registrá-lo como descoberta separada
  do contagem agregada.

---

## Resultado mínimo aceitável

Um relatório em Markdown que permite ao autor do projeto **decidir
informadamente** se segue para o `lente_resolve` como o ADR-0004 desenha, se
revisa o ADR-0004, ou se aceita as colisões como limite.

O relatório não precisa ser exaustivo — precisa ser honesto. Se algum crate
não pôde ser medido por razão técnica, dizer. Se algum padrão é
inconclusivo, dizer. Falsa precisão é pior do que medição clara com
limites declarados.
