# CLAUDE.md — Projeto Lente de Forma e Consequência

Orientação para o Claude Code (e outros agentes) ao trabalhar neste projeto.
Este documento é um **mapa**: aponta para onde cada decisão vive, e registra
apenas as poucas convenções operacionais que não têm outro lar. Ele não repete
o conteúdo dos ADRs nem da spec — para não duplicar e divergir. A fonte causal
de cada decisão é o documento citado, não este.

---

## O que é este projeto

A **Lente de Forma e Consequência** responde, em até dez segundos, a uma única
pergunta sobre um sistema de código: **"o que quebra se eu mexer aqui?"** Ela
computa o raio de impacto estrutural a partir do grafo de dependências do
sistema. Não responde "vai realmente quebrar" — mostra o que está no raio de
impacto; o humano julga. Ver `proposta-lente.md` para o propósito completo.

O projeto **adota a Arquitetura Cristalina (Tekt)** como estrutura. O manifesto
do Tekt está copiado em `00_nucleo/MANIFESTO.md` (cópia local — o projeto é
auto-contido em suas razões). As regras do agente estão em `.agentrules`.

**Princípio que orienta o trabalho** (proposta, §10): dados primeiro,
visualização por último. Olha-se o que se consegue extrair, traduz-se, e só
então pensa-se em mostrar. Uma coisa de cada vez.

---

## Estrutura de `00_nucleo/` (L0 — a Semente)

```
00_nucleo/
├── MANIFESTO.md          # cópia local do manifesto Tekt
├── forma-organizada.md   # a spec central: a forma do grafo (NÃO em specs/)
├── adr/                  # decisões de arquitetura
├── prompt/               # prompts de componente (singular: prompt/, não prompts/)
└── lessons/              # laudos de execução (L5)
```

Atenção aos caminhos reais: a spec está em `00_nucleo/forma-organizada.md`
(direto, sem subpasta `specs/`); os prompts em `00_nucleo/prompt/` (singular).
Referências a `00_nucleo/specs/` em documentos antigos são erro de caminho.

---

## Onde cada decisão vive (ponteiros — não duplicar aqui)

| Assunto | Documento |
|---------|-----------|
| Propósito e escopo da lente | `proposta-lente.md` |
| Fonte do grafo (fork do cargo-modules, repo externo) | `00_nucleo/adr/0001-*.md` |
| Modelagem do grafo (enums fortes, entrada fiel, marca stdlib) | `00_nucleo/adr/0002-*.md` |
| A forma organizada (estrutura, campos, limites) | `00_nucleo/forma-organizada.md` |
| Laudos do que foi gerado e por quê | `00_nucleo/lessons/` |

Ao trabalhar num componente, leia a spec e os ADRs aplicáveis ANTES de gerar
código (trava de nucleação — ver `.agentrules`).

---

## A fonte de dados (resumo operacional)

O grafo vem de um **fork** do `cargo-modules` (decisão e razões no ADR-0001).
O fork é um **projeto externo** (repositório separado), não faz parte do
lattice deste projeto. O L3 (futuro) o invoca; este projeto não o versiona.

- **Repositório**: https://github.com/Dikluwe/cargo-modules
- **Instalação** (atenção à ressalva): exige o nome do pacote no fim, porque o
  repositório embarca fixtures-com-binário; sem o nome, `cargo install` aborta
  com "multiple packages with binaries found":
  ```
  cargo install --git https://github.com/Dikluwe/cargo-modules cargo-modules
  ```
- **Invocação canônica** (o que o L3 vai chamar):
  ```
  cargo modules export-json --sysroot --compact
  ```
  O `--sysroot` é obrigatório para fidelidade (sem ele, ~metade das arestas de
  um crate com derives some). É política da lente sempre usá-lo; o fork em si é
  neutro (default desligado). Razão completa: ADR-0001 e Limite 1 da spec.
- **Requisito de toolchain**: rust ≥ 1.91, edition 2024 (herdado do fork).

---

## Convenções operacionais deste projeto

Estas vivem aqui porque não têm outro documento próprio:

- **Linguagem**: Rust. O projeto-lente é escrito em Rust.
- **Testes**: convenção idiomática Rust — `#[cfg(test)] mod tests` no fim do
  mesmo arquivo do código, **não** um arquivo `arquivo.test.rs` separado. (O
  template do Tekt fala em `arquivo.test.ts` por ser pensado para TypeScript;
  em Rust, teste inline.) Todo componente gera código + testes inline.
- **Pureza L1**: zero I/O, zero dependências externas, só stdlib. Verificável
  por `cargo tree` (deve mostrar só o crate, sem dependências). L1 NÃO usa
  `serde` — desserialização de JSON é responsabilidade do L3.
- **Cabeçalho de linhagem**: todo arquivo de código aponta para o prompt que o
  originou (ver `.agentrules`, seção 4).

---

## Estado atual

**Existe**:
- L0: ADR-0001 (fonte), ADR-0002 (modelagem), spec `forma-organizada.md`
  (com 5 limites declarados), manifesto local.
- L1: tipo de dados da forma organizada (`01_core/src/entities/grafo.rs`) —
  enums `Relation`/`Visibility`/`Kind`, structs `No`/`Aresta`/`Grafo`, newtype
  `Path`, conversões texto→enum com erro para valor desconhecido. Compila,
  11 testes verdes, pureza confirmada. Crate `lente_core` em `01_core/`.

**Próximos componentes** (cada um nasce de prompt L0 próprio):
- Cálculo do raio (L1) — consome o tipo de dados, computa base/folha, alcance,
  profundidade. Deve herdar o Limite 4 da spec (piso de granularidade do `uses`
  via import: o raio para no módulo, não no item).
- Adaptador da fonte (L3) — invoca o fork, desserializa o JSON (aqui sim com
  serde), materializa o tipo de dados validando os enums na borda.
- Filtro de stdlib (L1) — recebe a forma completa, esconde o ruído de stdlib,
  respeitando a fronteira delicada do Limite 2 (preservar impls do crate-alvo).

**Decisões pendentes registradas**:
- **Workspace Cargo na raiz**: quando surgir um segundo crate (L2/L3/L4), faz
  sentido um workspace que agrupe os estratos. Hoje só existe `01_core/` como
  crate único. Decisão adiada até haver o segundo crate (laudo 0001, D1).
- **Categoria Tekt para "fork externo mantido"**: a Arena (`lab/`) é para código
  descartável, não cobre código externo estável. Lacuna do Tekt observada;
  registrada no ADR-0001. Evolução do framework, não bloqueia o projeto.
