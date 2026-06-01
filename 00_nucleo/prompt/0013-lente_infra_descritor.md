# Prompt: Consumir o Descritor Semântico no `lente_infra`

**Camada**: L3 — Infraestrutura
**Criado em**: 2026-05-28
**Estado**: `PROPOSTO`
**Decisões de origem**: laudo 0012 (lente_core com os campos novos e
Modificadores); ADR-0006 (crate_name como marca de stdlib); prompt do
descritor no fork (fork 0.27.0).
**Pré-requisito**: fork 0.27.0 instalado (`cargo install --git ... --force`);
`lente_core` no estado pós-laudo 0012 (campos novos no No, Kind remodelado,
Modificadores).
**Segundo da cascata a jusante.** Depende do lente_core (laudo 0012); habilita
o lente_investiga (próximo).
**Arquivos afetados**: `03_infra/src/dto.rs`, `03_infra/src/traducao.rs`,
testes do lente_infra; fixtures.

---

## Contexto

O laudo 0012 adicionou ao `lente_core` os campos do descritor (`trait_`,
`trait_ref`, `cfg`, `macro_kind`, `is_non_exhaustive`, `crate_name`,
`modificadores`) e remodelou o `Kind` (tipo base puro; modificadores
separados). Mas a tradução no `lente_infra` ainda preenche esses campos com
**defaults placeholder** (ajuste mecânico mínimo para compilar — laudo 0012
"cascata a jusante").

Este prompt faz a desserialização **real**: o `lente_infra` passa a ler os
campos novos do JSON do fork 0.27.0 e preencher o `No` com valores reais.

---

## Restrições estruturais

- **L3 — admite serde e I/O.** Esta é a camada onde a desserialização mora.
- **Pureza do lente_core preservada.** `cargo tree -p lente_core` continua só
  o crate. Este prompt não toca o lente_core (ele já tem a forma).
- **Aditivo ao DTO, sem quebrar o que existe.** Os campos pré-existentes do
  DTO (id, path, name, kind, visibility, crate) continuam. Os novos são
  acréscimos.

---

## O que mudar

### DTO (`03_infra/src/dto.rs`)

Adicionar ao `NoDTO` os campos novos que o fork 0.27.0 emite:

```rust
#[derive(Deserialize)]
struct NoDTO {
    id: usize,
    path: String,
    name: String,
    kind: String,                      // continua string; o lente_core despe modificadores no TryFrom
    visibility: String,
    #[serde(rename = "crate")]
    crate_name: String,
    // NOVOS:
    #[serde(default)] is_const: bool,
    #[serde(default)] is_async: bool,
    #[serde(default)] is_unsafe: bool,
    #[serde(default)] trait_: Option<String>,      // ver nota de rename abaixo
    #[serde(default)] trait_ref: Option<String>,
    #[serde(default)] cfg: Option<String>,
    #[serde(default)] macro_kind: Option<String>,
    #[serde(default)] is_non_exhaustive: bool,
}
```

Notas importantes:

- **`#[serde(default)]` nos campos novos**: o fork emite `is_const` etc.
  **só quando true** (laudo do fork, decisão A.3). E `trait_`/`trait_ref`/
  `cfg`/`macro_kind` são ausentes quando não aplicam. Então o DTO precisa de
  `default` para esses campos — JSON sem o campo vira `false`/`None`, não erro.
  **Atenção**: isto é diferente do campo `id` (laudo 0006), que NÃO tem
  default (sua ausência é erro, porque distingue fork novo de antigo). Os
  campos do descritor têm default porque sua ausência é normal (nem todo nó
  tem trait, cfg, etc.).
- **Nome do campo trait no JSON**: verificar como o fork 0.27.0 nomeou o campo
  (provavelmente `trait`). Se for `trait` (palavra reservada em Rust como
  identificador de campo), usar `#[serde(rename = "trait")]` no DTO. Verificar
  o JSON real antes de assumir o nome.

### Tradução (`03_infra/src/traducao.rs`)

Substituir os defaults placeholder por preenchimento real:

- `modificadores`: construir `Modificadores { is_const, is_async, is_unsafe }`
  a partir dos **booleanos do DTO** — NÃO parsear da string `kind`. (A
  armadilha das duas fontes, fixada no laudo 0012: a string `kind` mantém os
  modificadores por retrocompat, mas a fonte da verdade são os booleanos.)
- `kind`: continua convertido via `TryFrom<&str>` do `lente_core`, que agora
  despe os modificadores e retorna o tipo base (laudo 0012). A string `kind`
  do JSON (que pode ter modificadores, ex.: `"const fn"`) entra no `TryFrom`,
  que retorna `Kind::Fn`. Os modificadores vêm dos booleanos, não daqui.
- `trait_`, `trait_ref`, `cfg`, `macro_kind`, `is_non_exhaustive`: copiar
  direto do DTO para o `No` (são Option<String>/bool, sem conversão).
- `crate_name`: copiar do DTO (campo `crate` renomeado). Já existia o
  tratamento do rename; manter.

### Invariantes

Os invariantes que o `lente_infra` já verifica (id único, id_from/id_to
referenciam id existente — laudo 0006) continuam. Os campos novos não
adicionam invariante obrigatório (são opcionais por natureza). Não inventar
validação sobre eles.

---

## Critérios de Verificação

```
Dado um JSON do fork 0.27.0 com um nó que tem trait="Display", trait_ref="Display"
Quando extrair_grafo é chamado
Então o No correspondente tem trait_ = Some("Display"), trait_ref = Some("Display")

Dado um nó com is_const=true, is_async=false (e kind "const fn")
Quando traduzido
Então o No tem kind = Kind::Fn E modificadores.is_const = true,
modificadores.is_async = false (modificadores vêm dos booleanos, não da string)

Dado um nó sem campos de descritor (item simples, sem trait/cfg/macro)
Quando traduzido
Então trait_, trait_ref, cfg, macro_kind = None; is_non_exhaustive = false;
modificadores = Default (tudo false)

Dado um nó com crate="core" (stdlib)
Quando traduzido
Então o No tem crate_name = "core" (preparando o filtro de stdlib do ADR-0006)

Dado um JSON do fork ANTIGO (sem os campos do descritor, mas com id)
Quando extrair_grafo é chamado
Então NÃO falha por causa dos campos do descritor (eles têm default);
os campos novos ficam None/false
(Nota: o id continua obrigatório — fork sem id ainda falha, como no laudo 0006)

Dado o crate lente_core extraído com o fork 0.27.0 (--sysroot)
Quando inspecionado o nó ErroRaio::fmt
Então as duas cópias têm trait_ distinto ("Display" e "Debug") — resolvendo a
matéria-prima da D4 (a nomeação em si é o lente_investiga/lente_resolve)
```

Casos a cobrir:
- Nó com descritor completo (trait, modificadores, cfg).
- Nó sem descritor (defaults).
- Modificadores vindo dos booleanos, não da string kind (testar que
  "const fn" + is_const=true dá Kind::Fn + modificador correto).
- crate_name preenchido (stdlib vs crate-alvo).
- Não-regressão: os testes existentes do lente_infra (14 + 2 ignored)
  continuam passando, ajustados para os campos novos.
- E2E: atualizar (ou adicionar) o teste E2E que extrai o lente_core, agora
  verificando que ErroRaio::fmt tem trait_ distinto nas duas cópias.

---

## Resultado esperado

- `NoDTO` com os campos novos (com `#[serde(default)]` nos opcionais).
- Tradução preenchendo o `No` com valores reais (Modificadores dos booleanos).
- Testes ajustados e novos, todos verdes.
- E2E confirmando trait_ distinto no ErroRaio::fmt (com fork 0.27.0 + sysroot).
- **Não-regressão**: workspace inteiro verde; pureza do lente_core preservada.
- **Laudo** em `00_nucleo/lessons/`: como o campo trait foi nomeado no JSON do
  fork (o rename real), decisões sobre defaults, e sinalização para o próximo
  prompt (lente_investiga: agora tem trait_ por nó, pode resolver a D4).

---

## O que NÃO entra (cascata a jusante)

- **lente_investiga**: usar o trait_ por nó para a evidência
  ImplDeTraitsDiferentes vir com o id correto, resolvendo a D4 na raiz.
  Próximo prompt.
- **lente_resolve**: nomear por trait com a precisão que o trait-por-nó
  permite. Prompt posterior.
- **Filtro de stdlib**: usar crate_name (ADR-0006) para esconder ruído.
  Componente futuro, fora desta cascata.
- **Enriquecimento por trait (flag no lente_infra, ADR-0005 Ajuste 3)**: a
  decisão de ligar leitura de fontes para enriquecer nomeação. Como agora o
  trait vem por nó direto do fork, talvez o enriquecimento por fontes nem
  seja mais necessário — avaliar quando chegar no lente_investiga/resolve.

---

## Nota sobre o enriquecimento por trait (importante para a cascata)

O ADR-0005 (Ajuste 3) previa que o trait viria de uma E2 que lê fontes,
ligada por flag no lente_infra. Mas o fork 0.27.0 agora emite o trait **por
nó, direto** — sem precisar ler fontes. Isso pode tornar a E2 (parser textual
de fontes) e o enriquecimento por flag **obsoletos** para o caso de trait: o
trait vem de graça no JSON, com o id correto associado.

Não resolver isso neste prompt — mas registrar a observação para quando
chegarmos no lente_investiga/lente_resolve. Pode ser que a D4 se resolva
trivialmente (trait vem por nó, id correto, sem adivinhação) e que a E2
inteira possa ser aposentada. A medição/integração dirá.

---

## Histórico de Revisões

| Data | Motivo | Arquivos afetados |
|------|--------|-------------------|
| 2026-05-28 | Desserialização real dos campos do descritor no lente_infra. Modificadores dos booleanos. Campos opcionais com serde default. crate_name preenchido. Segundo da cascata do descritor. | 03_infra/src/dto.rs, 03_infra/src/traducao.rs |
