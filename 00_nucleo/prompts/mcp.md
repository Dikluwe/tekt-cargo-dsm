# Prompt de Nucleação: `lente_mcp` — a boca MCP (L4)
Hash do Código: d6c4f79c

**Camada**: L4 — Fiação (ponto de entrada). Importa L4 (`lente_wiring`), L2
(`lente_cli`) e L1 (`lente_core`); compõe, não cria domínio.
**Unidade**: `04_wiring/mcp/src/main.rs` (crate `lente_mcp`, binário `lente-mcp`).
**Origem de trabalho** (referência): `00_nucleo/prompt/0070-boca_mcp.md`.

> Prompt de **nucleação** (descreve o código existente).

---

## Propósito

A **boca MCP** da lente (Momento B da proposta §4): um servidor por **stdio** que
anuncia ferramentas e as executa, para um agente perguntar "o que quebra se eu
mexer aqui?" **antes** de propor uma mudança, e o humano ver o raio **antes** de
aprovar. É uma boca nova sobre pipelines prontos — **não** muda o cálculo nem a
serialização: devolve **o mesmo JSON** que a CLI (`lente_cli`), do mesmo pipeline
(`lente_wiring`).

## Comportamento e invariantes

- **Protocolo**: JSON-RPC 2.0 **à mão** sobre stdio (uma mensagem por linha),
  síncrono, sem SDK nem async (razão no laudo 0070). Trata `initialize`
  (negocia versão — ecoa a do cliente), `notifications/initialized` (sem
  resposta), `tools/list`, `tools/call`, `ping`. Método desconhecido → `-32601`;
  JSON inválido → `-32700`.
- **Ferramentas** (3): `impacto_do_diff` (→ `analisar_diff`, JSON do
  `ResultadoDiff`), `raio_do_alvo` (→ `calcular_raio_de_alvo`, JSON do `Raio`,
  com `escopo`), `ranking` (→ `rankear_pacote`). As **descrições declaram o
  limite estrutural-não-comportamental** (proposta §3) — interface, não só doc.
- **Erros**: falha de pipeline (`ErroLente`) vira resultado com `isError: true` e
  a mensagem do `ErroLente` (Display) — não panica, não silencia. Validação de
  argumentos (fonte/alvo ausente ou ambíguo) também vira `isError: true`.
- **stdout é sagrado**: só protocolo; logs (se houver) em stderr.
- **Sem estado**: cada chamada roda o pipeline do zero. Latência (frio ~36s /
  quente ~0,07s, laudo 0070) fica registrada; cache é prompt futuro.

## Restrições (L4)

- O **topo**: importa as camadas abaixo (compõe). **Gravidade preservada**: nada
  de L1/L2/L3 depende deste crate. **Deps externas do protocolo só aqui**
  (`serde_json`, já do workspace). `main`/funções não são `pub` → snapshot vazio.

## Critérios de Verificação

```
Dado initialize com protocolVersion Quando tratar Então o resultado ecoa a versão
Dado tools/list Quando tratar Então as 3 ferramentas com o limite estrutural no texto
Dado tools/call de ferramenta desconhecida Então isError: true (não erro JSON-RPC)
Dado raio_do_alvo sem fonte Então isError: true (valida antes do pipeline; não panica)
Dado E2E por stdio (initialize→call impacto_do_diff) Então o conteúdo é o JSON do 0047
```

## Interface Snapshot
<!-- crystalline-snapshot: {"functions":[],"types":[],"reexports":[]} -->

## Histórico de Revisões

| Data | Motivo | Arquivos afetados |
|------|--------|-------------------|
| 2026-06-10 | Nucleação (prompt 0070) da boca MCP: binário `lente-mcp` (L4) servindo `impacto_do_diff`/`raio_do_alvo`/`ranking` por JSON-RPC/stdio à mão; reusa `lente_wiring` (pipelines) + `lente_cli` (montagem JSON); descrições declaram o limite estrutural. | `04_wiring/mcp/src/main.rs`, `04_wiring/mcp/Cargo.toml` |
