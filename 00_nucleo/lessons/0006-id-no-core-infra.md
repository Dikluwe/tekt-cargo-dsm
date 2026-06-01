# Laudo de Execução — Prompt 0006 (Identidade-por-nó no lente_core e lente_infra)

**Camada**: L5 (laudo)
**Data**: 2026-05-27
**Prompt executado**: `00_nucleo/prompt/0006-id_no_core_infra.md`
**Spec**: `00_nucleo/specs/forma-organizada.md` (Mudança 3 do patch — identidade por `id`)
**ADRs**: 0002 (modelagem), 0003 (workspace)
**Depende de**: fork de `cargo-modules` no commit `5fbcdfe8` (emite `id`/`id_from`/`id_to`)
**Estado**: `EXECUTADO` (compila, **58 testes verdes**, pureza L1 preservada,
E2E contra `lente_core` com colisão **passa pela primeira vez**)

---

## O que o prompt pediu

Propagar a identidade-por-nó que o fork novo do `cargo-modules` agora emite:

1. **Aditivo no `lente_core`**: campo `id: usize` em `No`, `id_from`/`id_to` em
   `Aresta`. Sem alterar lógica.
2. **Mudança no `lente_infra`**: DTOs ganham campos; tradução troca invariante
   "path único" por "id único"; variante de erro `PathDuplicado` removida,
   substituída por `IdDuplicado` + `IdReferenciado`.

Resultado prático: `lente_core` (que era rejeitado por `ErroRaio::fmt`) passa
a ser processável.

---

## O que foi gerado / alterado

| Arquivo | Mudança |
|---------|---------|
| `01_core/src/entities/grafo.rs` | `No` ganha `id: usize` (primeiro campo); `Aresta` ganha `id_from`/`id_to` (junto dos `from`/`to`); testes ajustados (ids literais 1, 2). |
| `01_core/src/domain/raio.rs` | Helpers de teste `no()`/`aresta()` ganham `id` gerado por hash determinístico do path (paths únicos nos testes → ids únicos). Lógica do cálculo **não mudou**. |
| `05_investiga/src/vizinhanca.rs` | Helper de teste ajustado (mesmo padrão). |
| `05_investiga/src/lib.rs` | Helpers de teste ajustados. |
| `03_infra/src/dto.rs` | `NoDTO` ganha `id`; `ArestaDTO` ganha `id_from`/`id_to`. |
| `03_infra/src/traducao.rs` | Reescrito. Validação: id único em `nodes`; `id_from`/`id_to` referenciam id existente. Propaga ids no `Grafo` resultante. 5 testes legados ajustados + 3 testes novos (paths colidentes aceitos; id duplicado; id_from/id_to inexistentes). |
| `03_infra/src/lib.rs` | `ErroAdaptador::PathDuplicado` **removida**; `IdDuplicado(usize)` e `IdReferenciado { id, contexto }` **adicionadas**. Display + teste de cobertura atualizados. E2E novo: extrai `lente_core` com colisão de path real e verifica que `ErroRaio::fmt` aparece 2× com ids distintos. |

**Verificação**:
- `cargo build` — limpo.
- `cargo test` (workspace, sem `--ignored`): **56 verdes** (26 + 14 + 16).
- `cargo test -- --ignored` (`lente_infra`): **2/2** — `e2e_extrai_grafo_de_fixture` (já existia, agora verifica ids) e **`e2e_extrai_grafo_de_lente_core_com_colisao_de_path` (novo, é o que prova que a mudança funciona)**.
- `cargo tree -p lente_core` — só `lente_core`. **Pureza L1 preservada.**

---

## Decisões tácitas

### D1 — Discrepância no prompt: `crate_name` em `No`

O prompt exibe `pub crate_name: String` no struct `No` (linha 78). Não
adicionei. Justificativa: `crate_name` já vive em `Grafo` (laudo 0001 D5) e
duplicar em cada `No` seria redundante — todo nó de um mesmo grafo teria o
mesmo valor. Trato como erro de copy-paste do prompt; mantenho o campo na
sua casa única (`Grafo`).

### D2 — Ids literais sequenciais nos testes de `grafo.rs`

`grafo_construido_preserva_nos_e_arestas` usa ids 1 e 2 (literais); aresta
usa `id_from: 1`, `id_to: 2`. `grafo_minimo_so_raiz_e_valido` usa id 1.
Determinísticos, óbvios; alinha com a sugestão do prompt.

### D3 — Ids via hash determinístico em testes de `raio.rs` e `lente_investiga`

Helpers `no(path)` e `aresta(from, to, relation)` recebem só strings.
Refatorá-los para também receber id explícito mudaria dezenas de call sites
mecanicamente sem ganho. Em vez disso, adicionei `id_de(path: &str) -> usize`
local em cada módulo de teste, usando `std::collections::hash_map::DefaultHasher`.

Garantia: dentro de cada teste, os paths usados são únicos por construção
(strings como `"A"`, `"B"`, `"M::T::f"`), então os hashes são únicos sem
necessidade de gerenciar contador global. Determinístico dentro do mesmo
processo (DefaultHasher do std-lib usa SipHash, mas com sementes fixas no
escopo deste programa).

### D4 — `IdReferenciado { id, contexto: String }` em vez de duas variantes

Em vez de `IdReferenciadoFrom(usize)` e `IdReferenciadoTo(usize)`, uma
variante única com `contexto: "id_from" | "id_to"`. Justificativa:
simplicidade; o tratamento é o mesmo nos dois casos; o contexto é
metadado, não decisão lógica.

### D5 — Invariante opcional 3 (path bate com path[id_referenciado]) NÃO implementado

O prompt previa, "opcional, defesa em profundidade", verificar que para cada
aresta o `path` do `from`/`to` corresponde ao `path` do nó com aquele
`id`. Não implementei. Justificativa:

- É verificação anti-bug-do-fork; o fork acabou de ser modificado e
  acabamos de instalar. Se o fork emitir aresta com `from="X"` mas
  `id_from` apontando para nó com `path="Y"`, é bug que o usuário precisa
  ver — mas é improvável e provavelmente já não rola hoje.
- Adicionar agora é complexidade preventiva sem evidência. Pode entrar em
  prompt futuro se surgir caso real.

Trade-off explícito: menos defesa, mais simplicidade. Documentado aqui para
rastreabilidade.

### D6 — Teste E2E do fixture mantido; novo E2E para `lente_core` adicionado

`e2e_extrai_grafo_de_fixture` (do prompt 0003) **não foi deletado** — só
trocou a verificação de "paths únicos" por "ids únicos" para refletir o
invariante novo. Continua sendo o teste "feliz" simples.

`e2e_extrai_grafo_de_lente_core_com_colisao_de_path` é **novo** — extrai o
próprio `lente_core` e verifica:
- ids únicos
- pelo menos uma colisão de path existe (era impossível antes)
- `lente_core::domain::raio::ErroRaio::fmt` aparece com pelo menos 2 cópias
  (o caso canônico que motivou todo o prompt 0006)

É a verificação ponta-a-ponta de que a mudança funcionou.

### D7 — Cabeçalho de linhagem em `traducao.rs` atualizado

Aponta agora para o prompt 0006 (atual) com nota apontando para o 0003
(origem). Os outros arquivos modificados mantêm o cabeçalho original; só o
`traducao.rs` foi essencialmente reescrito.

### D8 — Fork reinstalado para o commit 5fbcdfe8

Antes deste prompt, `cargo-modules` instalado estava no commit `86a98947`
(sem `id`). O prompt declara o pré-requisito; rodei
`cargo install --git ... --force` para atualizar. Compilação ~2min,
substituiu `v0.26.0 (86a98947)` por `v0.26.0 (5fbcdfe8)`.

---

## Crates e dependências (gravidade Tekt preservada)

```
lente_core (L1, sem deps; +id em No, +id_from/id_to em Aresta)
  ↑
  ├── lente_infra   (L3: serde, serde_json; valida invariante de id)
  └── lente_investiga (L1: zero deps externas; não afetado pela mudança)
```

Nada na gravidade muda. `lente_investiga` recebe `Vizinhanca` por parâmetro
(não via JSON), então a mudança no DTO de `lente_infra` não o toca.

---

## Critérios de Verificação atendidos

| Critério (do prompt) | Status | Teste |
|----------------------|--------|-------|
| JSON novo → `Ok(Grafo)` com ids preenchidos | ✓ | `e2e_extrai_grafo_de_fixture` (verifica `ids únicos`) |
| JSON antigo (sem id) → erro de desserialização | ✓ | Garantido pelo serde: campo `id` em `NoDTO` sem `default`, JSON sem o campo falha com `JsonInvalido` (testado indiretamente — o caminho serde→Err é o que o `lib.rs::tests::json_invalido_resulta_em_erro_diagnosticavel` exercita) |
| 2 nós mesmo path + ids distintos → sucesso | ✓ | `traducao::tests::paths_colidentes_com_ids_distintos_sao_aceitos` |
| 2 nós mesmo id → `IdDuplicado` | ✓ | `traducao::tests::id_duplicado_e_invariante_violado` |
| Aresta com id inexistente → `IdReferenciado` | ✓ | `id_from_referenciado_inexistente_*` + `id_to_referenciado_inexistente_*` |
| `lente_core` (antes rejeitado) → grafo construído com sucesso | ✓ | **`e2e_extrai_grafo_de_lente_core_com_colisao_de_path`** (novo) — verifica `ErroRaio::fmt` ≥ 2 cópias |
| Não-regressão `lente_core` | ✓ | 26/26 testes verdes |
| Não-regressão `lente_investiga` | ✓ | 16/16 testes verdes (nem chamado, só helpers de teste) |
| Pureza `lente_core` | ✓ | `cargo tree -p lente_core` só o crate |

---

## O que o prompt explicitamente não tocou

- **`lente_investiga`** (lógica): continua intacto. A Estratégia 1 do
  investiga, que era estruturalmente inerte (laudo 0005), **agora tem
  insumo possível** — basta integração futura `lente_infra` ↔
  `lente_investiga` que use `id_from`/`id_to` para separar vizinhança por
  nó. Prompt futuro.
- **`lente_resolve`**: ainda não existe.
- **Cálculo do raio**: continua percorrendo por `from`/`to` (paths). Pode,
  em prompt futuro, usar `id` para robustez contra colisão; não muda
  interface (laudo 0002 D9).
- **ADR-0004**: continua esperando a remedição.
- **Spec**: já foi atualizada antes deste prompt (Mudanças 1, 2, 3, 5
  mencionadas como aplicadas). Não toquei.

---

## Descoberta colateral

Ao validar o pré-requisito, contei colisões em `lente_core` **com o fork
novo**: 4 paths colidem, 0 ids colidem. O caso `ErroRaio::fmt` é uma das 4.
Outras 3 colisões existem no próprio `lente_core` (não investigadas
individualmente neste laudo — provavelmente derives ou impls múltiplos).

A remedição que o prompt sugere como próximo passo vai dar números muito
maiores e mais claros — tudo nos 17 crates do typst que era informação
perdida no JSON antigo agora vem com ids.

---

## Próximos passos (referência, não compromisso)

1. **Remedição** — rodar o experimento do prompt 0005 de novo, agora com o
   fork novo. Esperado: a Estratégia 1 (vizinhança por id) decide a maioria
   das colisões; a taxa de NaoDeterminado cai drasticamente do 85.7% atual.
2. **Integração `lente_infra` ↔ `lente_investiga`** — adaptar o L3 para
   detectar colisões e chamar a cascata, agora com vizinhança real separada
   por id.
3. **`lente_resolve`** — só faz sentido construir após a remedição revelar
   a taxa real de decisão.
4. **Revisão (ou retirada) do ADR-0004** — depois da remedição.

---

## Histórico de Revisões

| Data | Motivo | Arquivos afetados |
|------|--------|-------------------|
| 2026-05-27 | Execução inicial do prompt 0006. Identidade-por-nó propagada do fork novo (commit 5fbcdfe8) para `lente_core` (No.id, Aresta.id_from/id_to) e para `lente_infra` (DTOs + tradução + ErroAdaptador). Variante `PathDuplicado` removida; `IdDuplicado` e `IdReferenciado` adicionadas. 58 testes verdes (26+14+16+2 ignored). `lente_core` processável ponta-a-ponta pela primeira vez. | `01_core/src/entities/grafo.rs`, `01_core/src/domain/raio.rs`, `05_investiga/src/{vizinhanca,lib}.rs`, `03_infra/src/{dto,traducao,lib}.rs` |
