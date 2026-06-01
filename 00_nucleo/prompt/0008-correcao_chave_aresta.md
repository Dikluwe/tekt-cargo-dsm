# Prompt: Corrigir `ChaveAresta` em `lente_investiga` para Usar Ids

**Camada**: L1 — correção pontual no `lente_investiga`
**Criado em**: 2026-05-27
**Estado**: `PROPOSTO`
**Decisões de origem**: laudo da remedição (`lab/medicao-colisoes/remedicao/relatorio.md`,
§6 — Descoberta crítica), laudo 0006 (identidade-por-nó no `lente_core` e
`lente_infra`).
**Pré-requisito**: `lente_core` com `Aresta { id_from, id_to, ... }` (já
existe desde laudo 0006). Estado do projeto pós-laudo 0006.
**Arquivos afetados**: `05_investiga/src/vizinhanca.rs` (correção), e algum
teste novo (localização a decidir pelo gerador — provavelmente no próprio
`vizinhanca.rs` ou em `lib.rs`).

---

## Contexto

A remedição (`lab/medicao-colisoes/remedicao/relatorio.md`) descobriu que a
Estratégia 1 do `lente_investiga` (vizinhança no grafo) está classificando
**falsos `MesmoItem`** em casos clássicos de colisão como `Display+Debug`.

A causa raiz foi identificada na §6 do relatório: o struct `ChaveAresta`
em `05_investiga/src/vizinhanca.rs` usa apenas `(from, to, relation)` como
chave de comparação. Foi escrito no prompt do `lente_investiga` (laudo 0004)
**antes da identidade-por-nó existir**. O prompt 0006, que adicionou
`id_from`/`id_to` em `Aresta` do `lente_core` e fez o `lente_infra`
consumi-los, **não atualizou** o `lente_investiga`. Os 16 testes do
`lente_investiga` continuaram passando porque nenhum deles exercita o caso
de dois nós com mesmo path e ids distintos.

Consequência: para `impl Display for X { fn fmt }` e `impl Debug for X
{ fn fmt }`, as duas arestas `Owns from "X" to "X::fmt"` viram **a mesma
chave** (mesmos paths, mesma relação), e a vizinhança parece idêntica nos
dois nós colidentes — veredito `MesmoItem`, errado.

A remedição estimou que pelo menos 25% dos 116 vereditos "E1 decide" são
contaminados por esse bug. Corrigir o `ChaveAresta` é pré-requisito para a
próxima rodada de medição.

---

## Restrições estruturais

- **Camada L1 — pureza absoluta.** Zero I/O, zero dependências externas.
  `cargo tree -p lente_investiga` continua mostrando só o crate +
  `lente_core` (workspace).
- **Mudança mínima.** Só o que o bug exige: trocar a forma da chave e
  garantir que o crítério categórico de vizinhança continue funcionando
  com a nova chave. **Não tocar** no critério de `disjuntas`/`idênticas`/
  `inconclusivo` nesta correção (decisão de design a ser tomada com base
  na próxima medição, ver laudo da remedição §8).
- **Não-regressão**: os 16 testes existentes do `lente_investiga` continuam
  passando.

---

## O que mudar

### Em `05_investiga/src/vizinhanca.rs`

Substituir a struct `ChaveAresta` para usar `(id_from, id_to, relation)`
em vez de `(from, to, relation)`:

```rust
// Antes
struct ChaveAresta {
    from: String,
    to: String,
    relation: Relation,
}

// Depois
struct ChaveAresta {
    id_from: usize,
    id_to: usize,
    relation: Relation,
}
```

Ajustar a construção da chave a partir das arestas:

```rust
// Antes
ChaveAresta {
    from: aresta.from.as_str().to_string(),
    to: aresta.to.as_str().to_string(),
    relation: aresta.relation,
}

// Depois
ChaveAresta {
    id_from: aresta.id_from,
    id_to: aresta.id_to,
    relation: aresta.relation,
}
```

(A sintaxe exata depende de como o struct `Aresta` do `lente_core` expõe os
campos. Verificar antes de gerar.)

Tudo mais em `vizinhanca.rs` permanece igual: a lógica de "disjuntas /
idênticas / inconclusivo", a comparação de `HashSet<ChaveAresta>`, o
veredito retornado. A mudança é puramente sobre **o que conta como mesma
aresta** — e agora arestas que apontam para cópias distintas do mesmo path
(via ids diferentes) deixam de ser confundidas.

### Não mudar

- O critério categórico (`compartilhadas == 0` etc.) permanece como está
  — a decisão sobre relaxar fica para depois.
- Os tipos públicos do `lente_investiga` (`Vizinhanca`, `ArestasNo`,
  `Veredito`, etc.) não mudam.
- A função `investigar` mantém a mesma assinatura.
- Os outros módulos (`fontes.rs`, `lib.rs`) não são afetados.

---

## Teste novo

Adicionar **um teste** no `lente_investiga` (sugestão: no módulo
`vizinhanca` em `tests`) que exercita exatamente o caso `Display+Debug`:

**Cenário do teste**: dois nós com o mesmo `path` (ex.: `"X::fmt"`) e ids
distintos (ex.: 100 e 101). O grafo tem arestas:

- `Owns from "X" to "X::fmt"` com `id_from=42, id_to=100` (aponta para
  cópia A).
- `Owns from "X" to "X::fmt"` com `id_from=42, id_to=101` (aponta para
  cópia B).

A vizinhança de cada nó colidente deve conter **só a aresta que aponta
para o `id` daquele nó** (separação por id).

**Resultado esperado**: `Veredito::Distintos { evidencia:
VizinhancaDisjunta }`, porque cada cópia tem uma aresta exclusiva e nenhuma
compartilhada (cada cópia recebe uma aresta diferente, mesmo que ambas
tenham o mesmo `from` em string).

**Por que esse teste é crítico**: ele **falharia** com o `ChaveAresta`
antigo (as duas arestas viram a mesma chave, vizinhança parece idêntica,
veredito vira `MesmoItem`). Ele **passa** com o `ChaveAresta` novo (as
arestas têm `id_to` distintos, viram chaves distintas, vizinhança é
disjunta, veredito é `Distintos`).

Esse teste é a salvaguarda contra regressão deste bug, e também demonstra
no código a propriedade que a mudança garante.

Nomeação sugerida do teste:
`vizinhancas_de_copias_distintas_decidem_distintos` (ou equivalente).

### Não substituir testes existentes

Os 16 testes atuais continuam, intactos. Eles exercitam casos onde os ids
não estavam disponíveis (sem colisão real). Continuam válidos — agora
testam o caso "sem ids relevantes" que continua funcionando com a chave
nova (a chave usa ids, mas paths distintos têm ids distintos, então o
comportamento é equivalente para casos sem colisão).

---

## Critérios de verificação

```
Dado o struct ChaveAresta atualizado
Quando duas arestas têm mesmo from-path e mesmo to-path mas id_to diferentes
Então elas viram chaves distintas (HashSet as conta como duas)

Dado o cenário do teste novo (dois fm com ids distintos, arestas separadas)
Quando investigar é chamado
Então retorna Veredito::Distintos { evidencia: VizinhancaDisjunta }

Dado os 16 testes existentes do lente_investiga
Quando rodar cargo test -p lente_investiga
Então todos passam (não-regressão)

Dado o teste novo
Quando rodar cargo test -p lente_investiga
Então também passa
```

Total esperado: **17 testes verdes** no `lente_investiga` (16 + 1).

---

## Resultado esperado

- `05_investiga/src/vizinhanca.rs` com `ChaveAresta` corrigido.
- Teste novo adicionado (no mesmo arquivo ou em `lib.rs`, conforme
  organização atual do crate).
- `cargo build` limpo, `cargo test -p lente_investiga` com 17/17 verdes.
- `cargo tree -p lente_investiga` continua mostrando só `lente_core`
  (pureza preservada).
- **Não-regressão no resto do workspace**: `cargo test` rodando todos os
  crates continua com todos os testes verdes.
- **Laudo de execução** em `00_nucleo/lessons/` registrando: a mudança
  feita, o teste novo adicionado, qualquer decisão tácita sobre detalhes
  (ex.: se foi necessário ajustar imports, se houve algum outro lugar que
  usava a forma antiga de `ChaveAresta` etc.).

---

## Observações sobre o que NÃO entra neste prompt

Por decisão do autor, ficam para depois:

- **Relaxar o critério de "disjuntas"** (de `compartilhadas == 0` para
  "ambos com exclusivas"). É decisão de design separada, dependente da
  próxima rodada de medição com a chave corrigida.
- **Atualizar o ADR-0004**. Continua esperando até depois da próxima
  rodada de medição.
- **Construir o `lente_resolve`**. Continua esperando.
- **Re-rodar a remedição**. É próximo passo, mas depois deste prompt
  estar executado.

---

## Histórico de Revisões

| Data | Motivo | Arquivos afetados |
|------|--------|-------------------|
| 2026-05-27 | Criação. Corrige bug identificado na §6 do laudo da remedição: ChaveAresta passa a usar (id_from, id_to, relation) em vez de (from, to, relation). Pré-requisito para a próxima rodada de medição. | 05_investiga/src/vizinhanca.rs, teste novo |
