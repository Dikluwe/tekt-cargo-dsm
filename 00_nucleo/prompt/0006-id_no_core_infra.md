# Prompt: Identidade-Por-Nó no `lente_core` e no `lente_infra`

**Camada**: L1 (adição puramente aditiva ao `lente_core`) + L3 (modificação do
`lente_infra` para consumir campos novos do JSON)
**Criado em**: 2026-05-27
**Estado**: `PROPOSTO`
**Decisões de origem**: relatório da medição (`lab/medicao-colisoes/relatorio.md`),
fork atualizado com campo `id` no JSON (commit no fork de Dikluwe/cargo-modules,
`main`), patch da spec da forma organizada (Mudanças 1, 2, 3, 5 já aplicadas
no repositório).
**Pré-requisito**: o fork novo está publicado no GitHub e instalado localmente
(`cargo install --git https://github.com/Dikluwe/cargo-modules cargo-modules
--force`). A invocação `cargo modules export-json --sysroot --compact
--package <nome>` emite agora `id` nos nós e `id_from`/`id_to` nas arestas
(verificado por revisão visual contra a fixture `colliding_paths` do fork).
**Arquivos afetados**: `01_core/src/entities/grafo.rs`, testes existentes em
`01_core/`, `03_infra/src/dto.rs`, `03_infra/src/traducao.rs`, testes do
`lente_infra`.

---

## Contexto

A medição mostrou que a Estratégia 1 do `lente_investiga` (vizinhança no
grafo) é estruturalmente inaplicável ao JSON do fork como ele existia: as
arestas referenciavam paths, e quando havia colisão, a separação por nó era
informação perdida na serialização. O fork foi modificado para preservar a
identidade interna do nó como `id`, e as arestas passaram a carregar
`id_from`/`id_to` referenciando esses ids. A mudança é retrocompatível: `path`,
`from`, `to` continuam intactos.

Agora o projeto-lente precisa **consumir** essa nova identidade. Duas
modificações encadeadas:

1. O `lente_core` recebe os campos novos no tipo de dados (`No.id`,
   `Aresta.id_from`, `Aresta.id_to`). Adição puramente aditiva — nenhum
   campo existente é alterado.
2. O `lente_infra` desserializa os campos novos do JSON e os propaga para o
   `Grafo` construído.

Esta mudança **não toca** o cálculo do raio, nem o `lente_investiga`, nem
nenhum laudo já registrado. Os 55 testes verdes existentes continuam
verdes — a propriedade desejada é não-regressão estrita.

---

## Restrições estruturais

- **`lente_core` permanece puro.** Os tipos novos são `usize` (ou tipo
  numérico apropriado da stdlib). Sem `serde`, sem dependência externa.
  `cargo tree -p lente_core` continua mostrando só o crate.
- **`lente_infra` ganha campos no DTO, não muda função.** A assinatura
  pública do `lente_infra` (`extrair_grafo`) permanece a mesma; muda só o
  conteúdo do `Grafo` retornado (que agora tem ids preenchidos).
- **Retrocompatibilidade do JSON.** Se o JSON for de uma versão antiga do
  fork (sem `id`/`id_from`/`id_to`), o `lente_infra` deve falhar de forma
  diagnosticável, não preencher com zeros silenciosamente. Razão: a presença
  do campo `id` é o que distingue um JSON antigo (ambíguo em colisões) de um
  novo (com identidade preservada). Aceitar JSON antigo silenciosamente
  reintroduziria a ambiguidade que a medição mostrou.

---

## O que mudar no `lente_core`

### Tipo `No`

Adicionar o campo `id: usize` ao struct `No`. Posição: primeiro campo do
struct (antes de `path`), espelhando a posição no JSON.

```rust
pub struct No {
    pub id: usize,           // NOVO
    pub path: Path,
    pub name: String,
    pub kind: Kind,
    pub visibility: Visibility,
    pub crate_name: String,
}
```

### Tipo `Aresta`

Adicionar os campos `id_from: usize` e `id_to: usize` ao struct `Aresta`.
Posição sugerida: junto dos campos `from` e `to` correspondentes, formando
pares.

```rust
pub struct Aresta {
    pub from: Path,
    pub id_from: usize,      // NOVO
    pub to: Path,
    pub id_to: usize,        // NOVO
    pub relation: Relation,
}
```

### Invariante do `Grafo` (atualização do invariante 1)

A spec foi revisada: a unicidade de identidade passa a ser por `id`, não por
`path` (Mudança 3 do patch). Logo:

- **`path` pode repetir** entre nós de um mesmo `Grafo`. Não é mais erro.
- **`id` deve ser único** entre todos os nós de um mesmo `Grafo`.
- **`id_from` e `id_to` de toda aresta** devem corresponder ao `id` de algum
  nó do mesmo `Grafo`.

Esses invariantes não precisam ser verificados por código dentro do
`lente_core` (a verificação é responsabilidade do `lente_infra`, que recebe
dados externos). O `lente_core` apenas modela a forma; verificar é trabalho
de quem materializa.

### Testes existentes no `lente_core`

Os 26 testes existentes precisam ser ajustados para construir nós e arestas
com os novos campos. Sugestão: usar `0`, `1`, `2`, ... como ids nos testes,
de forma sequencial e simples. Não introduzir `id` aleatório nem stub —
testes devem ser deterministicos.

A mudança nos testes é **mecânica**: cada `No { path, name, ... }` vira `No
{ id, path, name, ... }`; cada `Aresta { from, to, relation }` vira `Aresta
{ from, id_from, to, id_to, relation }`.

Cuidado: o **cálculo do raio** (`01_core/src/domain/raio.rs`) constrói grafos
internamente nos testes. Esses testes também precisam ser ajustados. A
operação do cálculo em si **não muda** — ele continua percorrendo arestas
pelo `from`/`to`. (O cálculo pode, num prompt futuro, passar a usar `id` para
ser robusto contra colisões, mas não é este prompt.)

### Não-regressão obrigatória

Depois da mudança, `cargo test -p lente_core` deve mostrar **26/26 testes
verdes** (ou o número atual, ajustado se houver novos). Nenhum teste removido
ou desabilitado para fazer a mudança passar. Se algum teste do `lente_core`
exigia o invariante antigo (path único), ele deve ser **revisto**, não
removido — provavelmente passa a testar `id` único em vez.

---

## O que mudar no `lente_infra`

### DTOs (`03_infra/src/dto.rs`)

Adicionar os campos `id` ao DTO do nó e `id_from`/`id_to` ao DTO da aresta,
usando o serde para parsear como `usize`:

```rust
#[derive(Deserialize)]
struct NodeDto {
    id: usize,               // NOVO
    path: String,
    name: String,
    kind: String,
    visibility: String,
    #[serde(rename = "crate")]
    crate_name: String,
}

#[derive(Deserialize)]
struct EdgeDto {
    from: String,
    id_from: usize,          // NOVO
    to: String,
    id_to: usize,            // NOVO
    relation: String,
}
```

Se um JSON antigo (sem esses campos) for parseado, o serde retornará erro de
desserialização (campo faltando). Isso é o comportamento desejado — JSON
antigo deve falhar visivelmente, não silenciosamente.

### Tradução (`03_infra/src/traducao.rs`)

A conversão DTO → `lente_core` propaga os ids para os tipos correspondentes.
Mecânico — passa o campo, não faz mais nada com ele.

### Invariante a verificar (substituindo ou complementando o invariante 1
antigo)

Hoje o `lente_infra` verifica:

1. Cada `path` em `nodes` é único.
2. Todo `from`/`to` de aresta referencia um `path` existente em `nodes`.

Substituir/adicionar para o seguinte:

1. **Cada `id` em `nodes` é único** (substitui o invariante de path único).
2. Todo `id_from`/`id_to` de aresta referencia um `id` existente em `nodes`.
3. (Opcional, defesa em profundidade) Para cada aresta, o `path` em `from` e
   `to` corresponde ao `path` do nó com o `id` referenciado. Se houver
   discrepância, é bug do fork.

O invariante de path único **deixa de existir**. Path duplicado é agora caso
legítimo (foi a Descoberta 2 do laudo 0003, e a medição confirmou que é
comum).

### Erro `PathDuplicado` no `ErroAdaptador`

A variante `ErroAdaptador::PathDuplicado` que existia no `lente_infra` (que
rejeitou o JSON do `lente_core` na primeira execução do prompt 0003) **deve
ser removida**. Substituí-la por `ErroAdaptador::IdDuplicado` (novo, caso o
JSON do fork esteja malformado emitindo o mesmo `id` duas vezes) e
`ErroAdaptador::IdReferenciado` (novo, caso uma aresta referencie um `id`
inexistente).

A remoção da variante `PathDuplicado` é o sinal mais claro de que a mudança
funcionou: paths duplicados agora são dados normais, não erros.

### Testes do `lente_infra`

Os 13 testes existentes precisam ser revisados:

- Testes que esperavam `PathDuplicado` agora devem testar `IdDuplicado` ou
  `IdReferenciado`, com JSON forjado adequadamente.
- O teste E2E (com `#[ignore]`) que rodava contra a fixture
  `tests/fixtures/crate-amostra/` precisa ser revisitado: a fixture deve
  passar a ter ids no JSON gerado (se ela ainda é gerada com `cargo modules`
  ao vivo, o fork novo já produz os ids; se ela é JSON estático no
  repositório, precisa ser atualizada).
- Adicionar um teste novo: rodar o adaptador contra um crate que **tem
  colisão de path** (a fixture `colliding_paths` que o fork ganhou, ou o
  próprio `lente_core` com `ErroRaio::fmt`). Verificar que o `Grafo`
  resultante tem dois nós com mesmo path e ids distintos, e que as arestas
  envolvidas estão corretas.

Esse último teste é **a verificação real** de que a mudança funciona ponta a
ponta — o crate `lente_core`, que era rejeitado antes, agora deve ser
processado com sucesso.

---

## Critérios de Verificação

```
Dado um JSON do fork novo (com id/id_from/id_to)
Quando extrair_grafo é chamado
Então retorna Ok(Grafo) com nós tendo campo id preenchido e arestas tendo
id_from/id_to preenchidos

Dado um JSON do fork antigo (sem id no nó)
Quando extrair_grafo é chamado
Então retorna Err(ErroAdaptador::JsonInvalido) ou variante similar
(NÃO preenche com zeros silenciosamente)

Dado um JSON com dois nós tendo o mesmo path mas ids diferentes
Quando extrair_grafo é chamado
Então retorna Ok(Grafo) (não é mais erro)
E o Grafo tem dois nós distintos com mesmo path
E os ids são diferentes

Dado um JSON malformado com dois nós tendo o mesmo id
Quando extrair_grafo é chamado
Então retorna Err(ErroAdaptador::IdDuplicado)

Dado um JSON malformado com aresta referenciando id inexistente
Quando extrair_grafo é chamado
Então retorna Err(ErroAdaptador::IdReferenciado)

Dado o crate lente_core (que antes era rejeitado por ErroRaio::fmt)
Quando rodar o pipeline completo (fork → adaptador → Grafo)
Então o Grafo é construído com sucesso, contendo dois nós para ErroRaio::fmt
com ids distintos

Dado todos os testes do lente_core e lente_infra
Quando rodar cargo test
Então todos passam (não-regressão), com ajustes mecânicos para os novos
campos
```

---

## Resultado esperado

- `lente_core` com `id` em `No` e `id_from`/`id_to` em `Aresta`. Os 26
  testes existentes ajustados, todos verdes.
- `lente_infra` com DTOs atualizados, tradução propagando ids, invariantes
  novos (id único, id_from/id_to referenciam id existente). Variante de erro
  `PathDuplicado` removida; `IdDuplicado` e `IdReferenciado` adicionadas.
- Teste novo no `lente_infra` confirmando que crate com colisão de path
  agora é processado com sucesso.
- **Pureza preservada**: `cargo tree -p lente_core` continua mostrando só o
  crate.
- **Não-regressão**: `cargo test` no workspace continua com todos os testes
  verdes (incluindo os 16 do `lente_investiga`, que não são afetados — ele
  recebe dados via parâmetros, não via JSON).
- **Laudo de execução** em `00_nucleo/lessons/` registrando: decisões
  tácitas (como tratou a transição do invariante de path para invariante de
  id; como atualizou a fixture do E2E; o que fez com a variante
  `PathDuplicado`), e qualquer descoberta que apareça na execução.

---

## Observações sobre a sequência de prompts futuros

Este prompt não toca:

- O `lente_investiga`. Ele continua intacto, mas a **partir de agora** pode
  finalmente ser usado de verdade — a Estratégia 1 (vizinhança) deixa de ser
  estruturalmente inerte, porque a vizinhança pode ser separada por nó
  usando `id`. Mas a integração `lente_infra` ↔ `lente_investiga` não é
  feita neste prompt; é prompt futuro.
- O `lente_resolve`. Ainda não existe; depende da remedição (próximo passo
  depois deste prompt) que vai medir quanto a cobertura sobe com a
  identidade-por-nó.
- O ADR-0004. Continua esperando, como você decidiu, até depois da
  remedição.

A sequência depois deste prompt: rodar a medição novamente contra os 17
crates do typst, gerar relatório consolidado, e aí decidir o desenho final
do `lente_resolve` com os números reais na mão.

---

## Histórico de Revisões

| Data | Motivo | Arquivos afetados |
|------|--------|-------------------|
| 2026-05-27 | Criação. Identidade-por-nó propagada do fork para o lente_core e lente_infra. Resolve a Descoberta 2 do laudo 0003. | lente_core/src/entities/grafo.rs, lente_infra/src/dto.rs, lente_infra/src/traducao.rs, testes correspondentes |
