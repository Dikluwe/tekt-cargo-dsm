# Prompt de Nucleação: mapa de correspondência TOML (L3)

Hash do Código: 230697b2

**Camada:** L3 — infraestrutura.
**Unidade:** `03_infra/src/mapa_correspondencia.rs`.
**Origem de trabalho:** `00_nucleo/prompt/0079-mapa-correspondencia.md`.

## Propósito

Ler o arquivo TOML opcional do `--comparar`, preservar sua proveniência por
caminho e SHA-256 e traduzir a sintaxe para o contrato puro de
`lente_comparacao`. Esta unidade valida somente bytes, TOML, versão e enums; a
existência de paths, cardinalidade e conflitos pertencem ao L1.

## Invariantes

- aceita somente `tekt_comparison_map = 1`;
- I/O, UTF-8, TOML, versão e valores desconhecidos produzem erro de borda;
- `unknown` é um valor explícito de alegação, nunca fallback de parser;
- SHA-256 cobre exatamente os bytes lidos;
- ordem das tabelas não recebe significado semântico.

## Critérios de verificação

```text
Dado TOML v1 válido Então produz MapaCorrespondencia e SHA-256 dos bytes
Dado versão diferente de 1 Então ErroMapaCorrespondencia::Versao
Dado relacao ou alegacao desconhecida Então ErroMapaCorrespondencia::Valor
Dado arquivo ilegível Então ErroMapaCorrespondencia::Io
```

## Interface Snapshot
<!-- GENERATED — não edite manualmente -->
<!-- crystalline-snapshot: {} -->

## Histórico de Revisões

| Data | Motivo | Arquivos afetados |
|------|--------|-------------------|
| 2026-08-26 | P0079: parser e proveniência do mapa declarativo. | `03_infra/src/mapa_correspondencia.rs` |
