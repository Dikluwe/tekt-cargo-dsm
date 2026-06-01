# Prompt: Cálculo do Raio de Impacto

**Camada**: L1 — Núcleo
**Criado em**: 2026-05-27
**Estado**: `PROPOSTO`
**Decisões de origem**: spec `forma-organizada.md` (com 5 limites), ADR-0002
**Depende de**: tipo de dados da forma organizada (`01_core/src/entities/grafo.rs`)
**Arquivos a gerar**: `01_core/src/domain/raio.rs` (com testes inline)

---

## Contexto

Este é o coração da lente. Ele responde, sobre o grafo de dependências, à
pergunta central da proposta (§2): **"o que quebra se eu mexer aqui?"** — em
termos estruturais, o raio de impacto de um nó.

Consome o tipo de dados já existente (`Grafo`, `No`, `Aresta`, `Path`,
`Relation`, etc.). É lógica pura sobre esses tipos. Não lê o JSON (isso é L3),
não desenha nada (isso é L2 futuro), não filtra stdlib (isso é outro componente
L1). Só calcula o raio.

A proposta (§3) define o que mostrar: **hierarquia de risco** (base vs. folha) e
**alcance da propagação** (o que depende do nó e do que ele depende, com
profundidade). Este componente computa essas duas coisas.

---

## Restrições Estruturais

- **Camada L1 — pureza absoluta.** Zero I/O, zero dependências externas, só
  stdlib. Sem `serde`, sem rede, sem disco, sem relógio.
- **Recebe a forma completa.** A interface recebe o `Grafo` inteiro (todos os
  campos dos nós: path, name, kind, visibility; todas as arestas com sua
  relation). Isto é deliberado (decisão do projeto): a interface não se amarra a
  um subconjunto.
- **Primeira versão usa topologia + distinção de aresta.** Esta versão calcula o
  raio a partir da topologia (quem aponta para quem) e da distinção Owns/Uses.
  Os campos `visibility` e `kind` **entram na interface mas NÃO são usados no
  cálculo ainda** — ficam reservados para refinamento interno futuro (ex.:
  visibilidade como teto de alcance). Refinar depois NÃO deve mudar a interface.
- **Indexação interna só com stdlib** (ADR-0002, Decisão 2): para percorrer o
  grafo com eficiência, construir a estrutura indexada (ex.: mapas de path →
  vizinhos) à mão, com `HashMap`/`Vec` da stdlib. NÃO usar `petgraph` nem
  biblioteca de grafos.

---

## Instrução

Criar o cálculo do raio de impacto de um nó no grafo.

### Distinção de arestas (decisão do projeto)

As duas relações significam coisas diferentes para o raio, e o cálculo as trata
diferente:

- **`Uses`** é dependência funcional — é o **raio de consequência**. Se A `uses`
  B e B muda de forma, A sente. Este é o cerne de "o que quebra se eu mexer
  aqui".
- **`Owns`** é contenção hierárquica (módulo contém item) — é **contexto**, não
  consequência. Mexer num item não "quebra" o módulo que o contém no mesmo
  sentido. O cálculo do raio de consequência baseia-se em `Uses`; `Owns` serve
  para localizar/contextualizar (saber a que módulo um nó pertence), não para
  propagar consequência.

O cálculo deve permitir distinguir, no resultado, o que está no raio por `Uses`
(consequência real) do que é contexto hierárquico por `Owns`.

### O que calcular, dado um nó-alvo

1. **Hierarquia de risco (base vs. folha)** — sobre as arestas `Uses`:
   - Quantos dependem do nó (entradas `Uses`: quem usa o nó) vs. de quantos o nó
     depende (saídas `Uses`: o que o nó usa).
   - Um nó base = muitos dependem dele, ele depende de poucos (raio grande:
     mexer nele afeta muito). Um nó folha = depende de muitos, poucos dependem
     dele (raio contido). Expor essa caracterização (ex.: contagens de entrada e
     saída, e/ou uma classificação derivada delas).

2. **Alcance da propagação** — sobre as arestas `Uses`, transitivamente:
   - **Quem sente** (montante): o conjunto de todos os nós que dependem do alvo,
     direta e indiretamente (seguindo arestas `Uses` para trás). É o que pode
     quebrar se o alvo mudar.
   - **Do que depende** (jusante): o conjunto de todos os nós de que o alvo
     depende, direta e indiretamente (seguindo `Uses` para frente). É o que pode
     precisar mudar junto.
   - **Profundidade**: para cada nó alcançado, a quão longe (em saltos) ele está
     do alvo. Ou, no mínimo, a profundidade máxima de propagação em cada direção.

### Honestidade sobre granularidade (Limite 4 da spec)

A spec declara que arestas `Uses` originadas de `import` saem do **módulo**, não
do **item** que de fato usa. Consequência: para essas dependências, o raio
aponta o módulo, não a função exata. O cálculo **não pode fingir** granularidade
de item uniforme. Não precisa corrigir isso (não dá — é limite da fonte), mas o
resultado/documentação deve deixar claro que o raio de `Uses` tem esse piso de
granularidade. Não inventar precisão que o grafo não tem.

### Ciclos

O grafo pode ter ciclos (dependências circulares). A travessia transitiva
**deve terminar** mesmo com ciclos — marcar nós já visitados, não recursão
infinita. Um ciclo não é erro a rejeitar aqui; é topologia a percorrer com
segurança.

### Nó-alvo inexistente

Se o path-alvo não existe no grafo, retornar erro ou resultado vazio explícito
(decisão de design — documentar no laudo), nunca panic.

---

## Critérios de Verificação

```
Dado um grafo onde B usa A (B --Uses--> A), e C usa B
Quando se calcula o raio de A
Então "quem sente" (montante) inclui B (direto) e C (indireto, profundidade 2)

Dado o mesmo grafo
Quando se calcula o raio de C (folha — ninguém usa C)
Então "quem sente" é vazio; C é caracterizado como folha

Dado um nó do qual muitos dependem e que depende de poucos
Quando caracterizado
Então é classificado como base (raio grande)

Dado um grafo com aresta Owns (módulo M owns item I) e nenhuma Uses para I
Quando se calcula o raio de consequência de I
Então o raio de consequência por Uses é vazio (Owns não propaga consequência);
M aparece como contexto hierárquico, não como "quem sente"

Dado um grafo com ciclo (A usa B, B usa A)
Quando se calcula o raio de A
Então a travessia termina (sem loop infinito) e B está no alcance

Dado um path-alvo que não existe no grafo
Quando se calcula o raio
Então retorna erro/vazio explícito, sem panic

Dado um nó alcançado a N saltos do alvo
Quando se calcula a propagação
Então a profundidade reportada para esse nó é N
```

Casos de borda: grafo de um nó só (raio vazio nas duas direções); nó isolado
(sem arestas); ciclo; alvo inexistente; cadeia longa (profundidade > 2).

---

## Resultado Esperado

- `01_core/src/domain/raio.rs`: a função (ou funções) que recebe o `Grafo` e um
  path-alvo e retorna o raio — hierarquia de risco e alcance da propagação
  (montante, jusante, profundidade), distinguindo Uses de Owns. Estrutura(s) de
  resultado que expressem isso de forma clara. Indexação interna em stdlib.
  Cabeçalho de linhagem apontando para este prompt.
- Testes inline (`#[cfg(test)] mod tests`) cobrindo os critérios.
- **Pureza**: `cargo tree` sem dependência externa; sem I/O.
- **Laudo de execução** em `00_nucleo/lessons/`: o que o prompt pediu, o que foi
  gerado, decisões tácitas (forma do resultado, como classifica base/folha,
  tratamento de alvo inexistente, como expõe a distinção Uses/Owns).

---

## Histórico de Revisões

| Data | Motivo | Arquivos afetados |
|------|--------|-------------------|
| 2026-05-27 | Criação inicial. Cálculo do raio: topologia + distinção Uses/Owns; visibilidade/kind reservados para refinamento interno futuro. | raio.rs |
