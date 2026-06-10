# Prompt: escopo `seu-codigo` como default das superfícies de consumo

**Camada**: L2 (defaults da vista HTML e rótulos) + L4 (defaults/descrições da
boca MCP; possivelmente o pipeline do diff, ver Fase 1). O cálculo **não muda**;
o filtro de escopo **já existe** (0025/0030) — este prompt muda **qual lado é o
default** em duas superfícies, não o que é possível.
**Criado em**: 2026-06-10
**Estado**: `EXECUTADO` (laudo `00_nucleo/lessons/0072-escopo_padrao_consumo.md`)
**Decisões de origem**:
- **Laudo 0030** — o escopo é **escolha do usuário**, com a saída sempre
  **declarando** o escopo ativo. **Este prompt revisita o default, não a
  escolha**: a flag continua existindo nos dois sentidos; o que muda é o lado
  que vem sem pedir, em duas superfícies específicas. A revisita não é
  silenciosa — é justificada pela evidência abaixo e registrada aqui.
- **Laudo 0070, sinalização** — a boca do agente: o JSON do diff em escopo
  `completo` traz `combinado.jusante` com dezenas de paths de sysroot; "o
  agente quer quantos/quais tocados, não a lista crua de stdlib".
- **Laudo 0071, Fase 3** — a vista do humano: as matrizes saíram em `completo`
  e `core::fmt` etc. poluíram a leitura do egui; o laudo registra que
  `seu-codigo` "é mais limpo — só não foi o default".
- **Dois consumidores independentes** (agente e humano), em uso real, pediram
  o mesmo recorte. A disciplina do projeto é decidir por uso, não por
  argumento — o uso falou duas vezes.
**Pré-requisito**: estado pós-0071 (vista HTML entregue; boca MCP entregue;
suíte 289 + ignorados verdes).
**Arquivos afetados (a confirmar na Fase 1)**: `02_shell/cli` (default do
`--html`; rótulos), `02_shell/catalogo` (textos), `04_wiring/mcp` (defaults +
descrições), possivelmente `04_wiring` (escopo no pipeline do diff), testes.

---

## Contexto

O filtro de stdlib/sysroot existe desde o 0025 e o 0030 o transformou em
escolha explícita com saída rotulada. As duas superfícies de **consumo**
nascidas depois — a vista HTML (0071, para o humano) e a boca MCP (0070, para
o agente) — herdaram o default `completo`, e o primeiro uso real de ambas
registrou o mesmo incômodo: o sinal afogado em sysroot.

A mudança é de **default, por superfície**:

| Superfície | Default hoje | Default proposto |
|---|---|---|
| `--estrutura --html` (vista humana) | `completo` | **`seu-codigo`** |
| MCP `raio_do_alvo`, `ranking` | `completo` | **`seu-codigo`** |
| MCP `impacto_do_diff` | `completo` (sem parâmetro) | **`seu-codigo`** + parâmetro `escopo` (ver Fase 1) |
| CLI `--text` / `--json` | `completo` | **inalterado** |

A CLI texto/JSON fica como está por duas razões: é o contrato que agentes e
testes já consomem com contagens ancoradas (mudar o default mudaria números
sob os pés de consumidores existentes), e preserva o espírito do 0030 — quem
chama a CLI crua está mais perto da máquina e escolhe. As superfícies de
consumo escolhem o lado humano/agente por default. **Nos dois casos, a saída
continua declarando o escopo ativo** (0030, inalterado) e o outro lado continua
a uma flag/parâmetro de distância.

---

## Fase 1 — Leitura e verificação (obrigatória)

1. **Os nomes e a semântica reais das flags.** Ler `args.rs` e confirmar como
   escopo e modo de uses se chamam hoje (`--filtrar-stdlib`? `--escopo`?
   `--so-referencia` é modo de uses, não escopo — não confundir os dois eixos).
   O prompt usa "escopo `seu-codigo`" como conceito; os nomes de produto são os
   que existem. Se a inversão exigir uma flag nova (ex.: `--escopo completo`
   para restaurar o lado antigo na vista), registrar a escolha de nome no
   catálogo.
2. **Escopo no pipeline do diff.** Verificar se `analisar_diff` já passa pelo
   filtro de escopo (o filtro do 0025 opera sobre o grafo; o grafo de
   workspace do 0045 passa por ele?). **Se** a tubulação existe, expor o
   parâmetro `escopo` no `impacto_do_diff` com default `seu-codigo`. **Se
   exigir peça nova**, registrar o desenho e deferir — o `impacto_do_diff`
   fica como está, com a falta anotada (mesmo padrão do workspace no 0071).
3. **Quem quebra.** Levantar os testes que afirmam saída das superfícies
   afetadas (E2E do `--html`, unidade/E2E da boca MCP) e os que afirmam o
   default antigo — a lista do que vai ser reancorado, antes de mexer.

---

## Fase 2 — Construção

1. **Vista HTML**: default `seu-codigo`; a flag existente restaura `completo`;
   o cabeçalho da vista (que já declara o escopo, 0071) passa a destacar
   quando o escopo é o filtrado — ex.: "escopo: seu-codigo (sysroot/stdlib
   ocultos; use <flag> para o completo)" — texto no catálogo.
2. **Boca MCP**: `raio_do_alvo` e `ranking` com default `seu-codigo` no
   parâmetro `escopo`; `impacto_do_diff` conforme a Fase 1 (parâmetro +
   default, ou deferido com registro). **As descrições das ferramentas**
   (interface do agente, 0070) passam a declarar o default e como pedir
   `completo` — a honestidade do recorte vai no contrato, como a do limite
   estrutural já vai.
3. **Rotulagem**: toda resposta (HTML e MCP) continua carregando o campo/texto
   de escopo — verificar que nenhuma das superfícies o omite após a inversão.
4. **Testes**: os defaults novos afirmados (vista sem flag → sem sysroot na
   grade; MCP sem parâmetro → JSON com `escopo` declarado como `seu-codigo`);
   o caminho explícito para `completo` afirmado (restaura o comportamento
   antigo byte-a-byte onde aplicável); os testes da lista da Fase 1 item 3
   reancorados **com nota** de que a âncora mudou por este prompt.

---

## O que NÃO fazer

- **Não mudar o default da CLI `--text`/`--json`** — contrato existente.
- **Não remover nem renomear o escopo `completo`** — a escolha do 0030
  permanece; só o default das superfícies de consumo muda.
- **Não mexer no filtro em si** (0025) nem no cálculo — a inversão é de
  apresentação/entrada, não de motor.
- **Não construir tubulação nova para o diff** se a Fase 1 mostrar que não
  existe — registrar e deferir.
- **Não tocar o modo de uses** (`referência` vs `todas`) — eixo separado,
  fora deste prompt.

---

## Critérios de Verificação

```
Dado lente --pacote <X> --estrutura --html sem flag de escopo
Então a grade sai sem módulos de sysroot/stdlib e o cabeçalho declara o
escopo filtrado e como obter o completo

Dado a mesma chamada com a flag de escopo completo
Então o comportamento anterior ao prompt, byte-a-byte no dado

Dado tools/call raio_do_alvo ou ranking sem escopo
Então o JSON sai em seu-codigo, com o escopo declarado no payload; com
escopo=completo, o comportamento antigo

Dado tools/list
Então cada descrição declara o default de escopo e como pedir o completo

Dado impacto_do_diff
Então OU recebe escopo com default seu-codigo (tubulação existente, Fase 1)
OU está registrado no laudo o desenho do que falta, sem mudança

Dado a CLI --text/--json
Então defaults inalterados, byte-iguais ao pré-prompt

Dado a suíte e o linter
Então verde com os testes reancorados anotados; V1 = 0, V2 = 0 preservados;
V12 = 1 inalterado; nenhuma dep nova
```

---

## Resultado esperado

- As duas superfícies de consumo entregando por default o recorte que os
  laudos 0070 e 0071 mediram como o útil, com o `completo` a uma flag de
  distância e sempre declarado.
- O `impacto_do_diff` com escopo (ou o desenho registrado do que falta).
- A decisão do 0030 revisitada **com registro**: a escolha do usuário
  permanece; o default mudou onde o uso real pediu, e este laudo é a linha
  de procedência dessa mudança.
- **Laudo** em `00_nucleo/lessons/0072-…`: os nomes reais das flags, o
  resultado da verificação do diff, a lista de testes reancorados, e — se o
  uso seguinte mostrar que o default novo esconde algo que importava — o
  caminho de volta é uma linha, porque nada foi removido.

---

## Cuidados

- **Mudança de default é mudança de contrato** — por isso este prompt é
  separado e pequeno: um eixo, duas superfícies, procedência citada (0030 ⊕
  evidência 0070/0071).
- **Nada se torna impossível** — o critério de aceitação inclui o caminho
  explícito de volta ao `completo` em toda superfície tocada.
- **Não confundir os eixos** escopo × modo de uses na Fase 1 — nomes parecidos,
  semânticas diferentes.

---

## Histórico de Revisões

| Data | Motivo | Arquivos afetados |
|------|--------|-------------------|
| 2026-06-10 | Escopo `seu-codigo` como **default das superfícies de consumo**: vista HTML (`--estrutura --html`, 0071) e boca MCP (`raio_do_alvo`/`ranking`; `impacto_do_diff` ganha parâmetro `escopo` se a tubulação do filtro 0025 alcançar o grafo de workspace 0045 — verificado na Fase 1, deferido com desenho se não). CLI `--text`/`--json` **inalterada** (contrato existente, contagens ancoradas). Revisita **registrada** do default do 0030 — a escolha do usuário permanece (o `completo` fica a uma flag/parâmetro, sempre declarado na saída; caminho de volta byte-a-byte testado); muda só o lado que vem sem pedir, justificado por dois consumidores independentes em uso real (sinalização do 0070: jusante afogado em sysroot no JSON do diff; Fase 3 do 0071: `core::fmt` poluindo a matriz do egui). Descrições MCP passam a declarar o default (contrato do agente, padrão 0070); cabeçalho da vista destaca o recorte e como desfazê-lo (catálogo). Testes: defaults novos afirmados, caminho explícito `completo` afirmado, reancorados anotados. Sem mudança de motor/filtro/deps; modo de uses fora do escopo. | `02_shell/cli` (default + cabeçalho), `02_shell/catalogo` (textos), `04_wiring/mcp` (defaults + descrições), possivelmente `04_wiring` (escopo no diff), testes, `00_nucleo/lessons/0072-escopo_padrao_consumo.md` |
