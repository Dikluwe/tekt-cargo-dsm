# Laudo de Execução — Prompt 0032 (Medição: contribuição da raiz no SCC do egui)

**Camada**: L5 (laudo)
**Data**: 2026-06-03
**Prompt executado**: `00_nucleo/prompt/0032-medicao-ciclos-raiz.md`
**Tipo**: Arena — medição descartável; **não** entrega solução, entrega
um **número**.
**Estado**: `EXECUTADO` — sanidade OK (reproduz os 85 do laudo 0031);
hipótese da revisão **rejeitada pelo dado**; suíte de produção
intacta (176 verdes + 19 ignored, mesmo número do laudo 0031).

---

## Pergunta única

Quanto do SCC de 85 módulos do `egui` (laudo 0031) é sustentado pelo
módulo-raiz `egui`?

## Achado decisivo

**O maior SCC cai apenas de 85 para 84.** Apenas o módulo `egui` sai do
ciclo quando removido — os outros 84 continuam formando um SCC entre si,
sem qualquer participação da raiz. Mesma resposta nos dois escopos
(`Completo` e `SeuCodigo`).

| | Com a raiz | Sem a raiz | Δ |
|---|---|---|---|
| Maior SCC (egui Completo) | **85** | **84** | **−1** |
| Maior SCC (egui SeuCodigo) | **85** | **84** | **−1** |
| Maior SCC (lente_core — controle) | 0 | 0 | 0 |

**A hipótese da ponte-raiz está rejeitada.** O ciclo de 76% do crate é
**acoplamento mútuo genuíno** entre os módulos internos do `egui`
(`context`, `ui`, `response`, `memory`, `style`, todos os widgets, todos
os containers, etc.), não artefato do `pub use` do `lib.rs`.

## Por que era plausível

O `lib.rs` do egui é um `pub use foo::*; pub use bar::*; …` grande; pelo
**Limite 5 da spec** (reexport vira `Uses` ordinário), cada reexport
torna a raiz `egui` "depende de quase tudo". Era razoável imaginar que a
raiz fosse a **ponte** que unia widgets/containers/etc. num único SCC:
`button → context → egui → button`, fechando ciclos cosmeticamente. A
medição mostrou que **não é o caso**: tirar a raiz não desfaz nada
significativo. O acoplamento mútuo entre os módulos internos é
independente da raiz.

## Método

Programa de medição em `lab/medicao-ciclos-egui/` (Arena, fora do
workspace, isolado por `[workspace]` vazio próprio).

1. Capturados 3 dumps com a CLI da lente (release):
   - `dados/estrutura-egui-completo.json` — `lente --pacote egui --estrutura`.
   - `dados/estrutura-egui-seu-codigo.json` — idem `--filtrar-stdlib`.
   - `dados/estrutura-lente-core.json` — controle.
2. `src/main.rs` lê o JSON, reconstrói um `Grafo` módulo→módulo com ids
   artificiais (índice em `modulos`), e roda
   `lente_estrutura::detectar_ciclos` **como está** — exatamente a mesma
   função que o produto usa.
3. **Portão de sanidade**: o run "como está" tem que reproduzir o 85 do
   laudo 0031. Bateu em ambos os escopos.
4. Remove o nó da raiz (`egui` no caso do egui; `lente_core` no controle)
   e todas as arestas que o tocam; recomputa.
5. Reporta o delta.

## Sanidade (portão obrigatório)

```
egui Completo:    85 → bate com laudo 0031 ✓
egui SeuCodigo:   85 → bate com laudo 0031 ✓
lente_core:        0 → bate com laudo 0031 ✓
```

Sem o portão de sanidade, qualquer número da etapa "sem raiz" seria
suspeito. Com ele, a reconstrução do grafo está validada — confiamos no
delta.

## Resultado em linha

```
=== egui Completo ===
[com a raiz]    nº SCCs ≥2: 1    maior SCC: 85 módulos
[sem a raiz]    nº SCCs ≥2: 1    maior SCC: 84 módulos
Módulos que saíram de SCC ≥2: 1  (apenas `egui`)

=== egui SeuCodigo ===
[com a raiz]    nº SCCs ≥2: 1    maior SCC: 85 módulos
[sem a raiz]    nº SCCs ≥2: 1    maior SCC: 84 módulos
Módulos que saíram de SCC ≥2: 1  (apenas `egui`)

=== Controle — lente_core ===
[com a raiz]    nº SCCs ≥2: 0    maior SCC: 0
[sem a raiz]    nº SCCs ≥2: 0    maior SCC: 0   ← método não inventa ciclo
```

## Honestidade sobre o alcance

A medição testa **o Limite 5** (reexports da raiz como ponte). **Não**
testa o **Limite 4** (imports no nível do módulo: `use foo::bar;` no
topo de um módulo faz ele "depender" de `foo` mesmo que só uma função
interna use). O Limite 4 só o fork separa, via **subtipos de `uses`**
(`uses/import` vs `uses/reference`).

É plausível — e a Arena **não pode** confirmar — que parte do SCC de 84
também seja inflada por imports-com-pouco-uso. Qualquer redução por
essa via passa pelo fork; é caminho mais caro, fora do alcance de
`lab/`.

## Decisão que o número permite

| Caminho | Justificado pelo dado? | Por quê |
|---------|------------------------|---------|
| **Excluir a raiz** (flag `--sem-raiz` na CLI) | **Não** | 85 → 84 é ganho cosmético; não vale flag. |
| **Subtipos de `uses` no fork** | **Justifica investigar** | É o único caminho que pode reduzir o SCC sem renomear arquitetura. |
| **DSM visual sobre `uses` cru** | **Justifica** | A vista global continua útil mesmo com SCC grande; mostrar o emaranhado **ajuda** a entender (e talvez a refatorar). Independente do delta da raiz. |
| **Refatorar o egui** | Fora do escopo do projeto | Decisão dos mantenedores do egui, não da lente. |

A medição cumpriu o papel: deu o número (−1, não −60) para a decisão
ser feita com dado, **não** com aposta. A hipótese mais barata (raiz
como ponte) era a mais plausível antes; o dado a rejeitou.

## Verificação

| Item | Resultado |
|------|-----------|
| Portão de sanidade (reproduz 85 do laudo 0031) | **OK** em Completo, SeuCodigo, e controle |
| Maior SCC sem-raiz | **84** (delta −1) |
| Maior SCC sem-raiz no `lente_core` | 0 (método não inventa) |
| Suíte de produção | **176 verdes + 19 ignored** — mesma do laudo 0031 |
| `Cargo.toml` raiz | intocado |
| `members` do workspace | sem `lab/medicao-ciclos-egui` (Arena tem `[workspace]` vazio próprio) |
| Subprocessos do cargo | dois únicos (0023), intocados |

---

## Decisões tácitas

### D1 — Reusar `lente_estrutura::detectar_ciclos`, não reimplementar

Tarjan reimplementado na Arena seria fonte óbvia de erro. A medição usa
**exatamente** a função do produto; muda só a entrada. O portão de
sanidade (85 reproduzido) ancora que o algoritmo e a reconstrução estão
certos antes de confiar no delta.

### D2 — Reconstrução do grafo com ids artificiais

O JSON `--estrutura --json` lista módulos e dependências por path; ids
não são preservados (poderiam ser, mas o `EstruturaModulos` do wiring
expõe paths). Para `detectar_ciclos`, ids são só identificadores
internos — usei o índice em `modulos`. Estável dentro de cada rodada,
suficiente para a medição.

### D3 — Sem flag `--sem-raiz` no produto

Mesmo que o número tivesse confirmado a hipótese, o prompt foi
explícito: **não** virar flag de produto neste prompt. Isto é medição,
não solução. (E o número rejeitou a hipótese de qualquer forma.)

### D4 — Arena segue o padrão `medicao-egui`/`proto-ui`

`[workspace]` vazio no próprio Cargo.toml → isola do pai. Deps por path
para `lente_core` e `lente_estrutura`. `serde`/`serde_json` para parse.
Não entra em `members` nem em `exclude` do workspace pai — é simplesmente
invisível para o `cargo --workspace`. Confirmado por: `cargo test
--workspace` continua exatamente em 176 verdes / 19 ignored.

### D5 — Controle no `lente_core`

Sem o controle, qualquer mudança na contagem após "remover raiz" poderia
ser atribuída ao método. O `lente_core` (0 ciclos) prova que remover a
raiz **não** inventa ciclo — confirma que o delta 85→84 do egui é
significado real, não bug.

---

## Pendências cobertas / abertas

| Pendência | Estado pós-0032 |
|-----------|-----------------|
| Achado do laudo 0031 (SCC de 85 no egui) | **Caracterizado** — não é ponte-raiz; é acoplamento mútuo genuíno. |
| Hipótese da ponte-raiz (reexports do `lib.rs`) | **Rejeitada pelo dado**. |
| Subtipos de `uses` no fork (Limite 4) | **Aberta com justificativa** — é o caminho que pode reduzir, e a medição não o consegue testar. |
| DSM visual | **Aberta com justificativa independente** — útil independente do delta da raiz. |
| Flag `--sem-raiz` no produto | **Não justificada** — −1 não vale uma flag. |

---

## O que NÃO mudou

- **Crates de produção**: zero toques.
- **`Cargo.toml` raiz**: intocado.
- **`lente_estrutura` (L1)**: intocado — a medição **usa**, não modifica.
- **Fork (`cargo-modules`)**: zero toques (medição mediu o estado atual).
- **Spec, ADRs**: zero toques.
- **Suíte de testes**: 176 verdes + 19 ignored, mesma contagem do laudo 0031.

---

## Observação metodológica

"Medir antes de procurar solução" — o princípio do projeto à risca
(laudos 0012 afirmou sem medir, 0013 refutou; daí em diante mede-se
primeiro). Aqui a medição **rejeitou** a hipótese mais barata e plausível,
poupando o projeto de adotar uma solução (flag `--sem-raiz`) que não
resolveria o problema real.

O ganho da Arena não é a UI bonita nem a flag útil; é o **achado** que
move a decisão de "no escuro" para "com dado". Coerente com os laudos
0021 (medição egui), 0027 (Arena promovida a componente), 0029 (achados
do protótipo de UI).

---

## Arquivos

- `lab/medicao-ciclos-egui/Cargo.toml` — Arena isolada.
- `lab/medicao-ciclos-egui/src/main.rs` — programa de medição.
- `lab/medicao-ciclos-egui/dados/{estrutura-egui-completo,estrutura-egui-seu-codigo,estrutura-lente-core}.json`
  — dumps reais.
- `lab/medicao-ciclos-egui/relatorio.md` — relatório bruto da Arena (conteúdo
  denso, padrão laudo 0021 §"experimentos de Arena").

---

## Histórico de Revisões

| Data | Motivo | Arquivos afetados |
|------|--------|-------------------|
| 2026-06-03 | Medição em Arena: hipótese da ponte-raiz (reexports do `lib.rs` do egui inflando o SCC de 85) **rejeitada pelo dado**. Remover o nó da raiz reduz o maior SCC apenas de 85 para 84; só o próprio módulo `egui` sai. Mesma resposta nos dois escopos. Controle no `lente_core` (0 ciclos) confirma método. Portão de sanidade (reproduzir 85) bateu. Decisão prática: o caminho barato (flag `--sem-raiz`) não se justifica; subtipos de `uses` no fork (Limite 4) e DSM visual ficam como caminhos plausíveis para próximas iterações. Sem mudança no produto, no fork ou na spec. | `lab/medicao-ciclos-egui/{Cargo.toml,src/main.rs,dados/*.json,relatorio.md}`, `00_nucleo/lessons/0032-medicao-ciclos-raiz.md` |
