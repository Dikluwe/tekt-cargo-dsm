# Prompt: Verificação — o raio pode ver grafo não-resolvido?

**Tipo**: verificação de leitura (não modifica código)
**Camada**: investigação (não é Arena nem L0/L1 — é só inspeção)
**Criado em**: 2026-05-28
**Decisões de origem**: laudo 0015 (cascata do descritor fechada); dívida
latente do `raio.rs` operar por path; decisão do autor de verificar antes de
mudar.

---

## Propósito

A dívida do raio-por-id tem duas naturezas possíveis:

- **Bug latente**: se há um caminho onde o `raio.rs` é chamado sobre um grafo
  **não-resolvido** (com paths colididos), o BFS pode confundir cópias e
  contar/perder impacto. Mudar para id é correção urgente.
- **Robustez/correção conceitual**: se a arquitetura garante que o raio só
  vê grafo resolvido (paths únicos), operar por path funciona, e mudar para
  id é melhoria de robustez, não correção de bug.

Esta verificação responde qual das duas é a real, lendo o código atual.

---

## O que verificar (sem modificar nada)

1. **`01_core/src/algorithms/raio.rs`** (ou onde quer que o cálculo do raio
   esteja): como a função pública é assinada? Recebe `&Grafo`? Tem alguma
   pré-condição declarada (doc, contrato) sobre o grafo estar resolvido?
   Internamente, o que ela usa para identificar nós ao percorrer arestas —
   path ou id?

2. **Wiring / composição** (se já existe — `04_wiring/`): há ponto de entrada
   que orquestre extração → resolução → raio? Se sim, ele sempre passa o grafo
   pelo `lente_resolve` antes de calcular o raio? Ou há caminho que pule a
   resolução?

3. **Testes do raio** (`raio.rs::tests` ou similar): os testes constroem
   grafos com paths únicos garantidos, ou alguns teriam paths que colidiriam
   se o `lente_resolve` não tivesse rodado? Isso revela se o raio foi testado
   sob a suposição de paths únicos.

4. **Quem mais chama o raio**: `grep` ou inspeção por chamadas à função
   pública do raio no resto do workspace. Quantos chamadores há? Cada um
   chama com grafo resolvido?

---

## O que reportar

Um relatório curto (não precisa ser laudo formal — pode ser uma resposta
direta) respondendo:

1. **Como o raio percorre hoje** (por path ou por id, internamente)?
2. **Há pré-condição declarada** de "grafo resolvido"?
3. **Há caminho** (no wiring atual, em testes, em outros chamadores) **que
   chame o raio sobre grafo não-resolvido**? Lista os caminhos encontrados,
   se houver.
4. **Conclusão**: a arquitetura atual garante que o raio só vê grafo
   resolvido? Sim, não, ou parcialmente (ex.: garante no fluxo principal mas
   não em testes)?

Sem recomendação de mudança — a recomendação vem depois, com a resposta.

---

## Restrições

- **Não modificar nada.** Esta é leitura pura. Sem alterar `raio.rs`, sem
  alterar wiring, sem alterar testes.
- **Sem laudo formal em `00_nucleo/lessons/`.** Isso é verificação rápida; o
  resultado serve para decidir o tom do próximo prompt (mudança raio-por-id),
  não é um componente.
- **Sem instalação de fork, sem rodar medição.** Tudo é leitura de código.

---

## Resultado esperado

Resposta curta às quatro perguntas acima, com trechos de código ou caminhos
de arquivo quando relevante (para o autor poder ler o original se quiser
confirmar).

Depois desta verificação:

- Se **a arquitetura garante** que o raio só vê grafo resolvido → o próximo
  prompt (mudança para id) é "melhoria de robustez", e pode até ser adiado
  ou descartado se você decidir que a garantia atual é suficiente.
- Se **não garante** (há caminho que pula resolve, ou nada compõe ainda) →
  o próximo prompt é "correção de bug latente", e tem prioridade.
- Se **parcialmente** → o próximo prompt define o que muda (talvez só o
  ponto onde a garantia falha, talvez tudo).
