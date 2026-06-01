# Prompt: Composição do Pipeline (`lente_wiring`, L4)

**Camada**: L4 — Fiação (composição, sem lógica de negócio)
**Criado em**: 2026-06-01 (ajustado conforme laudos 0017/0018)
**Estado**: `PROPOSTO`
**Decisões de origem**: laudo 0016 (projeto não está composto); decisão do
autor (L4 só compõe, L2 cuida de mostrar; erros nas camadas onde nascem, L4
unifica num enum ErroLente; L4 consome via função de alto nível, sem lidar
com DTO/serde).
**Segundo de três prompts da composição** (L3 invocador consolidado →
**L4 wiring** → L2 CLI).
**Pré-requisito**: laudo 0018 (consolidação `invocacao.rs` + `fork.rs`);
cascata do descritor completa (laudo 0015).
**Arquivos afetados**: `03_infra/src/lib.rs` (expor `desserializar_grafo`);
novo crate `04_wiring/` no workspace.

---

## Contexto

Até agora, cada crate (`lente_core`, `lente_infra`, `lente_investiga`,
`lente_resolve`) existe e é testado isoladamente. Nada compõe o pipeline
completo. Este prompt cria o L4 que faz essa composição — pela primeira vez
o sistema vai rodar como um todo.

A decisão do autor (separação L4/L2): o L4 só compõe e devolve estrutura
pura. Não formata, não escreve em stdout, não lida com argumentos. Recebe
entrada (JSON ou nome de pacote) e devolve um `Raio`, ou um `ErroLente` que
embrulha os erros das camadas internas.

A decisão sobre interface com o `lente_infra` (mensagem após o laudo 0018):
o L4 não lida com DTO/serde. O `lente_infra` esconde esses detalhes
expondo uma função de alto nível.

---

## Restrições estruturais

- **L4 — só composição.** Zero lógica de negócio.
- **Importa todas as camadas numeradas necessárias**: `lente_core`,
  `lente_infra`, `lente_investiga`, `lente_resolve`. Não importa `lab/`.
- **Erros embrulhados, não recriados.** O `ErroLente` (definido em L4)
  agrupa via variantes; usa `From` impls para uso natural com `?` (apesar da
  D2 do laudo 0018 preferir mapeamento explícito em alguns lugares, em L4
  o `From` é apropriado — a composição precisa propagar muitos erros, e
  forçar mapeamento explícito em todo `?` é ruído).
- **Não toca `lente_core`, `lente_investiga`, `lente_resolve`.** Só o
  `lente_infra` ganha exposição nova (parte 1 deste prompt). E o crate novo
  `lente_wiring` (parte 2).

---

## Parte 1 — Expor `desserializar_grafo` no `lente_infra`

Pré-requisito para a Parte 2. Sem isso o L4 não pode desserializar JSON
sem invadir a `pub(crate) traducao::traduzir`.

### Função pública nova

Em `03_infra/src/lib.rs` (ou módulo conforme a estrutura), adicionar:

```rust
/// Desserializa um JSON do fork (cargo-modules export-json 0.27.0) num Grafo.
///
/// Encapsula a leitura do DTO via serde_json e a tradução para o tipo do
/// lente_core. O chamador (L4) não precisa lidar com serde nem com o
/// formato do JSON.
pub fn desserializar_grafo(json: &str) -> Result<Grafo, ErroAdaptador> {
    let dto: GrafoDTO = serde_json::from_str(json).map_err(/* variante apropriada */);
    traducao::traduzir(dto)  // ou método equivalente
}
```

Detalhes:
- Reaproveita o `GrafoDTO` que já existe (laudos 0006/0013), não duplica.
- O erro de `serde_json::from_str` deve cair numa variante de `ErroAdaptador`
  (provavelmente já existe — `ErroAdaptador::JsonInvalido` ou similar; se
  não existir, adicionar).
- A `traducao::traduzir` continua `pub(crate)` — esta função pública é a
  fachada limpa.

### Testes para a nova função

Pelo menos:
- JSON válido (use uma fixture existente do projeto) → `Ok(Grafo)`.
- JSON inválido (string `"{"`) → `Err(ErroAdaptador::variante_de_json)`.

Não-regressão: os testes existentes do `lente_infra` continuam passando
(a `traducao::traduzir` interna não muda).

---

## Parte 2 — Novo crate `lente_wiring` em `04_wiring/`

### Estrutura

- `04_wiring/Cargo.toml`: dependências por path para `lente_core`,
  `lente_infra`, `lente_investiga`, `lente_resolve`. Sem deps externas
  (composição pura).
- `04_wiring/src/lib.rs`: a função pública de composição e o tipo
  `ErroLente`.

### Função pública principal

```rust
/// Compõe o pipeline completo: obtém o JSON (do arquivo ou via fork),
/// desserializa, detecta colisões de path, investiga e resolve cada uma,
/// e calcula o raio do alvo.
pub fn calcular_raio_de_alvo(
    fonte: FonteGrafo,
    alvo: AlvoBusca,
) -> Result<Raio, ErroLente>
```

### Tipos auxiliares

```rust
/// De onde vem o grafo.
pub enum FonteGrafo {
    /// JSON pronto (o L2 leu de arquivo ou stdin).
    Json(String),
    /// Nome de pacote — o wiring invoca o fork via lente_infra::fork.
    Pacote(String),
}

/// Como o alvo é apontado.
pub enum AlvoBusca {
    PorPath(Path),
    PorId(usize),
}
```

### Fluxo de `calcular_raio_de_alvo` (passo a passo)

1. **Obter o JSON**:
   - `FonteGrafo::Json(s)` → usa `s` direto.
   - `FonteGrafo::Pacote(p)` → chama `lente_infra::fork::invocar_fork(&p)`,
     que devolve `Result<String, ErroFork>`. Erro é convertido para
     `ErroLente::Fork(...)` via `?`/`From`.

2. **Desserializar**: `lente_infra::desserializar_grafo(&json)`. Erro vira
   `ErroLente::Adaptador(...)`.

3. **Detectar colisões**: percorrer os nós do grafo, agrupar por `path`.
   Paths com 2+ nós são colisões. Helper interno do `lente_wiring` (é
   orquestração — "para cada path com cópias, faça X"). Não é lógica de
   negócio.

4. **Para cada colisão, para cada par de cópias** (primeiro par, conforme
   3ª medição):
   - Chamar `lente_investiga::investigar(par, fontes_vazias)`. O parâmetro
     `fontes` é passado **vazio** — a E2 está em quarentena (laudo 0014),
     o trait vem por nó, fontes não são consultadas. (Conforme decisão
     pós-quarentena: passar `Default::default()` ou `&[]` no parâmetro
     herdado, ver D1 do laudo 0014.)
   - Chamar `lente_resolve::aplicar(grafo, path_colidido, &veredito)`. O
     grafo de saída é o próximo grafo de trabalho.

5. **Resolver o alvo**:
   - `AlvoBusca::PorPath(p)` → usar direto.
   - `AlvoBusca::PorId(id)` → procurar o nó com aquele id no grafo
     resolvido, usar seu `path`. Se não existir, `ErroLente::IdInexistente(id)`.

6. **Calcular o raio**: `lente_core::domain::raio::calcular_raio(&grafo,
   &path_alvo)`. Erro vira `ErroLente::Raio(...)`.

7. **Devolver `Raio`**.

### Tipo `ErroLente`

```rust
pub enum ErroLente {
    /// Falha ao invocar o fork (subprocess).
    Fork(lente_infra::fork::ErroFork),
    /// Falha ao desserializar ou outra falha do lente_infra.
    Adaptador(lente_infra::ErroAdaptador),
    /// Falha em uma resolução de colisão.
    Resolucao(lente_resolve::ErroResolve),
    /// Falha no cálculo do raio (ex.: alvo não existe).
    Raio(lente_core::ErroRaio),
    /// Alvo por id que não existe no grafo.
    IdInexistente(usize),
}
```

`impl From<...> for ErroLente` para cada variante interna (permite `?`).
`impl Display` para texto humano-legível (que o L2 usa). `impl
std::error::Error` para integração com ferramentas.

### `main()` não vive aqui

Este crate entrega **só a lib**: a função pública e o tipo de erro. O
`main()` (e o parser de args, o formatador) é do L2 CLI (terceiro prompt).
Razão: testar o L4 sem CLI exige a lib pura.

---

## Critérios de Verificação

```
Dado um JSON do lente_core (com colisões reais — ErroRaio::fmt)
Quando calcular_raio_de_alvo(FonteGrafo::Json(json), AlvoBusca::PorPath("ErroRaio"))
Então retorna Ok(Raio) — o pipeline ponta a ponta rodou; raio coerente

Dado o mesmo JSON, AlvoBusca::PorId(<id real do nó ErroRaio>)
Quando calcular_raio_de_alvo
Então retorna Ok(Raio) — caminho por id também funciona

Dado um pacote válido do próprio workspace
Quando calcular_raio_de_alvo(FonteGrafo::Pacote("lente_core"), AlvoBusca::PorPath("ErroRaio"))
Então invoca o fork, desserializa, resolve colisões, calcula o raio. Ok(Raio).
(Pode ser #[ignore] se o ambiente de CI não tiver fork — mesmo padrão do
lente_infra::fork.)

Dado um id que não existe no grafo
Quando calcular_raio_de_alvo
Então retorna Err(ErroLente::IdInexistente(...))

Dado um JSON inválido
Quando calcular_raio_de_alvo
Então retorna Err(ErroLente::Adaptador(...))

VERIFICAÇÃO CRUCIAL — resolução de colisões ponta a ponta:
Dado o JSON do lente_core (que tem ErroRaio::fmt como path colidente)
Quando o pipeline compõe extrair → resolver
Então o grafo final NÃO TEM mais "ErroRaio::fmt" como path —
o resolve renomeou para "ErroRaio::<Display>::fmt" e "ErroRaio::<Debug>::fmt".
Este teste é o primeiro ponto onde se prova que a cascata do descritor
inteira (laudos 0012-0015) funciona quando composta, não só isolada.
```

Casos a cobrir:
- Pipeline ponta a ponta com `FonteGrafo::Json`.
- Pipeline ponta a ponta com `FonteGrafo::Pacote` (pode ser `#[ignore]`).
- Alvo por path e por id.
- Cada variante de erro num teste smoke.
- **A verificação crucial**: a renomeação por trait acontece de fato na
  composição.

---

## Resultado esperado

- Parte 1: `lente_infra::desserializar_grafo` exposta, testada,
  retrocompatível (testes existentes inalterados).
- Parte 2: crate `lente_wiring` em `04_wiring/`, com `calcular_raio_de_alvo`,
  `FonteGrafo`, `AlvoBusca`, `ErroLente` (+ `From`/`Display`/`Error` impls).
- Testes inline do `lente_wiring` cobrindo o pipeline completo — o **primeiro
  ponto onde o sistema inteiro roda junto**.
- **Workspace verde**: pré-existentes + novos.
- **Pureza**: `cargo tree -p lente_core` ainda só o crate; `cargo tree -p
  lente_wiring` mostra os quatro componentes + suas deps (esperado).
- **Laudo** registrando:
  - Como `desserializar_grafo` foi montada (variante de erro do serde, etc.).
  - Como a detecção de colisões foi implementada (helper interno).
  - Como a iteração `investigar → resolver` é feita quando há várias
    colisões (ordem? grafo acumulado?).
  - O **resultado real da verificação crucial**: confirmação de que
    `ErroRaio::fmt` não existe mais no grafo final, e que
    `ErroRaio::<Display>::fmt` / `ErroRaio::<Debug>::fmt` existem.
  - Qualquer ajuste descoberto na composição (a observação metodológica
    abaixo).

---

## Observação metodológica (importante)

Este é o primeiro prompt onde o sistema **roda como um todo**. Até agora,
cada peça foi testada isoladamente — e isso pode esconder problemas de
composição: assinaturas que precisam de pequenos casts; tipos que não
casam num detalhe; suposições implícitas que cada peça tinha sobre quem
chama; tratamento de bordas que cada peça delegava ao próximo na cadeia.

Esses ajustes são **descobertas, não erros**. O laudo deve registrá-los
cuidadosamente, porque cada um é informação sobre o sistema real.

**Regra dura**: se um ajuste exigir mudar o **contrato** de alguma camada
(assinatura pública, tipo de erro exposto, semântica de retorno), o
gerador **deve parar e relatar** em vez de mudar silenciosamente. Mudança
de contrato é decisão do autor. Ajustes internos (renomear variável,
adicionar helper interno, etc.) são ok.

---

## O que NÃO entra

- **L2 CLI**: parsing de args (clap), formatação, stdout/stderr, `main()`.
  Próximo prompt.
- **Modos ranking e agregado**: só modo focado neste prompt.
- **Filtro de stdlib**: marcar/esconder nós de stdlib. Componente futuro.

---

## Histórico de Revisões

| Data | Motivo | Arquivos afetados |
|------|--------|-------------------|
| 2026-06-01 | L4 wiring (e parte 1 no lente_infra). Parte 1: expor desserializar_grafo. Parte 2: crate lente_wiring com calcular_raio_de_alvo, FonteGrafo, AlvoBusca, ErroLente. Primeira composição do sistema completo. Verificação crucial: a cascata do descritor funciona quando composta (ErroRaio::fmt vira <Display>/<Debug>). | 03_infra/src/lib.rs (parte 1), 04_wiring/ (novo crate) |
