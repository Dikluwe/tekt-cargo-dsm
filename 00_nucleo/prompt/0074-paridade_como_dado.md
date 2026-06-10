# Prompt: paridade como dado — comparar a estrutura de um projeto e sua refatoração

**Camada**: L1 (peça nova: pareamento + deltas entre duas estruturas) + L4
(pipeline `comparar`) + L2 (saída texto + JSON). Os pipelines existentes
**não mudam**; a extração de cada lado é a que já existe (`--estrutura`).
**Criado em**: 2026-06-10
**Estado**: `PROPOSTO`
**Decisões de origem**:
- **Requisito do autor** (2026-06-10): "poder ver um projeto e sua
  refatoração lado a lado para conseguir ver como está a paridade". Os
  lados, conforme o autor: pastas diferentes, branches do mesmo repo, ou
  repositórios distintos — **todos se reduzem a duas raízes no disco**
  (branch → `git worktree add`; a receita vai na doc/laudo, não no código).
- **Método 0035→0036→0071**: calcular primeiro — este prompt entrega a
  comparação **como dado** (texto + JSON). A tela lado a lado é o prompt
  seguinte, desenhado sobre o que este dado mostrar.
- **Laudo 0072** (procedência de escopo): a comparação é superfície de
  consumo nova → default `seu-codigo` (parear sysroot inflaria a paridade
  com acertos triviais), `--completo` disponível, escopo **idêntico nos dois
  lados e declarado**.
**Pré-requisito**: estado pós-0073 (291 + 31 verdes); `--estrutura` com
`ordem`/`blocos`/`dependencias{peso}`.
**Arquivos afetados (a confirmar na Fase 1)**: peça L1 nova (crate
`01_core/comparacao` ou módulo em `lente_estrutura` — decidir e registrar),
`04_wiring` (pipeline), `02_shell/cli` + `catalogo` (modo + saída),
`04_wiring/app` (dispatch), testes.

---

## Contexto

Refatorar (ou reescrever) um projeto cria a pergunta que nenhum modo atual
responde: **a forma nova cobre a antiga?** O `--diff` compara o working tree
contra o HEAD do mesmo repositório — outra pergunta. A paridade compara
**duas estruturas inteiras e independentes**: o que existe dos dois lados,
o que só existe de um, e como as dependências, os pesos e os ciclos mudaram
entre os pares.

### O problema central: o pareamento (e a honestidade dele)

Refatoração renomeia e move. A primeira versão pareia pelo **único critério
que não adivinha**: o path do módulo, **normalizado na raiz do crate** (o
segmento do nome do crate vira neutro, para `velho::nucleo::raio` parear com
`novo::nucleo::raio` mesmo com o crate renomeado). O que não casar é
declarado **sem par**, dos dois lados — um módulo movido (`a::b` → `c::b`)
aparece como sem-par duas vezes, **não** como detectado. Essa limitação vai
escrita na saída (rótulo do catálogo), não escondida: heurística de
similaridade para detectar movidos é trilha futura, decidida pelo uso, e o
dado de hoje não pode fingir que ela existe.

**Sem nota única.** A saída não inventa um "score de paridade" — reporta os
conjuntos e as contagens; o julgamento é do humano (proposta §3, o mesmo
princípio do raio).

---

## Restrições estruturais

- **Pareamento e deltas são cálculo puro** → L1, determinístico, sem deps
  novas, testável com estruturas forjadas. Nada disso no L2/JS futuro.
- **Cada lado é extraído pelo pipeline existente** — mesmo escopo, mesmo
  modo de uses nos dois lados, **forçado** (não é opção comparar universos
  diferentes; o cabeçalho declara o par de parâmetros uma vez).
- **JSON novo, contrato próprio** (`comparacao`): não mexe no JSON do
  `--estrutura`. Texto humano + `--json`, padrão das outras saídas.
- **A receita do worktree é documentação**: o produto recebe **duas
  raízes**; transformar "branch X vs branch Y" em duas raízes é uma linha de
  git registrada no laudo/help — não vira código neste prompt.
- **Convenção Cristalina**: linhagem nos arquivos novos; V1 = 0, V2 = 0
  preservados; V12 = 1 inalterado.

---

## Fase 1 — Leitura e verificação (obrigatória)

1. **Onde a peça mora.** Decidir crate novo (`01_core/comparacao`) vs módulo
   em `lente_estrutura`, pelo critério de dependência: a peça consome **duas**
   `EstruturaModulos` (ou os dois grafos?) — escolher a entrada que evita
   acoplamento desnecessário e registrar.
2. **A normalização do path.** Confirmar a forma real dos paths de módulo
   (`crate::a::b`) e definir a normalização (substituir o 1º segmento por
   marcador) com os casos de borda: o módulo-raiz do crate; paths de
   re-export; colisão pós-normalização (dois módulos do mesmo lado
   normalizando igual — pode? se sim, regra determinística e declarada).
3. **A CLI.** Desenhar o modo: sugestão `lente comparar --antes <raiz>
   --depois <raiz> [--pacote-antes X] [--pacote-depois Y]` (pacotes podem
   ter nomes diferentes pós-refatoração; default = pacote único da raiz, e
   erro claro do catálogo se for ambíguo). Verificar como os modos atuais
   resolvem "pacote da raiz" e reusar.
4. **Worktree na prática.** Validar a receita uma vez de verdade
   (`git worktree add /tmp/lado-b <branch>` → duas raízes) e registrar o
   comando exato no help/laudo.

---

## Fase 2 — Construção

### L1 — o pareamento e os deltas

Entrada: duas estruturas (lado A = antes, lado B = depois), extraídas com
parâmetros idênticos. Saída (`Comparacao`):

- **`pareados`**: lista de pares (path A, path B) casados pela normalização;
- **`sem_par_antes`** / **`sem_par_depois`**: paths que não casaram;
- **`arestas`** (restritas a módulos pareados, nos dois lados):
  `comuns` (com `peso_antes`/`peso_depois` — o delta de acoplamento),
  `so_antes` (dependência que sumiu), `so_depois` (que apareceu);
- **`ciclos`**: contagem e tamanho do maior SCC de cada lado (os números do
  0033/0035, lado a lado — a refatoração desfez o emaranhado?);
- **contagens** de tudo acima (módulos de cada lado, pareados, arestas por
  categoria).

Testes de unidade: estruturas forjadas cobrindo crate renomeado (pareia),
módulo movido (sem-par dos dois lados — o teste-contrato da honestidade do
pareamento), dependência que muda de peso, ciclo desfeito, lado vazio.

### L4 — o pipeline

`comparar(raiz_a, pacote_a, raiz_b, pacote_b, escopo, modo)`: extrai as duas
estruturas com os **mesmos** parâmetros, chama a peça L1, devolve
`Comparacao`. Erros de cada lado identificam **qual lado** falhou (mensagem
do catálogo).

### L2 — a saída

- **Texto**: cabeçalho (os dois lados, pacotes, escopo/modo, a declaração do
  limite do pareamento por nome), resumo de contagens, e as listas (sem-par
  de cada lado; arestas só-antes/só-depois; ciclos lado a lado; maiores
  deltas de peso).
- **`--json`**: a `Comparacao` serializada — **este JSON é o insumo da tela
  lado a lado** (prompt seguinte) e do agente.

### E2E `#[ignore]` (convenção)

- `lente_core` contra **ele mesmo** (duas cópias do mesmo path): paridade
  total — todos pareados, zero sem-par, arestas todas comuns, deltas zero.
- `lente_core` contra uma cópia adulterada em `/tmp` (módulo renomeado +
  dependência removida): os sem-par e o só-antes esperados aparecem.

---

## Fase 3 — Uso real

1. **Evolução real via worktree**: duas tags distantes de um projeto real
   (ex.: egui N vs egui N+k) — a receita do worktree exercitada de verdade,
   e a leitura registrada: o que a comparação mostrou da evolução (módulos
   novos, ciclo cresceu/encolheu)?
2. **Um par real seu de refatoração** (o caso motivador): rodar nos dois
   lados e registrar — a resposta de paridade ajudou a ver o que falta
   migrar? O que a saída de texto não deixou ver (a fila da tela lado a
   lado)?

---

## O que NÃO fazer

- **Não construir a tela lado a lado** — prompt seguinte, sobre este dado.
- **Não inventar heurística de módulo movido/renomeado** — sem-par honesto;
  similaridade é trilha futura decidida pelo uso.
- **Não inventar score único de paridade** — conjuntos e contagens.
- **Não comparar com parâmetros diferentes entre os lados** — forçado igual.
- **Não ensinar git à lente** (checkout/worktree em código) — duas raízes;
  a receita é documentação.
- **Não tocar `--diff`, `--estrutura`, raio, MCP** — aditivo.

---

## Critérios de Verificação

```
Dado lente comparar com duas raízes válidas
Então texto e JSON com pareados, sem-par dos dois lados, arestas
comuns/só-antes/só-depois com pesos, ciclos lado a lado, contagens — e o
cabeçalho declarando lados, parâmetros idênticos e o limite do pareamento

Dado o crate renomeado entre os lados
Então os módulos pareiam pela normalização (teste de unidade)

Dado um módulo movido (a::b → c::b)
Então sem-par dos dois lados — não adivinhado (teste-contrato)

Dado lente_core contra si mesmo (E2E)
Então paridade total, deltas zero

Dado a cópia adulterada (E2E)
Então os sem-par e o só-antes esperados, identificados

Dado falha de extração em um lado
Então o erro identifica o lado, mensagem do catálogo

Dado a suíte e o linter
Então verde, com o número EXATO de ignorados declarado (disciplina 0068);
V1 = 0, V2 = 0 preservados; V12 = 1; sem deps novas no L1
```

---

## Resultado esperado

- `lente comparar`: a paridade entre duas formas como dado — texto para o
  humano, JSON como contrato da tela lado a lado e do agente.
- O pareamento honesto (nome normalizado; sem-par declarado; limite escrito
  na saída) e os deltas que respondem "o que falta migrar, o que mudou de
  acoplamento, o emaranhado encolheu?".
- A receita branch→duas-raízes validada e documentada.
- **Laudo** em `00_nucleo/lessons/0074-…`: as decisões da Fase 1, as
  leituras da Fase 3 (a evolução real e o par real do autor), o número
  exato de ignorados, e a fila da tela lado a lado ditada pelo que o texto
  não deixou ver.

---

## Cuidados

- **O teste-contrato do movido é o coração** — é ele que impede o pareamento
  de virar adivinhação silenciosa no futuro.
- **Os dois lados no mesmo universo** — escopo/modo forçados iguais; o
  cabeçalho declara uma vez.
- **O JSON nasce pensado como insumo da tela** — nomes e formas estáveis;
  a tela do prompt seguinte não deve precisar de campo que este JSON não
  tenha previsto barato (índices/paths completos, contagens prontas).

---

## Histórico de Revisões

| Data | Motivo | Arquivos afetados |
|------|--------|-------------------|
| 2026-06-10 | Paridade como dado (requisito do autor: projeto vs refatoração; lados = pastas, branches ou repos distintos — **tudo reduzido a duas raízes**, branch via receita `git worktree` documentada, não codificada). Método 0035→0036: o dado antes da tela. Peça L1 nova: pareamento por **path normalizado na raiz do crate** (crate renomeado pareia; módulo movido = **sem-par dos dois lados, declarado** — teste-contrato; sem heurística, sem score único) + deltas: arestas comuns (peso antes/depois) / só-antes / só-depois entre pareados, ciclos lado a lado, contagens. L4 `comparar(raiz_a, pacote_a, raiz_b, pacote_b, escopo, modo)` — parâmetros **forçados iguais** nos dois lados, default `seu-codigo` (procedência 0072), erro identifica o lado. L2 texto (cabeçalho com limite do pareamento) + `--json` (contrato da tela lado a lado, prompt seguinte). E2E: lente_core vs si mesmo (paridade total) e vs cópia adulterada. Fase 3: tags reais via worktree + um par real de refatoração do autor. Aditivo; nº exato de ignorados no laudo (disciplina 0068). | peça L1 nova (`comparacao`), `04_wiring` (pipeline), `02_shell/{cli,catalogo}` (modo `comparar` + saída), `04_wiring/app` (dispatch + E2E), `00_nucleo/lessons/0074-paridade_como_dado.md` |
