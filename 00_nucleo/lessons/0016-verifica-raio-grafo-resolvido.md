# Laudo de Verificação — Prompt 0016 (o raio pode ver grafo não-resolvido?)

**Camada**: L5 (laudo) — verificação de leitura, não construção
**Data**: 2026-05-28
**Prompt executado**: `00_nucleo/prompt/0016-verifica_raio_grafo_resolvido.md`
**Decisões de origem**: laudo 0015 (cascata do descritor fechada); dívida
latente do `raio.rs` operar por path.
**Estado**: `VERIFICADO` — leitura pura, nenhum código alterado.

---

## Propósito

Decidir a natureza da dívida "raio-por-id": **bug ativo** (há caminho que chama
o raio sobre grafo não-resolvido) ou **robustez** (a arquitetura garante que o
raio só vê grafo resolvido). Lendo o código atual, sem modificar nada.

---

## Respostas às quatro perguntas

### 1. Como o raio percorre hoje — por path ou por id?

Por **path**. Em `01_core/src/domain/raio.rs`, os índices internos são todos
chaveados por path:

```rust
struct Indices {
    uses_entrada: HashMap<Path, Vec<Path>>,
    uses_saida:   HashMap<Path, Vec<Path>>,
    owns_pai:     HashMap<Path, Path>,
    owns_filhos:  HashMap<Path, Vec<Path>>,
}
```

As arestas são indexadas por `aresta.from`/`aresta.to` (paths, `raio.rs:118-138`);
o alvo é `&Path` (`raio.rs:153`); o BFS percorre por path. O `id` — presente em
`No`/`Aresta` desde o laudo 0006 — **não é usado em nenhum ponto do cálculo**.

### 2. Há pré-condição declarada de "grafo resolvido"?

**Não.** O doc de `calcular_raio` declara apenas "Erro se o alvo não existir
entre os nós do grafo" (`raio.rs:150-152`). Nenhuma menção a paths únicos ou
grafo resolvido.

### 3. Há caminho que chame o raio sobre grafo não-resolvido?

- **Wiring/composição**: **não existe.** Membros do workspace: `01_core`,
  `03_infra`, `05_investiga`, `06_resolve`. Não há `04_wiring` nem `main.rs`
  (fora de `lab/`). Nada compõe extração → resolução → raio.
- **Chamadores**: `calcular_raio` é invocado **só pelos próprios testes** de
  `raio.rs`. Nenhum outro crate o chama.
- **Testes**: todos constroem grafos com paths literais **únicos**
  (`grafo_com(vec!["A","B","C"], …)`). Nenhum exercita paths colididos.

### 4. A arquitetura garante que o raio só vê grafo resolvido?

**Indeterminado, tendendo a "não garante" — mas sem violação ativa.** O raio
não está conectado a nenhum pipeline. A garantia "resolve→raio" não existe
porque a composição ainda não foi escrita; também não há violação, porque
ninguém chama o raio fora dos testes (todos com paths únicos).

---

## Conclusão: dívida latente, não bug ativo

A dívida raio-por-id é de **robustez/correção conceitual**, não bug em
produção:

- **Hoje não há bug** — o raio só roda em testes, com paths únicos.
- O bug é **latente e condicional**: materializa-se *se* um futuro wiring
  conectar o raio pulando o `lente_resolve`. Aí os índices `HashMap<Path, …>`
  agregariam arestas de cópias colididas sob o mesmo path (BFS conta/perde
  impacto errado), e o alvo `&Path` ficaria ambíguo (qual cópia?).

---

## A lição

**Verificar antes de mudar evitou tratar como urgente o que é latente.** A
dívida do raio-por-id parecia, no fechamento do laudo 0015, um possível bug a
corrigir com prioridade. A leitura mostrou que **não há nem pipeline que o
acione** — então não há bug ativo a corrigir, só uma garantia a desenhar.

Consequência para o tom do próximo passo: a decisão "raio-por-id" **se
entrelaça com o desenho do wiring** (L4, ainda inexistente), e não precisa
precedê-lo:

- Se o wiring garantir `resolve → raio` sempre, o path basta — a colisão nunca
  chega ao raio.
- Se não, ou para robustez independente da ordem de composição, raio-por-id
  fecha o buraco na raiz.

Não é correção urgente; é **decidir a garantia antes que ela possa ser
violada** — o que é melhor feito junto com o wiring, não isolado.

Generalização (candidata ao futuro `LESSONS.md`): *"dívida latente num
componente ainda não-composto não é bug — é uma garantia a ser escrita no
ponto de composição. Verificar quem chama, antes de blindar o chamado."*

---

## Pendências relacionadas (registradas, não resolvidas aqui)

- **Wiring (L4)**: ainda não existe. Quando nascer, define a garantia
  resolve→raio (ou a ausência dela).
- **Raio-por-id**: decisão acoplada ao wiring. Adiável sem risco enquanto não
  houver composição.
- **L2 (mostrar)**: a visualização, primeiro objetivo da proposta, ainda não
  começou.

---

## Histórico de Revisões

| Data | Motivo | Arquivos afetados |
|------|--------|-------------------|
| 2026-05-28 | Verificação de leitura do prompt 0016. Constatado: raio percorre por path, sem pré-condição de grafo resolvido, sem wiring, só chamado por testes (paths únicos). Dívida raio-por-id é latente/condicional, não bug ativo — decisão acoplada ao desenho do wiring. Nenhum código alterado. | (nenhum — verificação) |
