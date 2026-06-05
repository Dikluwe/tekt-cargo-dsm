# Laudo de Execução — Prompt 0029 (Protótipo de Arena para UI)

**Camada**: L5 (laudo)
**Data**: 2026-06-03
**Prompt executado**: `00_nucleo/prompt/0029-proto-ui-arena.md`
**Estado**: `EXECUTADO` — Arena `lab/proto-ui/` criada, dumps reais
capturados, página web consome o JSON. Convenção de Arena (laudos
0021/0027): fora do workspace, descartável, **contém um achado
não-trivial sobre o contrato de JSON** que decide o próximo passo.
Suíte intacta: 143 verdes + 15 ignored (mesma contagem do laudo 0028).

---

## Fase 1 — Dumps capturados

5 arquivos em `lab/proto-ui/dados/`, capturados com `lente` em release:

| Arquivo | Tamanho | Como foi gerado |
|---------|---------|-----------------|
| `ranking-egui.json` | 2,7 KB | `lente --pacote egui --ranking --top 30`, cwd = `<egui>/crates/egui` |
| `ranking-lente-core.json` | 1,5 KB | `lente --pacote lente_core --ranking --top 15`, cwd = workspace |
| `raio-path.json` | 1,8 KB | `lente --pacote lente_core --alvo lente_core::entities::grafo::Path --verbose` |
| `raio-kind.json` | 0,8 KB | idem com `Kind` (#2 do ranking) |
| `raio-classificacao.json` | 0,6 KB | idem com `Classificacao` (#6) |

### Shape confirmado do JSON

**Ranking**: `{"ranking": [{ posicao, impacto, classificacao, path }, …]}`.
Item tem 4 campos; o array é ordenado decrescente por `impacto` com
desempate por `path` ascendente (laudo 0027 D-ordenação).

**Raio (modo per-nó)**:
```
{
  "alvo": "<path resolvido>",
  "classificacao": "Isolado|Folha|Base|Intermediário",
  "diretos":     <int>,   // uses_entrada
  "transitivos": <int>,   // montante.len()
  "impactados":  [path,…] // só quando --verbose
}
```

`--verbose` é necessário para `impactados` aparecer; sem ele, só contagens.

### Achado 1 (decisivo) — `impactados` é lista plana

Coerente com o que o prompt antecipou: **sem profundidade** e **sem
arestas entre impactados**. O `Raio.montante` em memória **tem** a
profundidade (`HashMap<Path, usize>`), mas o `saida::formatar_json`
descarta na hora de emitir e ordena os paths alfabeticamente:

```rust
// 02_shell/cli/src/saida.rs (referência, não tocado neste prompt)
let mut paths: Vec<String> = raio.montante.keys().map(|p| p.as_str().into()).collect();
paths.sort();
```

Para uma vista de **lista** (o que este protótipo entrega) é suficiente.
Para vista de **grafo** ou **camadas por profundidade**, faltariam **dois
campos** no JSON:

| Falta | Custo de adicionar |
|-------|--------------------|
| Profundidade por impactado | **Barato** — já está em `Raio.montante`; é só emitir o `HashMap` em vez de só as chaves. |
| Arestas entre impactados | **Caro** — não está em `Raio`; precisaria expor o subgrafo induzido pelos nós impactados ou um campo `arestas[]` paralelo. |

A decisão fica para o autor: se a próxima iteração da UI for por
camadas, basta o **barato** (mudar `saida.rs` para emitir
`{path: profundidade}` em vez de `[path]`); se for grafo, paga-se o
caro. **Não consertado aqui** (escopo do protótipo).

### Achado 2 (não previsto, importante) — classificação diverge entre os modos

O nó `lente_core::entities::grafo::Path` aparece como:

| Caminho | Classificação |
|---------|---------------|
| ranking (`--ranking`, com `filtrar_stdlib`) | **Base** (39 impacto) |
| raio (`--alvo …Path --verbose`, **sem** filtro) | **Intermediário** (39 transitivos) |

Causa raiz: o pipeline `rankear_pacote` aplica `lente_filtro::filtrar_stdlib`
**antes** do cálculo; o pipeline `calcular_raio_de_alvo` **não** aplica
nada — opera sobre o grafo cru-mas-resolvido. No grafo cru, `Path` tem
`Uses` saindo para nós de sysroot (`String::*`, `&str` traits, etc.) →
`uses_saida > 0` → classificação `Intermediário`. No grafo filtrado,
esses uses somem → `uses_saida = 0` → `Base`.

**Implicação para o autor**:
- Visto **da UI**, é confuso: o usuário clica em "Base #1" do ranking e
  vê "Intermediário" no detalhe. Esperaria mesma classificação.
- Visto **do desenho**, é uma divergência semântica entre dois pipelines
  que prometem ser "o mesmo grafo". Não é bug — é **decisão omitida**.

Opções (não decididas aqui):
- **Aplicar o filtro também no modo per-nó** (`calcular_raio_de_alvo`
  via filtro). Coerência total, mas muda comportamento existente — quebra
  a propriedade "modo per-nó intacto" defendida no laudo 0027 D.
- **Manter modos diferentes** e **anotar na UI** que ranking é
  "sem-sysroot" e raio é "completo". Honesto, mas exige documentar e
  treinar o leitor.
- **Adicionar bandeira CLI** `--filtrar-stdlib` ao modo per-nó. Mais
  flexível, paga complexidade.

Registrado aqui (e na UI do protótipo, via badge na coluna
classificação que muda entre os dois lados) para o próximo prompt
decidir.

### Achado 3 (menor) — ordenação dos `impactados`

`saida::formatar_json` ordena alfabeticamente os paths em `impactados`.
Bom para diff / leitura humana, ruim para qualquer agrupamento por
profundidade (a ordem **alfabética** mistura camadas). Resolvido junto
com o achado 1 quando/se a profundidade for emitida.

---

## Fase 2 — Protótipo

Estrutura final:

```
lab/proto-ui/
├── index.html         # ~11 KB; HTML + CSS + JS inline; sem CDN
├── README.md          # como rodar, achados, dumps
└── dados/
    ├── ranking-egui.json
    ├── ranking-lente-core.json
    ├── raio-path.json
    ├── raio-kind.json
    └── raio-classificacao.json
```

### Como rodar

```
cd lab/proto-ui && python3 -m http.server 8080
# http://localhost:8080/
```

Smoke-testado neste laudo (servidor levantado, `fetch` de
`dados/ranking-lente-core.json` retorna 15 itens com a forma esperada).

### O que faz

- **Vista de ranking** (painel esquerdo): tabela ordenável (clique nas
  colunas `#`/`Impacto`/`Classif.`/`Path`). Selector troca a fonte
  (`lente_core` ↔ `egui`). Linhas com dump de raio disponível são
  clicáveis; demais ficam visualmente desativadas com `title` explicando
  que falta dump.
- **Vista de raio** (painel direito): cabeçalho com o `alvo`, badge da
  classificação, `Diretos` e `Transitivos`. Lista plana de impactados
  com contador. Nota fixa na parte de cima informa sobre o limite do
  contrato JSON (achado 1).
- **Bootstrap**: ao abrir, carrega `ranking-lente-core.json` por
  default; clicar numa linha de `Path`/`Kind`/`Classificacao` abre o
  raio correspondente.

### Decisões de implementação

- **Sem CDN, sem framework**. CSS e JS embarcados no `index.html`.
  Compatível com `python3 -m http.server` puro. Coerente com "qualidade
  de protótipo" do prompt.
- **Mapa explícito** `MAPA_RAIO = { path → arquivo }` no JS. Não
  reinvento URL routing — o protótipo carrega dumps locais, não
  consulta um backend. Quem clica num nó **sem** dump vê a linha
  desativada com explicação no tooltip.
- **Badges coloridos por classificação** (Base/Intermediário/Folha/
  Isolado). Útil para o achado 2 ser visível em ambos os painéis ao
  mesmo tempo: o protótipo **exibe** a divergência, não a esconde.
- **Achado 1 declarado dentro da UI**, em caixa amarela na vista de
  raio. Quem abre o protótipo lê o achado **lá** — não só no README.
  Coerente com "o protótipo é também uma medição" (observação
  metodológica do prompt).

---

## Verificação

| Item | Resultado |
|------|-----------|
| Arena criada em `lab/proto-ui/` (sem Rust, sem build) | sim |
| `Cargo.toml` raiz | **intocado** — Arena fora do workspace |
| 5 dumps reais em `dados/` | sim — egui + lente_core, ranking + raio |
| Página renderiza dado real | sim (smoke-test via `python3 -m http.server`) |
| Vista de ranking ordenável (4 colunas) | sim |
| Vista de raio com classificação + diretos + transitivos + impactados | sim |
| Nota dentro da UI sobre o que falta no JSON | sim (caixa amarela na vista de raio) |
| `cargo test --workspace` | **143 verdes + 15 ignored** (igual ao laudo 0028) |
| Subprocessos do cargo | dois únicos, intocados |
| Pureza do L1 | intacta |

---

## Decisões tácitas

### D1 — Web embarcada, sem CDN

A linguagem de Arena pede mínimo. Adicionar CDN (d3 etc.) traria
ergonomia mas adiciona dependência de rede, vincula a uma lib, e a
vista que sobra (lista de impactados) não pede biblioteca de grafo.
Decisão: **só DOM nativo**. Se a próxima iteração for "vista de grafo",
e se o JSON ganhar as arestas (achado 1), aí entra d3/cytoscape — mas
**aí**, não agora.

### D2 — `MAPA_RAIO` explícito, não auto-fetch

Auto-fetchar `dados/raio-<slug>.json` para cada path do ranking
poluiria o protótipo com 404s (raio só capturado para 3 nós). Mapa
explícito mostra o **escopo do experimento**: três nós-base do
`lente_core` mais o ranking do egui. Quem quiser testar com outro nó
captura o dump e adiciona uma linha no mapa.

### D3 — Achado 2 registrado, **não consertado**

A divergência ranking/raio é **achado de Arena** — exatamente o tipo
de coisa que o protótipo serve para descobrir e o prompt explicitamente
mandou registrar como tal ("medir se o JSON é suficiente para desenhar
a lente"). Consertar mudaria o produto; aqui só registra-se, com as
três opções claras para o autor escolher numa próxima rodada.

### D4 — README + nota dentro da UI duplicam de propósito

A nota amarela na vista de raio e o README dizem coisas parecidas
sobre o achado 1. Por design: quem só abre o protótipo no navegador vê
o achado lá; quem só lê o repositório vê no README. Duplicação curta,
pequeno custo, dois leitores cobertos.

### D5 — Build release para os dumps

Capturar com release (`cargo build -p lente_cli --release`) economiza
~70% do tempo na extração do `egui` (o gargalo do laudo 0021 também). O
binário fica em `target/release/lente`. Os JSONs em si independem de
debug/release.

---

## Pendências cobertas / abertas

| Pendência | Estado pós-0029 |
|-----------|-----------------|
| Primeira superfície de UI | **Coberto** — protótipo em Arena. |
| Decidir se vale estender o contrato JSON | **Aberta com material para decidir** — achados 1 e 3 (profundidade barata; arestas caras). |
| Decidir o tratamento da divergência ranking/raio | **Aberta com 3 opções claras** — achado 2. |
| Nuclear um componente de UI | **Aberta** — depende do que a Arena ensinar. |
| Filtro de "folhas comportamentais" (Limite 3) | **Aberta** — trilha separada. |

---

## O que NÃO mudou (declaração explícita)

- **Workspace**: `Cargo.toml` raiz intocado; Arena fora do `members`.
- **Código de produção**: zero linhas tocadas.
- **CLI / wiring / catalogo / ranking / filtro**: zero toques.
- **Spec, ADRs, laudos pré-existentes**: zero toques.
- **Suíte de testes**: idêntica ao laudo 0028 (143 verdes + 15 ignored).
- **Subprocessos do cargo**: continuam dois únicos.
- **Pureza do L1**: intacta.

---

## Observação metodológica

Padrão da Arena reforçado: **medir, registrar, decidir depois**. Este
protótipo tem dois ganhos distintos:

1. **Direto**: a UI renderiza o dado. Bom mas pequeno.
2. **Lateral**: dois achados não-triviais — o contrato JSON é
   insuficiente para grafo (achado 1) **e** os dois pipelines da CLI
   classificam o mesmo nó de modos diferentes (achado 2). Ambos são
   coisas que **só apareceriam ao desenhar contra dado real**. Adivinhar
   no escuro custaria mais.

Coerente com o padrão dos laudos 0021 e 0027: a Arena prepara o
nucleamento, e o achado lateral muitas vezes vale mais que o entregável
direto.

---

## Histórico de Revisões

| Data | Motivo | Arquivos afetados |
|------|--------|-------------------|
| 2026-06-03 | Arena `lab/proto-ui/` criada: HTML+CSS+JS embarcados, sem CDN; 5 dumps reais (ranking egui top-30 + ranking lente_core top-15 + raio de Path/Kind/Classificacao). Vista de ranking ordenável + vista de raio (lista plana). Achados: (1) `impactados` sem profundidade nem arestas — vista de grafo precisa estender JSON; (2) classificação diverge entre `--ranking` (com `filtrar_stdlib`) e `--alvo` (sem filtro); (3) `impactados` ordenado alfabético. Zero mudança no produto. | `lab/proto-ui/{index.html,README.md,dados/*.json}`, `00_nucleo/lessons/0029-proto-ui-arena.md` |
