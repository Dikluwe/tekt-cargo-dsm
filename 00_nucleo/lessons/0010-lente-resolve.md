# Laudo de Execução — Prompt 0010 (lente_resolve)

**Camada**: L5 (laudo)
**Data**: 2026-05-27
**Prompt executado**: `00_nucleo/prompt/0010-lente_resolve-v2.md`
**ADRs**: 0004 (cascata de resolução), 0005 (validação pós-medição)
**Depende de**: `lente_core` (No.id, Aresta.id_from/id_to, Veredito). **Não**
depende de `lente_investiga`.
**Estado**: `EXECUTADO` — crate criado, 9 testes verdes, pureza preservada,
workspace inteiro sem regressão (66 verdes + 2 ignored).

---

## O que o prompt pediu

Segundo (e último) componente do mecanismo de resolução do ADR-0004: o
`lente_resolve` **aplica** no grafo o veredito que o `lente_investiga`
produziu. Conforme ADR-0005: nomeação por contador (ordem de id) como
padrão, trait opcional; caminho `Distintos` comum, `MesmoItem` por
completude; redistribuição determinística por id (sem
`RedistribuicaoIndeterminada`).

---

## O que foi gerado

| Arquivo | Conteúdo |
|---------|----------|
| `Cargo.toml` (raiz) | `members` += `"06_resolve"`. |
| `06_resolve/Cargo.toml` | Crate `lente_resolve`, dep única `lente_core`. |
| `06_resolve/src/lib.rs` | `aplicar(grafo, colisao, veredito)`, enum `ErroResolve`, lógica por variante de veredito, 9 testes inline. |

**Verificação**:
- `cargo build` — limpo.
- `cargo test -p lente_resolve` — **9/9** verdes.
- `cargo test` (workspace) — **66 verdes + 2 ignored** (core 26, infra 14+2, investiga 17, resolve 9). Não-regressão total.
- `cargo tree -p lente_resolve` — só `lente_core`. **Pureza L1 preservada.**

---

## Decisões tácitas

### D1 — Diretório `06_resolve/`

Segue a convenção `NN_nome/` (`01_core`, `03_infra`, `05_investiga`).
Próximo número livre.

### D2 — `Path` aceita `#` e `<>` sem mudança no `lente_core`

O prompt pedia para verificar antes de assumir. Verificado: `Path` é
newtype `Path(String)` **sem validação** (`From<&str>`/`From<String>` só
embrulham). Os nomes novos (`M::T::fmt#1`, `M::T::<Display>::fmt`) são
aceitos sem tocar o `lente_core`. Nenhum relaxamento necessário.

### D3 — Nomeação por trait só com `ImplDeTraitsDiferentes` E exatamente 2 cópias

A evidência `ImplDeTraitsDiferentes` carrega exatamente **dois** traits.
Quando há 2 cópias, mapeia uma para cada. Com 3+ cópias (ou evidência
`VizinhancaDisjunta`), cai no contador — não há traits suficientes para
nomear todas. Coerente com o ADR-0005 (contador é o piso; trait é upgrade
quando disponível).

### D4 — Correspondência id↔trait por ordem de id (limitação registrada)

A evidência `ImplDeTraitsDiferentes { traits: (t0, t1) }` **não diz qual
id corresponde a qual trait**. Atribuo `t0` ao menor id e `t1` ao maior,
de forma determinística. É arbitrário, mas estável. Consequência: o nome
`M::T::<Display>::fmt` pode, no limite, estar trocado com
`M::T::<Debug>::fmt` se o fork emitir os ids em ordem inversa à ordem dos
impls no fonte. Como o caso `Display+Debug` é raro (9 de 384, ADR-0005) e
o enriquecimento por trait é opcional/desligado por padrão, a limitação é
tolerável. Resolver de verdade exigiria a evidência carregar o id de cada
trait — evolução futura do `lente_investiga`.

### D5 — `MesmoItem` preserva campos do menor id silenciosamente

Quando cópias divergem em `name`/`kind`/`visibility`, o nó canônico (menor
id) prevalece, sem aviso. Razão: L1 é puro — não há canal de log, e a
função retorna `Grafo`, não um par `(Grafo, Vec<Aviso>)`. Adicionar canal
de aviso seria complexidade para um caso que a medição mostrou ocorrer
**0 vezes** em 384 colisões (ADR-0005 Ajuste 4). Se algum dia
`MesmoItem` com divergência aparecer e importar, um campo de aviso no
retorno é mudança localizada.

### D6 — `MesmoItem` deduplica arestas idênticas resultantes

Após redirecionar as referências das cópias para o id canônico, duas
arestas podem ficar idênticas (ex.: dois usuários que apontavam para
cópias distintas, mas que após unificação apontam para o mesmo nó com a
mesma relação). Dedup por chave `(id_from, id_to, relation)`. Sem dedup, o
grafo teria arestas redundantes. Testado (`mesmo_item_dedup_arestas_identicas`).

### D7 — `IdInconsistente` mantida sem caminho ativo

O prompt pede "ao menos" `ColisaoNaoResolvida`, `ColisaoInexistente`,
`IdInconsistente`. As duas primeiras têm caminho claro. `IdInconsistente`
seria para "evidência referencia ids que não correspondem aos nós
colidentes" — mas **nenhuma evidência atual carrega ids**
(`VizinhancaDisjunta` tem contagens; `ImplDeTraitsDiferentes` tem strings
de trait). Logo, não há caminho que a dispare hoje. Mantida por contrato
do prompt e como defesa para evidências futuras que venham a carregar ids.

### D8 — `RedistribuicaoIndeterminada` não existe

Conforme ADR-0005 / prompt: a identidade-por-nó tornou a redistribuição de
arestas sempre determinística (cada aresta sabe a qual cópia pertence pelo
`id_from`/`id_to`). A variante de erro do desenho original (laudo 0004) foi
omitida.

### D9 — Formato dos paths novos

- Contador: `format!("{}#{}", path, i+1)` → `M::T::fmt#1`, `M::T::fmt#2`.
- Trait: insere `<Trait>` antes do último segmento via `rsplit_once("::")`
  → `M::T::<Display>::fmt`. Se o path não tem `::` (raro), usa
  `<Trait>::path`.

---

## Propriedade central garantida: redistribuição determinística

A redistribuição de arestas funciona porque cada aresta carrega
`id_from`/`id_to` (desde o laudo 0006). Para os nós renomeados:

- Aresta com `id_to == 1` → seu `to`-path vira o novo path do nó id 1.
- Aresta com `id_to == 2` → seu `to`-path vira o novo path do nó id 2.

Sem ambiguidade, sem heurística. É a propriedade que o desenho original
(laudo 0004) não tinha — antes, com arestas referenciando só paths, era
impossível saber a qual cópia uma aresta pertencia. Testado em
`distintos_contador_renomeia_e_redistribui` (verifica que a aresta de
`id_to=1` aponta para `#1` e a de `id_to=2` para `#2`).

---

## Critérios de Verificação atendidos

| Critério (do prompt) | Status | Teste |
|----------------------|--------|-------|
| Distintos/VizinhancaDisjunta → nós `X#1`/`X#2`, arestas redistribuídas por id, sem colisão de path | ✓ | `distintos_contador_renomeia_e_redistribui` |
| Distintos/ImplDeTraitsDiferentes → nós `X::<Display>::...`/`X::<Debug>::...` | ✓ | `distintos_com_trait_nomeia_por_trait` |
| MesmoItem → um nó, arestas de ambos apontam para ele | ✓ | `mesmo_item_unifica` |
| NaoDeterminado → `Err(ColisaoNaoResolvida)`, grafo intacto | ✓ | `nao_determinado_propaga_erro_sem_modificar` |
| Path sem colisão → `Err(ColisaoInexistente)` | ✓ | `path_sem_colisao_e_erro` |
| 3+ cópias → contador #1/#2/#3 | ✓ | `distintos_tres_copias_usa_contador_mesmo_com_trait` |
| Determinismo (aplicar 2× = mesmo resultado) | ✓ | `determinismo_aplicar_duas_vezes_da_mesmo_resultado` |
| Invariantes do grafo de saída (ids únicos, paths únicos, integridade ref.) | ✓ | `checar_invariantes` + `paths_unicos` em cada teste de sucesso |
| Grafo de entrada não modificado | ✓ | `grafo_original_nao_e_modificado` |
| Dedup de arestas no MesmoItem | ✓ | `mesmo_item_dedup_arestas_identicas` |

---

## Crates e dependências (gravidade Tekt preservada)

```
lente_core (L1, sem deps externas)
  ↑
  ├── lente_infra     (L3: serde, serde_json)
  ├── lente_investiga (L1: zero deps externas)
  └── lente_resolve   (L1: zero deps externas) ← NOVO
```

`lente_resolve` depende **só de `lente_core`** — não importa
`lente_investiga` (recebe o `Veredito` pronto, não o produz). Coerente com
ADR-0005 (os dois L1 se comunicam pelo `Veredito` que mora no `lente_core`).

---

## O que NÃO entra neste prompt

- **Integração `lente_infra` ↔ investiga/resolve.** Adaptar o L3 para
  detectar colisões, chamar investiga, chamar resolve, e (opcionalmente)
  ligar o enriquecimento por fontes — é prompt futuro (ADR-0005 Ajuste 3).
- **Atualizar a spec com o Limite 6.** O ADR-0005 §Ajuste 5 declara que
  `forma-organizada.md` recebe o Limite 6 (colisões em código gerado por
  macro). Este prompt não toca a spec — é trabalho à parte. O
  `lente_resolve` já trata o caso: `NaoDeterminado` → `ColisaoNaoResolvida`,
  propagando o diagnóstico.
- **Enriquecimento por trait ligado.** O `lente_resolve` reage à evidência
  que recebe (se tem trait, usa; senão, contador). Quem decide ligar o
  enriquecimento (ler fontes, acionar E2) é o `lente_infra` — não este crate.

---

## Próximos passos (referência, não compromisso)

1. **Integração no `lente_infra`** — fechar a pipeline: fork → adaptador →
   detectar colisões → investiga → resolve → grafo resolvido. Inclui a flag
   de enriquecimento (ADR-0005 Ajuste 3).
2. **Limite 6 na spec** — registrar formalmente os casos de macro.
3. **Cálculo do raio sobre grafo resolvido** — verificar que o `raio.rs`
   (L1) opera corretamente sobre um grafo já resolvido (paths únicos).
4. **Generalização** — medir contra crates de outras origens (reexports
   podem finalmente exercer o caminho `MesmoItem`, que aqui ficou só
   testado por grafo forjado).

---

## Histórico de Revisões

| Data | Motivo | Arquivos afetados |
|------|--------|-------------------|
| 2026-05-27 | Criação do `lente_resolve` conforme ADR-0005. Nomeação por contador (ordem de id) padrão; por trait quando evidência traz e há 2 cópias. Redistribuição determinística via id. `MesmoItem` por completude (dedup de arestas). Sem `RedistribuicaoIndeterminada`. 9 testes verdes; workspace 66 verdes + 2 ignored; pureza L1 preservada. | `Cargo.toml` (raiz), `06_resolve/Cargo.toml`, `06_resolve/src/lib.rs` |
