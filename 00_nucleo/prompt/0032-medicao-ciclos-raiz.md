# Prompt: Medir a contribuição do módulo-raiz no ciclo de 85 módulos do egui

**Camada**: Arena (`lab/`) — medição descartável, como `lab/medicao-egui` e
`lab/proto-ui`. **Não** é componente; **não** é solução.
**Criado em**: 2026-06-03
**Estado**: `PROPOSTO`
**Decisões de origem**: laudo 0031, achado-cabeçalho — o `egui` tem **um SCC de
85 módulos** (≈76% do crate), e o JSON mostra o módulo-raiz `egui` **dentro** do
ciclo. Hipótese da revisão: o SCC é **inflado pelos reexports do módulo-raiz**.
O `lib.rs` do egui é um `pub use` grande; pelo Limite 5 da spec, reexport vira
aresta `Uses`; isso torna a raiz "depende de quase tudo", e ela vira a **ponte**
que une os módulos num único SCC (raiz → `slider` → … → raiz fecha o ciclo para
cada widget reexportado). Decisão do autor: **medir antes de procurar solução.**
**Pré-requisito**: `lente_estrutura` (`detectar_ciclos`); a saída
`lente --pacote egui --estrutura --json`; o `egui` v0.34.3.
**Posição**: medição que esclarece **o que o ciclo de 85 significa**, antes de
decidir entre os caminhos (excluir a raiz como prática; subtipos de `uses` no
fork; DSM sobre `uses` cru). **Não** decide o caminho — dá o número para decidir.
**Arquivos afetados**: `lab/medicao-ciclos-egui/` (programa de medição + dumps +
relatório) e `00_nucleo/lessons/0032-…` (registro). **Nenhum crate de produção,
nenhum flag novo, nenhuma mudança no fork.**

---

## Contexto

A pergunta única: **quanto do SCC de 85 módulos é sustentado pelo módulo-raiz?**

O teste: pegar o grafo de dependências módulo→módulo do egui (da saída
`--estrutura --json`) e computar os SCCs **(a)** como está e **(b)** com o nó do
módulo-raiz removido (o nó e as arestas que o tocam). Comparar o tamanho do
maior SCC nos dois.

Se o maior SCC despencar (de 85 para algo pequeno) ao tirar a raiz, a hipótese
da ponte-raiz está confirmada: o "ciclo de 76% do crate" é, em boa parte,
artefato dos reexports (Limite 5), não acoplamento arquitetural genuíno. Se
encolher pouco, há acoplamento mais fundo, e o caminho caro (fork) se justifica.

**Honestidade sobre o alcance**: tirar a raiz ataca o Limite 5 (reexport como
ponte). **Não** isola o Limite 4 (import no nível do módulo — um módulo que
importa de cinco "depende" dos cinco mesmo que só uma função use um). O Limite 4
só o fork separa (subtipos de `uses`). Então esta medição mede a **contribuição
da raiz**, não a inflação total. Declarar isso no relatório.

---

## Restrições estruturais

- **Arena, descartável, em `lab/`.** Sem crate de produção, sem `members`, **sem
  flag no produto** (medir não é resolver).
- **Reusar o MESMO algoritmo de ciclos.** A medição roda `detectar_ciclos` (do
  `lente_estrutura`) ou um SCC equivalente; de qualquer forma, **o run "como
  está" tem que reproduzir o 85 do laudo 0031** — é o portão de sanidade. Se não
  reproduzir, a reconstrução do grafo está errada; parar e corrigir antes de
  confiar no número com-raiz-removida.
- **Identificar a raiz** = o módulo cujo `path` é o nome do crate, sem `::`
  (ex.: `egui`).
- **Não tocar o fork, os crates de produção, nem a spec.**

---

## Fase 1 — Capturar e validar a sanidade

1. Capturar `lente --pacote egui --estrutura --json` (egui v0.34.3, do diretório
   do crate) em `lab/medicao-ciclos-egui/dados/`.
2. Reconstruir o grafo módulo→módulo a partir de `modulos` (nós) e
   `dependencias` (arestas) e rodar a detecção de ciclos **como está**.
3. **Portão de sanidade**: o resultado tem que bater com o laudo 0031 — 1 SCC de
   85 módulos. Se não bater, parar e corrigir a reconstrução.
4. Identificar o nó do módulo-raiz (`egui`).

---

## Fase 2 — Medir

- Computar os SCCs (≥2) do grafo **como está** → 85 (sanidade).
- Remover o nó do módulo-raiz **e** as arestas que o tocam; recomputar os SCCs.
- **Reportar**:
  - maior SCC e número de SCCs, com-raiz vs sem-raiz;
  - quantos módulos **saem** do SCC grande ao remover a raiz (são os que estavam
    presos nele **só** pela raiz);
  - (se barato) se o resíduo **fragmenta** em vários SCCs menores ou continua um;
  - (se barato) a lista dos módulos que saem — para ver se o resíduo é o núcleo
    plausível (`context`/`ui`/`response`/…).
- **Controle**: rodar o mesmo no `lente_core` (que tem 0 ciclos) — remover a raiz
  mantém 0. Confirma que o método **não inventa** ciclo.
- Escrever o relatório em `lab/medicao-ciclos-egui/relatorio.md` e o registro em
  `00_nucleo/lessons/` (convenção de Arena: bruto em `lab/`, registro em
  `lessons/`).

---

## Critérios de Verificação

```
Dado o grafo de módulos do egui reconstruído da saída --estrutura --json
Quando os ciclos são computados como está
Então reproduz 1 SCC de 85 módulos (portão de sanidade do laudo 0031)

Dado o mesmo grafo com o módulo-raiz (egui) removido
Quando os ciclos são recomputados
Então o relatório reporta o maior SCC e o número de SCCs resultantes

Dado a diferença com-raiz vs sem-raiz
Então o relatório reporta quantos módulos saem do SCC grande

Dado o lente_core (0 ciclos) como controle
Quando a raiz é removida e os ciclos recomputados
Então continua 0 ciclos (o método não inventa ciclo)
```

(Não há suíte de produção — é Arena. A "verificação" é o portão de sanidade
bater 85 e o relatório de achados existir.)

---

## Resultado esperado

- Um número claro: maior SCC **com** a raiz (85, sanidade) vs **sem** a raiz, e
  quantos módulos saem.
- Uma conclusão escrita: **quanto** do ciclo de 85 é a ponte-raiz / reexport
  (Limite 5), com a nota explícita de que o Limite 4 (import no nível do módulo)
  **não** foi medido aqui (precisa do fork).
- Material para decidir o próximo passo **com dado**, não por adivinhação:
  - se encolheu muito → excluir a raiz (prática barata) provavelmente basta para
    a vista de ciclos ser útil;
  - se encolheu pouco → o acoplamento é mais fundo, e os subtipos de `uses` no
    fork passam a se justificar.

---

## O que NÃO entra

- **Flag no produto** (`--sem-raiz`/`--excluir-raiz`): **não** — é solução; isto
  é medição. Se o número justificar, a flag é prompt próprio depois.
- **Subtipos de `uses` no fork**: **não** — caminho caro, separado, **decidido**
  por esta medição.
- **DSM visual**: não.
- **Isolar o Limite 4 (import vs referência)**: impossível sem o fork; fora do
  alcance; declarado como limite desta medição.
- **Tocar crates de produção, spec, ou ADRs.**

---

## Observação metodológica

"Medir antes de procurar solução" — o princípio do projeto à risca (laudos 0012
afirmou sem medir, 0013 refutou; daí em diante mede-se primeiro). A medição usa
o **mesmo** `detectar_ciclos` (reproduz o 85 como sanidade) e muda **só a
entrada** (grafo sem a raiz), reportando o delta. E é honesta sobre o alcance:
mede a contribuição da raiz (Limite 5), não a do import no nível do módulo
(Limite 4), que só o fork separa. O número que sair decide se o caminho barato
basta ou se o caro se justifica — em vez de apostar em qualquer um no escuro.

---

## Histórico de Revisões

| Data | Motivo | Arquivos afetados |
|------|--------|-------------------|
| 2026-06-03 | Medição em Arena: quanto do SCC de 85 módulos do egui é sustentado pelo módulo-raiz (ponte de reexports, Limite 5). Recomputa os ciclos com a raiz removida e reporta o delta no maior SCC; controle no lente_core (0 ciclos). Portão de sanidade: o run como-está reproduz o 85 do laudo 0031. Não toca produto/fork/spec; sem flag novo. | `lab/medicao-ciclos-egui/{*, dados/*.json, relatorio.md}`, `00_nucleo/lessons/0032-medicao-ciclos-raiz.md` |
