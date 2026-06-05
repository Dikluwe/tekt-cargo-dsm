# Prompt: Protótipo de Arena — tela visual da DSM (matriz N×N)

**Camada**: Arena (`lab/`) — experimento descartável, como `lab/proto-ui`
(laudo 0029).
**Criado em**: 2026-06-04
**Estado**: `PROPOSTO`
**Decisões de origem**: trilha da DSM. O laudo 0035 entregou a **matriz como
dado** (`ordem` topológica + `blocos` = SCCs + `dependencias`) no
`--estrutura --json`. A tela visual é o consumidor **humano** desse dado — o
agente já consome o JSON direto (a "saída para IA" já existe). Método do projeto:
**protótipo de Arena antes de nuclear** (padrão do `proto-ui`).
**Pré-requisito**: o `--estrutura --json` com `ordem`/`blocos`/`dependencias`/
`modo_uses`/`escopo` (laudos 0035/0034); o fork atualizado instalado; o egui.
**Posição**: primeira **vista visual** da DSM. Descartável, para aprender se a
matriz é legível e o que o JSON ainda precisa, antes de nuclear uma tela de
verdade.
**Arquivos afetados**: `lab/proto-dsm/` (web: HTML/JS, **sem Rust**) + dumps +
relatório. **Nenhum crate de produção.**

---

## Contexto

Uma DSM é a matriz N×N — linha e coluna são os mesmos módulos, na **ordem** que o
laudo 0035 calculou; a célula `(i,j)` marca "o módulo da linha `i` depende do
módulo da coluna `j`". Com a ordem topológica, as dependências se concentram num
**triângulo**, e os SCCs (os `blocos`) aparecem como **quadrados densos na
diagonal** — os emaranhados. As células do "lado errado" do triângulo são
justamente as de **dentro** dos blocos (as arestas que fecham o ciclo). É a vista
clássica do Lattix.

O dado para desenhar já existe, do laudo 0035:
- `ordem` → os eixos (linhas e colunas, na mesma ordem);
- `dependencias` (`{de, para}`) → as células marcadas;
- `blocos` → os quadrados de ciclo a destacar na diagonal.

No egui em `--so-referencia`: o bloco de 42 contíguo (ordem[55..97]), 55 módulos
"abaixo" (deps externas/stdlib) e 14 "acima" (derivados). A tela mostra esse
formato como matriz.

**A saída para IA não é esta tela** — o agente consome o `--estrutura --json`
direto. Esta tela é o lado **humano** do mesmo dado.

---

## Restrições estruturais

- **Arena, descartável, em `lab/`.** Sem crate, sem tocar o workspace, sem
  `members`. Web (HTML/JS); sem CDN se der (como o `proto-ui` D1); **sem**
  `localStorage`/`sessionStorage` (mantém estado em memória).
- **Consome o `--estrutura --json` como está.** **Não** muda o JSON; o que faltar
  vira **achado**, não conserto.
- **Renderiza legível na escala do egui** (~111 módulos): células pequenas,
  rótulos por hover ou truncados, blocos emoldurados.
- **Qualidade de protótipo.** O objetivo é aprender, não polir.

---

## Fase 1 — Capturar e confirmar o dado

1. Capturar dumps do `--estrutura --json` em `lab/proto-dsm/dados/`:
   - `estrutura-egui-so-referencia.json` (`lente --pacote egui --estrutura
     --so-referencia --json`, do diretório do egui);
   - `estrutura-egui-todas.json` (idem sem a flag);
   - `estrutura-lente-core.json` (controle pequeno, 7 módulos).
2. Confirmar que cada dump tem `ordem`, `blocos`, `dependencias`, `modo_uses`,
   `escopo`.
3. **Registrar o que falta para uma DSM mais rica**: as `dependencias` são
   **binárias** (`{de, para}`) — não há **força de acoplamento** (quantas
   arestas-de-item estão por trás de cada par módulo→módulo). Ferramentas Lattix
   põem **números** nas células (peso). Esse peso é conhecido na agregação
   (`agregar_por_modulo` colapsa N arestas-de-item numa aresta-de-módulo) e
   **descartado** — emiti-lo é mudança pequena de produto, **decidida depois**
   pelo que o protótipo mostrar. Anotar como o achado-cabeçalho (irmão do Achado
   1 do laudo 0029).

**Reportar nas notas**: os campos confirmados, e o que faltaria para pôr peso nas
células.

---

## Fase 2 — Protótipo

Uma página (`lab/proto-dsm/index.html`) que carrega um dump e desenha a matriz:

- **Grade N×N**: linhas e colunas na ordem de `ordem`.
- **Células**: `(i,j)` marcada se `(ordem[i], ordem[j]) ∈ dependencias`.
- **Blocos de ciclo emoldurados**: para cada bloco em `blocos`, um retângulo em
  volta do quadrado contíguo na diagonal — é o achado-cabeçalho (o emaranhado),
  destacado.
- **Rótulos**: os paths dos módulos nos eixos (por hover ou truncados, já que 111
  é muito para caber inteiro).
- **Seletor de dump**: alternar entre egui `--so-referencia` / egui `todas` /
  `lente_core` — para **ver** a diferença do bloco de 42 vs 85 (o que os laudos
  0034/0035 mediram, agora visual).
- **(Opcional)** hover numa célula → mostra "linha depende de coluna" com os dois
  paths.

---

## Critérios de Verificação

```
Dado o dump egui --so-referencia
Quando a página abre
Então renderiza a grade 111×111, com o bloco de 42 emoldurado na diagonal e as
  células vindas de dependencias

Dado a troca para egui todas
Então o bloco maior (85) aparece como quadrado de diagonal maior

Dado o lente_core (7 módulos)
Então renderiza trivialmente (sem blocos — 0 ciclos)

Dado o protótipo
Então há uma nota (README ou na própria tela) registrando o que o JSON precisaria
  para pôr peso nas células (força de acoplamento por par módulo→módulo)
```

(Não há suíte — é Arena. A verificação é a matriz renderizar sobre o dado real e
a nota de achados existir.)

---

## Resultado esperado

- Uma tela de matriz DSM descartável que torna a estrutura **visível como
  matriz**: o bloco de acoplamento emoldurado, o resto em camadas, a diferença
  `--so-referencia` vs `todas`.
- Aprendizado: a matriz é legível em 111? que interações ajudam? o que o JSON
  ainda precisa (força de acoplamento nas células)?
- Material para decidir a forma da tela DSM **real** e se vale enriquecer o JSON,
  **antes** de nuclear.

---

## O que NÃO entra

- **Nuclear uma tela DSM de produção**: vem **depois** do protótipo ensinar.
- **Mudar o `--estrutura --json`** (peso nas células): só **registrar** a falta;
  conserto é prompt de produto próprio, decidido pelo achado.
- **A trilha local (`position`/diff→nós)**: separada.
- **Empacotar a lente como ferramenta de agente (MCP/tool)**: separado — o JSON
  já é o insumo do agente; embrulhá-lo é outra decisão.
- **Multi-nível (crate-a-crate, item)**: a `ordem` é genérica, mas a tela aqui é
  do nível módulo.

---

## Observação metodológica

Arena antes de nucleação — o padrão do `proto-ui` (laudo 0029). A tela é o
consumidor **humano**; o JSON do laudo 0035 já é o consumidor **de máquina/
agente** — o mesmo dado, duas superfícies. O protótipo é também uma **medição**:
mede se a matriz-como-dado basta para desenhar uma DSM útil, e revela o que falta
(força de acoplamento), do mesmo jeito que o `proto-ui` revelou o Achado 1.
"Calcular primeiro, desenhar depois" — a parte difícil (o ordenamento do laudo
0035) está pronta; a tela é apresentação.

---

## Histórico de Revisões

| Data | Motivo | Arquivos afetados |
|------|--------|-------------------|
| 2026-06-04 | Protótipo de Arena da tela visual da DSM: página web que consome o `--estrutura --json` (laudo 0035) e desenha a matriz N×N — eixos na `ordem`, células de `dependencias`, blocos de ciclo emoldurados na diagonal — contra dumps reais (egui `--so-referencia` com bloco de 42, egui `todas` com 85, `lente_core` controle). Descartável, web, sem Rust. Registra o que o JSON precisaria para peso nas células (força de acoplamento por par módulo→módulo, conhecida na agregação e hoje descartada). A tela é o lado humano; o JSON já é a saída para IA. | `lab/proto-dsm/{index.html,README.md,dados/*.json}`, `00_nucleo/lessons/0036-proto-dsm-tela.md` |
