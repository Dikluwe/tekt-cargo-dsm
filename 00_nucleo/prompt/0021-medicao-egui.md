# Prompt: Medição da Lente contra o egui (panorama)

**Tipo**: Experimento de Arena (`lab/`) — primeira medição do sistema
completo contra um projeto externo.
**Camada**: bancada (sem linhagem obrigatória).
**Criado em**: 2026-06-01
**Decisões de origem**: laudo 0020 (sistema composto, lente roda do
terminal); decisão do autor (medir contra o egui inteiro, panorama via
Arena que importa `lente_wiring`).
**Pré-requisito**: fork 0.27.0 instalado; workspace egui clonado localmente
(o caminho exato pode ser pedido como argumento do programa).
**Posição**: primeiro teste de usuário do sistema composto, valida
generalização além do typst.

---

## Contexto

A composição inicial da lente está completa (laudo 0020). O sistema rodou
do terminal pela primeira vez contra o `lente_core`. Esta medição é o
primeiro teste contra um projeto que **não é o próprio projeto-lente**, e
contra um projeto qualitativamente diferente do typst (que motivou a saga
da resolução de colisões):

- **Diferenças do egui versus typst**: maior reexport (predição: pode
  exercitar `MesmoItem`, que deu zero no typst); muito trait genérico (`Into`,
  `From`, operadores em tipos de coordenadas — predição: o `trait_ref` com
  args resolve a maioria); menos macros geradoras de nomes colidentes
  (predição: cobertura E1 ainda alta).
- **Diferença qualitativa do experimento**: as três medições anteriores
  contavam vereditos (estatística). Esta é **panorama** — observação do que
  a lente diz sobre o egui inteiro, com leitura humana.

A CLI atual só tem modo focado (alvo único). Para panorama, escrever
**programa de Arena que importa `lente_wiring`** e roda em loop sobre todos
os itens. Não usar o binário `lente` repetidamente (overhead de processo).

---

## Restrições

- **Arena (`lab/`)**: regime relaxado. Sem linhagem obrigatória, sem
  cabeçalho `@prompt` no programa.
- **Não modificar nenhum crate do projeto-lente.** Esta medição usa o
  sistema como está. Se algum bug aparecer, registrar — corrigir é outro
  prompt.
- **Importar `lente_wiring` como biblioteca**, não invocar o binário `lente`
  em loop.
- **Não tocar o egui.** Só leitura (via fork).

---

## Programa: estrutura sugerida

`lab/medicao-egui/Cargo.toml`: bin, deps por path para `lente_wiring` e
`lente_core` (para tipos como `Raio`, `Classificacao`); `serde_json` para
output do panorama.

`lab/medicao-egui/src/main.rs`:

1. **Argumentos**: caminho para o workspace do egui (ou usar fixo se o
   gerador preferir; registrar no laudo).
2. **Descobrir crates do workspace egui**: ler o `Cargo.toml` do egui ou
   listar diretórios. Lista esperada (verificar o que existe de fato):
   `egui`, `eframe`, `ecolor`, `emath`, `epaint`, `egui_demo_lib`,
   `egui_extras`, `egui-winit`, `egui_glow`, `egui_wgpu`. Pular crates que
   não compilam ou são `dev-dependencies` only.
3. **Para cada crate**:
   a. Invocar o fork via `lente_wiring::FonteGrafo::Pacote(nome)` para obter
      o JSON e desserializar. Capturar tempo de invocação e tamanho do JSON.
   b. Após a desserialização (interna ao L4) e a resolução de colisões
      (interna ao L4), inspecionar o **grafo final** para coletar
      estatísticas. **Pergunta de design (decisão do gerador)**: como
      acessar o grafo após resolução? O `calcular_raio_de_alvo` atual
      recebe alvo e devolve raio — não expõe o grafo intermediário. Opções:
        - Replicar parte da composição no programa de Arena (chamar
          `lente_infra::fork::invocar_fork` + `desserializar_grafo` + iterar
          colisões manualmente). Mais flexível, mas duplica lógica.
        - Adicionar uma função pública no `lente_wiring` que devolve o
          grafo resolvido (sem calcular raio). Mais limpo, mas mexe no L4.
        - Iterar com `calcular_raio_de_alvo` para cada nó do crate. Mais
          lento mas usa o L4 como está.
      A escolha afeta o que mais é registrado no laudo.
   c. **Coletar por crate**:
      - Tempo de fork (segundos).
      - Tamanho do JSON (bytes/KB).
      - Total de nós no grafo (antes/depois da resolução).
      - Total de arestas.
      - Colisões detectadas (paths com 2+ nós no grafo desserializado).
      - Vereditos da investigação: `Distintos`, `MesmoItem`, `NaoDeterminado`.
      - Padrões dos `Distintos`: quantos `VizinhancaDisjunta`, quantos
        `ImplDeTraitsDiferentes`. (Predição: o último cai a zero, igual ao
        typst pós-trait-por-nó.)
      - **Aparece `MesmoItem`?** Quantos? (Predição: aparece, diferente do
        typst.)
      - **Padrões do `NaoDeterminado`**: quantos e que padrão (macros?
        novo padrão não visto no typst?).
      - **Perfil de raios**: para cada nó, calcular o raio. Histograma:
        quantos `Isolados`, `Folhas`, `Bases`, `Intermediarios`. Mediana
        do impacto direto, mediana do transitivo. Top-10 nós por impacto
        transitivo (os "se mexer aqui, quebra muita coisa").
4. **Agregar**: tabela por crate + agregados do workspace inteiro.
5. **Output**: relatório `lab/medicao-egui/relatorio.md` em prosa com as
   estatísticas, e `lab/medicao-egui/dados.json` com os números brutos.

---

## As perguntas que a medição deve responder

No relatório, responder explicitamente:

### Bloco A — Sistema funciona em projeto não-trivial?

- O fork extrai todos os crates do egui sem erro? Quais (se houver) falham e
  por quê?
- O pipeline (desserialização + resolução + raio) roda sem panic, sem erro
  ao longo do egui inteiro?
- Tempo total da medição. Tempo médio por crate. Crate mais demorado.
- Tamanhos: o maior JSON, o crate com mais nós, com mais colisões.

### Bloco B — As predições da resolução de colisões se confirmam?

- **Cobertura E1**: percentual no egui. Hipótese: ≥ 95%. Resultado?
- **`MesmoItem`**: quantos casos? Predição: > 0 (diferente do typst). Padrão
  desses casos (reexport? alias? algo novo)?
- **`trait_ref` com args**: quantos `Distintos/VizinhancaDisjunta` envolvem
  nós com `trait_ref` diferente entre as cópias? (Indica que o `trait_ref`
  está fazendo trabalho na nomeação.) Quantos com `trait_ref` igual?
- **`NaoDeterminado`**: quantos? Padrões? Aparece padrão diferente dos
  macros do typst (que foi o Limite 6)?
- **Variantes de erro no pipeline**: alguma colisão produziu `NaoDeterminado`
  que a 3ª medição do typst não tinha? Registrar.

### Bloco C — O que a lente diz é útil?

Esta é a pergunta qualitativa. Para responder:

- **Top-10 do egui inteiro por raio transitivo**: ler os nomes. Eles
  intuitivamente parecem itens centrais (que um desenvolvedor do egui
  reconheceria como "se mexer aqui, quebra muito")? Ou são itens que
  parecem irrelevantes (sinal de que a métrica privilegia algo errado)?
- **Distribuição de classificações**: o egui é quase todo Folha, ou tem
  Bases e Intermediários reconhecíveis?
- **A observação Folha/comportamental do laudo 0020**: aparece com volume?
  Ex.: quantos `impl Display::fmt`, `impl Debug::fmt`, `impl From::from`,
  `impl Default::default` aparecem como Folha com zero impacto? Esses
  itens **são** folhas estruturais, mas conceitualmente um humano poderia
  perguntar "se mexer aqui, quebra coisa?" e a resposta humana seria
  diferente da resposta da lente. Quantificar o fenômeno.

### Bloco D — Descobertas inesperadas

Qualquer coisa que apareceu durante a medição que não estava previsto:
crate que não roda, padrão novo de colisão, raio de magnitude inesperada,
tempo muito fora do esperado, etc. Registro honesto do que o experimento
revelou.

---

## Estrutura sugerida do relatório

```
# Medição: Lente contra workspace egui

Data, fork, lente, versão do egui medida.

## Bloco A — Sistema funciona?
(estatísticas operacionais)

## Bloco B — Predições da resolução de colisões
(números vs hipóteses)

## Bloco C — A lente é útil?
(observação qualitativa, top-10, Folha/comportamental)

## Bloco D — Descobertas
(o que apareceu sem ser previsto)

## Limites declarados
(escopo, o que não foi medido, vieses)

## Tabela bruta por crate
(estatísticas detalhadas)
```

---

## Restrição operacional importante

A medição vai demorar (estimativa: 1-2h de execução total). Estruturar o
programa para que **resultados parciais sejam persistidos**:

- Após cada crate processado, escrever um arquivo de checkpoint em
  `lab/medicao-egui/checkpoints/<crate>.json` com as estatísticas dele.
- Se o programa for interrompido, ele pode ser religado e pular crates já
  processados.
- O agregado final é construído lendo os checkpoints + dado novo.

Isso evita perder uma hora de execução por erro num crate específico, e
permite revisar parciais enquanto roda.

---

## O que NÃO entra

- **Mudanças no projeto-lente.** Esta é medição de leitura, não de
  construção. Bugs descobertos viram prompts próprios depois.
- **Recomendações de novos modos da CLI** (ranking, agregado). Embora a
  medição efetivamente construa um "ranking improvisado", a decisão de
  promover isso a feature da CLI é separada, depois da medição.
- **Mudanças no fork.** Mesmo que apareça padrão novo, registrar — não
  agir.
- **Otimização de performance.** Se 2h é o tempo, é o tempo. Otimizar a
  lente para ser mais rápida é trabalho futuro com motivação.

---

## Observação metodológica

Esta medição tem maturidade diferente das três anteriores (todas focadas em
resolução de colisões). Aqui o foco se divide entre **confirmar predições**
(estatística) e **avaliar utilidade** (qualitativo). O bloco C é o mais
delicado — exige leitura humana da saída e julgamento. Não tem como o
Claude Code "responder objetivamente se a lente é útil"; ele apenas
**organiza os dados** para o autor avaliar.

Conclusões qualitativas do bloco C devem vir do autor após ler o relatório,
não do Claude Code. O relatório pode **apresentar** ("os 10 itens com maior
raio transitivo são: ..., ..., ...") sem **concluir** ("a lente é útil para
o egui"). A conclusão é decisão do autor.

Esta postura preserva o método do projeto: dados primeiro, conclusão por
quem decide.

---

## Resultado esperado

- Programa em `lab/medicao-egui/` que rodou contra os crates do workspace
  egui.
- Checkpoints persistidos por crate.
- Relatório `lab/medicao-egui/relatorio.md` respondendo aos quatro blocos
  com dados, sem conclusão qualitativa sobre utilidade (que vem depois,
  do autor).
- Dados brutos em `lab/medicao-egui/dados.json` para análises posteriores.
- Registro de tempo total, tempo por crate, e qualquer crate que não
  rodou (com razão).
