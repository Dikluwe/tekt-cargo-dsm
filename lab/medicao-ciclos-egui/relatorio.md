# Medição: contribuição do módulo-raiz no SCC de 85 do egui

**Data**: 2026-06-03
**Prompt**: `00_nucleo/prompt/0032-medicao-ciclos-raiz.md`
**Tipo**: Arena — medição descartável, sem produto.

---

## A pergunta

O laudo 0031 reportou que o `egui` tem um único SCC de **85 módulos** (≈76%
dos 111 módulos do crate) no grafo módulo→módulo agregado. A hipótese da
revisão: esse ciclo gigante é, em boa parte, **artefato dos reexports do
módulo-raiz** (`lib.rs` é um `pub use` largo; pelo Limite 5 da spec,
reexport vira aresta `Uses`; logo `egui → slider`, `egui → button`, etc.,
e cada widget reexportado de volta para `egui` fecha o ciclo via a
ponte-raiz).

**Quanto** disso é a raiz, e quanto é acoplamento real?

## Método

1. Capturar `lente --pacote egui --estrutura --json` em **ambos** os
   escopos (`Completo` e `SeuCodigo`) e o `lente_core` (controle).
2. Reconstruir o grafo módulo→módulo (paths como nós, dependencias como
   arestas Uses).
3. Rodar `lente_estrutura::detectar_ciclos` **como está** — portão de
   sanidade: precisa reproduzir os 85 do laudo 0031.
4. Remover o nó da raiz (`egui`) **e** todas as arestas que o tocam.
5. Rodar `detectar_ciclos` de novo. Comparar.

O algoritmo de ciclos é exatamente o mesmo da produção (mesmo Tarjan).
Muda apenas a entrada.

## Sanidade — portão obrigatório

| | Completo | SeuCodigo | lente_core |
|---|---|---|---|
| Módulos | 111 | 109 | 7 |
| Dependências | 864 | 862 | 3 |
| SCCs ≥ 2 (como está) | **1** | **1** | **0** |
| Maior SCC | **85** | **85** | **0** |

Bate com o laudo 0031. Portão OK. Confiamos na reconstrução.

## Resultado

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

Tabela do delta:

| | Com raiz | Sem raiz | Δ |
|---|---|---|---|
| Maior SCC (Completo) | 85 | 84 | **−1** |
| Maior SCC (SeuCodigo) | 85 | 84 | **−1** |
| Nº SCCs ≥ 2 | 1 | 1 | 0 |

**Apenas o módulo `egui` (a própria raiz) sai do SCC quando removido.** Os
84 módulos restantes — incluindo `egui::context`, `egui::ui`,
`egui::response`, todos os widgets, todos os containers, `egui::style`,
`egui::memory`, e tudo mais — **continuam num único componente
fortemente conexo entre si**, sem qualquer participação da raiz.

## Interpretação

A hipótese da **ponte-raiz** está **rejeitada pelo dado**. O ciclo não é
artefato dos reexports do `lib.rs`. A raiz contribui zero para o
acoplamento mútuo entre os módulos internos: ela participa do SCC só por
ser o próprio nó intermediário, mas a "rede" entre os 84 outros é
fechada **sem ela**.

Os 84 módulos formam um SCC **genuíno**: cada um, em alguma cadeia, depende
de outro do mesmo grupo, e há ciclos múltiplos entre eles. É acoplamento
arquitetural real, não cosmético.

### O que isto significa para o egui

Olhando os membros (`context`, `ui`, `response`, `memory`, `style`,
`input_state`, `widgets::*`, `containers::*`, `text_selection`, etc.), o
acoplamento mútuo é característico de bibliotecas de **UI imediata**: o
`Context` empresta `Ui`, que devolve `Response`, que pergunta ao
`Context` de novo, que consulta `Memory` e `Style`, etc. — anéis mútuos
inevitáveis no estilo de API.

Para uma vista Lattix, isso aparece como "76% do crate é um SCC". É
informação verdadeira, e talvez "OK" no contexto (é o que o estilo de
imediata-UI faz), mas não é resolvível tirando a raiz.

### Honestidade sobre o alcance

Esta medição mede **a contribuição da raiz** (Limite 5 da spec — reexports
viram `Uses`). **Não** mede a contribuição do **Limite 4** (imports no
nível do módulo: um `use foo::bar;` no topo de um módulo faz ele
"depender" de `foo` mesmo que só uma função interna use `bar`). Para
separar isso, o fork `cargo-modules` precisaria emitir **subtipos de
`uses`** (`uses/import` vs `uses/reference`) — caminho mais caro,
externo ao projeto.

É possível — não medido aqui — que parte do SCC de 84 também seja inflada
por imports-com-pouco-uso. Mas qualquer redução por essa via passa pelo
fork; não é alcançável só com `lab/`.

## Decisão que o número permite

| Caminho | Justificado pelo dado? |
|---------|------------------------|
| **Excluir a raiz** (flag `--sem-raiz` ou prática manual) | **Não** — encolhe 85 → 84, ganho cosmético. Não vale uma flag de produto. |
| **Subtipos de `uses` no fork** (separar import de reference) | **Justifica investigar** — é o único caminho que pode reduzir o SCC sem renomear módulos. |
| **DSM visual sobre `uses` cru** (deixar o humano ver) | **Justifica** independente — a vista global continua útil mesmo com SCC grande; mostrar o emaranhado **ajuda** a entender, e talvez a refatorar. |

A medição cumpriu o papel: deu o número (−1, não −60) para a decisão ser
feita com dado, não com aposta. A hipótese mais barata (raiz como ponte)
era a mais plausível antes da medição; o dado a rejeitou.

## Arquivos

- `dados/estrutura-egui-completo.json` — `lente --pacote egui --estrutura` (Completo).
- `dados/estrutura-egui-seu-codigo.json` — idem `--filtrar-stdlib`.
- `dados/estrutura-lente-core.json` — controle.
- `src/main.rs` — programa de medição. Lê os JSONs, reconstrói o grafo,
  chama `lente_estrutura::detectar_ciclos`, reporta.

## Rodar de novo

```
cd lab/medicao-ciclos-egui && cargo run --release
```

Os JSONs em `dados/` são versão `egui` v0.34.3. Para uma versão diferente,
re-capturar com `lente --pacote egui --estrutura --json` do diretório do
crate egui.
