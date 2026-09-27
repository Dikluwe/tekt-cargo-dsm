# AGENTS.md — Projeto Lente de Forma e Consequência

Orientação para o Codex (e outros agentes) ao trabalhar neste projeto.
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
impacto; o humano julga. Ver `docs/proposta-lente.md` para o propósito completo.

O projeto **adota a Arquitetura Cristalina (Tekt)** como estrutura. O manifesto
do Tekt está copiado em `00_nucleo/MANIFESTO.pt.md` (cópia local — o projeto é
auto-contido em suas razões). A configuração arquitetural está em
`crystalline.toml`. Não há `.agentrules` neste checkout.

**Princípio que orienta o trabalho** (proposta, §10): dados primeiro,
visualização por último. Olha-se o que se consegue extrair, traduz-se, e só
então pensa-se em mostrar. Uma coisa de cada vez.

---

## Estrutura de `00_nucleo/` (L0 — a Semente)

```
00_nucleo/
├── MANIFESTO.pt.md       # cópia local do manifesto Tekt
├── specs/forma-organizada.md # a spec central: a forma do grafo
├── adr/                  # decisões de arquitetura
├── prompt/               # prompts históricos de trabalho
├── prompts/              # contratos de nucleação por arquivo (desde 0058)
└── lessons/              # laudos de execução (L5)
```

A spec está em `00_nucleo/specs/forma-organizada.md`. As duas pastas de prompts
coexistem por decisão do laudo 0058: consulte o `@prompt` do arquivo antes de
alterá-lo; não renomeie nem mova os prompts históricos.

---

## Onde cada decisão vive (ponteiros — não duplicar aqui)

| Assunto | Documento |
|---------|-----------|
| Propósito e escopo da lente | `docs/proposta-lente.md` |
| Fonte do grafo (fork do cargo-modules, repo externo) | `00_nucleo/adr/0001-*.md` |
| Modelagem do grafo (enums fortes, entrada fiel, marca stdlib) | `00_nucleo/adr/0002-*.md` |
| A forma organizada (estrutura, campos, limites) | `00_nucleo/specs/forma-organizada.md` |
| Laudos do que foi gerado e por quê | `00_nucleo/lessons/` |

Ao trabalhar num componente, leia a spec e os ADRs aplicáveis ANTES de gerar
código, além do contrato indicado no cabeçalho `@prompt`.

---

## A fonte de dados (resumo operacional)

O grafo vem de um **fork** do `cargo-modules` (decisão e razões no ADR-0001).
O fork é um **projeto externo** (repositório separado), não faz parte do
lattice deste projeto. O L3 o invoca; este projeto não o versiona.

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
- **Pureza L1**: zero I/O e zero dependências de terceiros em produção;
  stdlib e crates locais L1. Verificável por `cargo tree --edges normal`. L1 NÃO usa
  `serde` — desserialização de JSON é responsabilidade do L3.
- **Cabeçalho de linhagem**: todo arquivo de código aponta para o prompt que o
  originou (cabeçalho `Crystalline Lineage`, com `@prompt`).

---

## Estado atual (verificado em 2026-09-26)

O `Cargo.toml` raiz agrupa 13 crates (ADR-0003): sete em L1, dois em L2,
um em L3 e três em L4. O tipo do grafo vive em
`01_core/core/src/entities/grafo.rs`. Já existem raio, resolução, filtros,
ranking, estrutura, comparação, adaptador, CLI e servidor MCP.

Consulte os contratos de `00_nucleo/prompts/` e os laudos para cada componente.
O diagnóstico e as limitações verificadas na auditoria estão em
`00_nucleo/lessons/0080-auditoria-2026-09-26.md`.
