# Laudo de Execução — Prompt 0004 (lente_investiga)

**Camada**: L5 (laudo)
**Data**: 2026-05-27
**Prompt executado**: `00_nucleo/prompt/0004-lente_investiga.md`
**Spec de origem**: `00_nucleo/specs/forma-organizada.md`
**ADRs aplicáveis**: 0004 (resolução de colisões), 0003 (workspace), 0002 (modelagem)
**Depende de**: laudos 0001 e 0003. Cria adição ao `lente_core` (Veredito) e novo crate L1 puro.
**Estado**: `EXECUTADO` (compila, **55 testes verdes**, pureza dos dois L1 preservada)

---

## O que o prompt pediu

Resolver a Descoberta 2 do laudo 0003 (colisões de path) por duas frentes:

1. **Adição ao `lente_core`**: tipos `Veredito` e `Evidencia` (ADR-0004 §5).
2. **Novo crate `lente_investiga`** (L1 puro): cascata vizinhança → fonte
   que classifica cada colisão sem modificar grafo nem nomear identidades.

---

## O que foi gerado

| Arquivo | Propósito |
|---------|-----------|
| `01_core/src/entities/veredito.rs` | `Veredito` enum (3 variantes), `Evidencia` enum (2 variantes), testes mínimos. |
| `01_core/src/entities/mod.rs` (edit) | `pub mod veredito;` adicionado. |
| `Cargo.toml` (raiz, edit) | `members` recebe `"05_investiga"`. |
| `05_investiga/Cargo.toml` | Crate `lente_investiga`. Dep única: `lente_core` (path). |
| `05_investiga/src/lib.rs` | API pública `investigar(par, vizinhanca, fontes)`, tipos auxiliares (`ParColidente`, `Vizinhanca`, `ArestasNo`, `ArquivoFonte`), orquestração da cascata. |
| `05_investiga/src/vizinhanca.rs` | Estratégia 1 — comparação de conjuntos de arestas. |
| `05_investiga/src/fontes.rs` | Estratégia 2 — parser textual limitado de `impl <Trait> for <Tipo>`. |

**Verificação**:
- `cargo build` — limpo.
- `cargo test` (workspace): `lente_core` **26/26** · `lente_infra` **13/13** (+ 1 `--ignored`) · `lente_investiga` **16/16**. Total: **55 verdes + 1 ignored**.
- `cargo tree -p lente_core` — só `lente_core`.
- `cargo tree -p lente_investiga` — só `lente_core` (workspace), sem deps externas. **Pureza L1 confirmada**.

---

## Decisões tácitas

### D1 — Diretório `05_investiga/`

Sugestão do prompt: `01_core_investiga/`, `05_investiga/`, ou similar.
Adotado `05_investiga/` (próximo número disponível na convenção `NN_nome/`
que o projeto já usa: `00_nucleo`, `01_core`, `03_infra`). Pulei `04_` para
deixar espaço a um futuro `04_*` que faça sentido temático.

### D2 — `Veredito` + `Evidencia` em `lente_core` (decisão do ADR-0004 §5)

`lente_investiga` produz; o (futuro) `lente_resolve` consome. Por estar entre
dois crates novos, o tipo da interface vai ao vocabulário central. Adição
**puramente aditiva** ao `lente_core` (não muda nada existente; os 22 testes
prévios continuam passando).

### D3 — Tipos de entrada da função em `lente_investiga` (não em `lente_core`)

`ParColidente<'a>`, `Vizinhanca`, `ArestasNo`, `ArquivoFonte` são tipos da
assinatura específica de `investigar()`. Mantê-los no crate consumidor evita
poluir o `lente_core` com vocabulário que só vive aqui. Se outro crate
precisar reutilizá-los algum dia, migram para o `lente_core`.

### D4 — `ParColidente<'a>` zero-cópia (referências)

Em vez de clonar os dois `No`, carrega `&'a No`. Investigação é pura leitura;
não há razão para alocar.

### D5 — Critério categórico de vizinhança (sem thresholds mágicos)

Coerente com o princípio fixado no laudo 0002 D1. Três casos puros:

| Caso | Critério | Conclusão |
|------|----------|-----------|
| Disjunta | `compartilhadas == 0` E `exclusivas_a > 0` E `exclusivas_b > 0` | `Distintos { VizinhancaDisjunta }` |
| Idêntica | `exclusivas_a == 0` E `exclusivas_b == 0` E `compartilhadas > 0` | `MesmoItem` |
| Outro | resto (sobreposição parcial, um lado vazio, etc.) | `Inconclusivo` → próxima estratégia |

### D6 — Ambos vazios → Inconclusivo (não decidir nada com zero evidência)

Subcaso explícito antes dos demais checks. Dois nós com zero arestas não
provam coincidência nem distinção; mais honesto passar a bola.

### D7 — Identidade comparável das arestas: `(from, to, relation)`

`HashSet<ChaveAresta>` onde `ChaveAresta { from: String, to: String,
relation: Relation }`. Comparação por igualdade da tripla. Cumulamos
`entrando` e `saindo` num mesmo conjunto por nó — se duas cópias têm a
mesma aresta (mesma origem/destino/tipo), conta como uma "compartilhada"
qualquer que tenha sido a direção.

### D8 — Tipo e método extraídos dos dois últimos segmentos do path

`crate::mod::ErroRaio::fmt` → `("ErroRaio", "fmt")`. Suficiente para o caso
canônico (Rust idiomático). Path com menos de 2 segmentos →
`NaoDeterminado` com diagnóstico.

### D9 — Trait normalizado pelo último segmento

`fmt::Display` → `Display`; `std::fmt::Debug` → `Debug`. Cobre tanto o
caso "trait sem qualificação" quanto "trait qualificado por módulo". Não
distingue `core::fmt::Display` de `std::fmt::Display` (são o mesmo trait
reexportado) — colapso intencional.

### D10 — Parser linha-a-linha; limitações declaradas

O ADR-0004 já antecipa: macros, genéricos com `where` em múltiplas linhas,
`#[cfg]`, strings com `{`/`}` literais não são cobertos. Quando o parser
não encontra dois `impl <Trait> for <tipo>` com o método, o resultado é
`Inconclusivo` (cascata cai em `NaoDeterminado`) — explícito, não silencioso.

### D11 — Métodos contados apenas em `depth == 1`

Dentro do `impl`, o corpo está em `depth >= 1`; o corpo de uma função
aninhada está em `depth >= 2`. Métodos do impl ficam em `depth == 1`.
Coberto por teste (`metodo_dentro_de_funcao_aninhada_nao_conta`).

### D12 — `impl Tipo { ... }` (inerente, sem `for`) é ignorado

Só impls-de-trait geram evidência. Inerentes (sem `for`) não trazem trait
para discriminar. Coberto por teste (`impl_inerente_e_ignorado`).

### D13 — Comentários `// impl ... for X` não geram falso positivo

`trim()` da linha; `starts_with("//")` rejeita. Coberto por teste.

### D14 — Genéricos do impl pulados via contador `<`/`>`

`impl<T: Clone + Send> Trait for Tipo<T>` — o parser pula o `<T: ...>`
inicial contando profundidade de `<` e `>` antes de procurar `for`.
Coberto por teste.

### D15 — Pré-condição lógica verificada (paths iguais)

Se `par.a.path != par.b.path` (chamador errado), `investigar()` devolve
`NaoDeterminado` com diagnóstico explícito em vez de `panic!`. Coerente com
o espírito do ADR-0004 de "honestidade, não silencioso". Diferente de
"corrigir o erro" — só descrever.

### D16 — Diagnóstico é concatenação transparente

Cada estratégia que falha em decidir adiciona sua linha ao `diagnostico`
final. O usuário lê e sabe exatamente o que cada estratégia tentou e por
quê não bastou.

---

## Crates e dependências resultantes (gravidade Tekt → Cargo)

```
lente_core (L1, sem deps)
  ↑
  ├── lente_infra   (L3: serde, serde_json)
  └── lente_investiga (L1: zero deps externas)
```

Gravidade preservada: o crate L1 novo (`lente_investiga`) **não importa**
`lente_infra`. As três flechas vão de cima para baixo no lattice.

---

## Critérios de Verificação atendidos

| Critério (do prompt) | Status | Teste |
|----------------------|--------|-------|
| Vizinhanças disjuntas → `Distintos / VizinhancaDisjunta` | ✓ | `vizinhanca::tests::vizinhancas_disjuntas_decidem_distintos` + integração `vizinhancas_disjuntas_decidem_sem_fontes` |
| Vizinhanças idênticas → `MesmoItem` | ✓ | `vizinhanca::tests::vizinhancas_identicas_decidem_mesmo_item` + integração `vizinhancas_identicas_decidem_mesmo_item` |
| Vizinhança ambígua + sem fontes → `NaoDeterminado` (E1 inconclusiva, E2 não tentada) | ✓ | `vizinhanca_ambigua_sem_fontes_e_nao_determinado` (verifica diagnóstico textual) |
| Caso `ErroRaio` (Display+Debug) com fontes → `Distintos / ImplDeTraitsDiferentes` | ✓ | `caso_canonico_erro_raio_com_fontes_decide_pelos_traits` |
| Fontes não-canônicas → `NaoDeterminado` com diagnóstico | ✓ | `fontes_sem_padrao_canonico_caem_em_nao_determinado` |
| Par inválido (paths diferentes) → `NaoDeterminado` (decisão D15) | ✓ | `par_com_paths_diferentes_nao_e_colisao` |
| Construção dos tipos `Veredito` e `Evidencia` no `lente_core` | ✓ | 4 testes em `veredito::tests` |
| Não-regressão `lente_core` | ✓ | 22 testes originais continuam passando |
| Pureza `lente_core` | ✓ | `cargo tree -p lente_core` mostra só o crate |
| Pureza `lente_investiga` | ✓ | `cargo tree -p lente_investiga` mostra só `lente_core` |

Casos de borda extras cobertos (além dos do prompt):

- `impl<T> Trait for Tipo<T>` — genéricos no impl reconhecidos (`impl_com_genericos_e_reconhecido`).
- `fn` dentro do corpo de outro `fn` — não conta como método do impl (`metodo_dentro_de_funcao_aninhada_nao_conta`).
- Comentário `// impl ... for X` — não casa (`comentario_com_impl_for_nao_e_falso_positivo`).
- Apenas um `impl <Trait>` encontrado — inconclusivo (`so_um_impl_de_trait_e_inconclusivo`).
- Ambos os lados com zero arestas — inconclusivo (`ambos_vazios_e_inconclusivo`).

---

## O que o prompt explicitamente não pediu (não fiz)

- **Não escrevi `lente_resolve`.** Componente irmão, futuro (ADR-0004 §3).
- **Não modifiquei `lente_infra`.** Adaptação do L3 para usar a cascata é
  prompt futuro (ADR-0004 "Prompts Afetados").
- **Não toquei disco.** Leitura dos arquivos `.rs` é responsabilidade do
  `lente_infra` (gravidade Tekt: I/O em L3, lógica em L1).
- **Não nomeei identidades novas.** Só reporto evidência; nomear é
  responsabilidade do `lente_resolve` (ADR-0004 §3).
- **Não tentei resolver Limites 1–5 da spec.** Escopo declarado do ADR-0004
  §7: só colisões de path.

---

## Risco aceito (registrado no próprio ADR-0004)

A arquitetura foi decidida sem medir a prevalência real das colisões em
crates Rust. O ADR-0004 reconhece isso explicitamente. O cenário concreto
medido até agora: **1 caso** (`ErroRaio::fmt` em `lente_core`). Se vier a
medir muitos crates e descobrir que a Estratégia 2 raramente decide, a
arquitetura precisa de revisão (`lente_investiga` simplificado, ou
estratégia diferente).

---

## Próximos componentes (referência, não compromisso)

1. **`lente_resolve`** (L1 novo) — aplica o veredito no grafo. Convenção
   inicial declarada no ADR-0004 §3: `Tipo::<Trait>::metodo`.
2. **Adaptação do `lente_infra`** — detecta colisão, lê arquivos `.rs`,
   invoca `lente_investiga`, invoca `lente_resolve`, materializa o grafo
   resolvido. Adiciona variante de erro `ColisaoNaoResolvida`.
3. **Medição da prevalência** — rodar o pipeline contra ~20 crates reais
   após `lente_resolve` existir, contar quantos colidem, e em quantos a
   cascata resolve. Se a Estratégia 2 raramente decidir, revisar o ADR-0004.

---

## Histórico de Revisões

| Data | Motivo | Arquivos afetados |
|------|--------|-------------------|
| 2026-05-27 | Execução inicial do prompt 0004. Adição de Veredito/Evidencia ao lente_core (4 testes). Novo crate lente_investiga (16 testes), L1 puro, cascata vizinhança→fontes. 55 testes verdes total. Pureza dos dois L1 preservada. | `01_core/src/entities/veredito.rs` (novo), `01_core/src/entities/mod.rs`, `Cargo.toml` (raiz), `05_investiga/Cargo.toml`, `05_investiga/src/{lib,vizinhanca,fontes}.rs` |
