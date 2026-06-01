# ADR-0003: Workspace Cargo com cada estrato como crate

**Status**: `PROPOSTO`
**Data**: 2026-05-27

---

## Contexto

O projeto-lente nasceu com um único crate (`lente_core`, em `01_core/`),
contendo o tipo de dados e o cálculo do raio — ambos L1, ambos puros (sem
dependências externas, conforme verificado por `cargo tree`).

O próximo componente é o adaptador L3 (invocar o fork, ler JSON, materializar o
`Grafo`). Pela natureza de L3 (operação de I/O, leitura de subprocesso,
desserialização de JSON), ele exige dependências externas — pelo menos
`serde`/`serde_json` e algum mecanismo de execução de processo.

Isso cria uma tensão estrutural: a pureza de `lente_core` (verificada
mecanicamente) não pode coexistir com dependências externas no mesmo crate. Ou
o L3 vive fora de `lente_core`, ou a pureza de L1 deixa de ser verificável por
`cargo tree`.

A questão já tinha sido sinalizada como pendência arquitetural no laudo do
primeiro componente (Decisão Tácita 1): "se cada estrato (L1, L2, L3, L4)
virar um crate separado, faz sentido um workspace Cargo na raiz que os
agrupe". O L3 força a decisão.

Esta decisão **transcende um componente** — não é só sobre o L3, é sobre como o
projeto-lente cresce a partir de agora. Por isso é registrada aqui como ADR e
não no histórico de um prompt isolado.

---

## Decisão

O projeto-lente adota **workspace Cargo**, com cada estrato Tekt que tenha
necessidade própria de dependências (ou propósito próprio de compilação) como
**crate Cargo distinto**. A partir de agora, a raiz do projeto é um workspace;
`lente_core` deixa de ser o projeto e passa a ser um membro do workspace.

A estrutura resultante:

```
projeto-lente/
├── Cargo.toml          # workspace, lista os membros
├── 00_nucleo/          # L0 (Markdown, fora do Cargo)
├── 01_core/            # crate `lente_core` (membro do workspace)
│   └── Cargo.toml
├── 03_infra/           # crate `lente_infra` (novo, membro do workspace, para o L3)
│   └── Cargo.toml
└── (futuros)           # 02_shell/, 04_wiring/, conforme surgirem
```

Detalhes:

1. **`lente_core` permanece puro.** Continua sem `serde`, sem dependências
   externas, verificável por `cargo tree -p lente_core` (deve mostrar só o
   crate). Não muda código, só passa a ser membro do workspace.

2. **`lente_infra` é o crate de L3.** Depende de `lente_core` (para usar os
   tipos `Grafo`, `No`, etc.) e das bibliotecas externas que o L3 precisa
   (`serde`, `serde_json`, e qualquer outra que o adaptador exija). É o lugar
   onde o `serde` finalmente entra no projeto.

3. **Cada futuro estrato com necessidade própria de compilação vira um crate.**
   Se o L2 (mostrar) tiver dependências (provável — UI tende a tê-las), vira
   `lente_shell` em `02_shell/`. O L4 (composição) tende a ser um crate
   binário (`main()`) que depende dos outros.

4. **Gravidade Tekt em estrutura Cargo**: as dependências entre crates do
   workspace seguem a gravidade do lattice. `lente_core` depende de nada
   (interno); `lente_infra` depende de `lente_core`; um futuro `lente_shell`
   dependerá de `lente_core`; o L4 (wiring) dependerá de todos. Esta é a
   única forma de dependência permitida entre crates do projeto.

5. **Nomes**: o padrão é `lente_<estrato>` (snake_case obrigatório do Cargo).
   `lente_core` já existe; `lente_infra` para L3; `lente_shell` (sugestão) para
   L2; `lente_wiring` (sugestão) para L4.

---

## Prompts Afetados

| Prompt / artefato | Como esta decisão o molda |
|-------------------|---------------------------|
| Prompt do L3 (adaptador) | Cria o crate `lente_infra` em `03_infra/`, declarado no workspace. Importa tipos de `lente_core`. |
| (futuro) Prompt do L2 | Se tiver dependências, vira `lente_shell` em `02_shell/`. |
| (futuro) Prompt do L4 | Crate binário, geralmente o ponto de entrada. |
| `lente_core` existente | Não muda código. Passa a ser membro de um workspace; o `Cargo.toml` da raiz declara `members = ["01_core", "03_infra", ...]`. |

---

## Consequências

**Positivas**:
- A pureza de cada estrato fica verificável separadamente. `cargo tree -p
  lente_core` continua mostrando só o crate; quem auditar a pureza de L1 olha
  ali, e pronto.
- A gravidade do Tekt vira gravidade do Cargo: dependências entre crates do
  workspace só podem fluir do estrato mais alto para o mais baixo. Tentar
  inverter falha na compilação. O Cargo passa a ser linter parcial da
  arquitetura.
- Estratos podem evoluir independentemente — versão, testes, build. Cada um é
  uma unidade própria.
- A pendência arquitetural do laudo 0001 (D1) fica resolvida.

**Negativas**:
- Mais arquivos (`Cargo.toml` na raiz e em cada crate). É o custo aceito da
  separação.
- O ponto de entrada (binário, futuro L4) precisa ser explicitamente um crate
  binário que importa os outros — não dá para "rodar a lente" de dentro de um
  crate biblioteca isolado.

**Neutras**:
- O `cargo test` na raiz roda os testes de todos os membros — comportamento
  natural do workspace, sem trabalho adicional.

---

## Alternativas Consideradas

| Alternativa | Por quê foi rejeitada |
|-------------|----------------------|
| Crate único `lente_core` com módulos para L3 | Viola a pureza de L1 (serde entra no crate, contamina `cargo tree`). Mistura estratos no mesmo crate, ferindo a gravidade do Tekt. |
| Módulo dentro de `lente_core` atrás de feature flag | Pureza condicional confunde o auditor: `cargo tree` mostra dependências dependendo de qual feature está ligada. A pureza de L1 deve ser propriedade do crate, não de uma configuração de compilação. |
| Crates separados sem workspace (cada um com seu repo) | Excessivo. Os crates estão acoplados pelo propósito comum do projeto-lente; manter em repositórios separados criaria fricção de versionamento sem benefício real. |

---

## Referências

- Laudo 0001 (D1) — pendência arquitetural do workspace
- ADR-0002 — pureza de L1 como invariante a preservar
- `CLAUDE.md` — convenções operacionais do projeto
- `.agentrules` — regras de gravidade Tekt
