# Prompt: Re-execução da Remedição (ChaveAresta Corrigido)

**Tipo**: Experimento de Arena (`lab/`) — re-execução
**Criado em**: 2026-05-27
**Decisões de origem**: laudo 0008 (correção do `ChaveAresta`); laudo da
remedição (`lab/medicao-colisoes/remedicao/relatorio.md`, §6).
**Pré-requisito**: `lente_investiga` com `ChaveAresta` corrigido (laudo 0008,
17 testes verdes). O programa de medição já existe em
`lab/medicao-colisoes/remedicao/`.

---

## Contexto

A segunda medição (`lab/medicao-colisoes/remedicao/relatorio.md`) rodou com o
fork novo (com `id`), mas o `lente_investiga` ainda tinha o bug do
`ChaveAresta` (chave por path, não por id) — o que contaminou os resultados
da Estratégia 1 com falsos `MesmoItem`. O bug foi corrigido (laudo 0008).

Esta re-execução roda a mesma medição com o `lente_investiga` corrigido, para
obter os números limpos da Estratégia 1.

---

## O que fazer

### 1. Re-executar o programa existente

O programa de medição já existe em `lab/medicao-colisoes/remedicao/`. Ele
depende do `lente_investiga` por path, então recompilar pega a versão
corrigida automaticamente.

- **Recompilar e rodar** o programa contra os mesmos 17 crates do typst.
- **Reaproveitar os JSONs** já gerados em
  `lab/medicao-colisoes/remedicao/json/` se eles ainda existem e foram
  gerados com o fork novo (commit 5fbcdfe8). Não precisa regerar os JSONs —
  o fork não mudou, só o `lente_investiga` mudou. Se os JSONs não existirem
  mais, regerar com `cargo modules export-json --sysroot --compact
  --package <nome>`.
- A análise (parsing + cascata) roda de novo, agora chamando o
  `lente_investiga` corrigido.

### 2. Gerar o relatório com comparação tripla

Atualizar (ou criar nova versão de) o relatório. O ponto central é a
**comparação das três medições** numa tabela:

| Métrica | 1ª medição (fork antigo) | 2ª medição (fork novo, bug) | 3ª medição (fork novo, corrigido) |
|---------|--------------------------|------------------------------|-----------------------------------|
| Total de colisões | 384 | 384 | ? |
| Decididas por E1 | 0 (inaplicável) | 116 (30.2%, contaminado) | ? |
| Decididas por E2 | 55 (14.3%) | 23 (6.0%) | ? |
| NaoDeterminado | 329 (85.7%) | 245 (63.8%) | ? |
| Total decidido | 55 (14.3%) | 139 (36.2%) | ? |

O delta entre a 2ª e a 3ª medição **isola o efeito do bug** — quantos dos
116 falsos `MesmoItem` viraram `Distintos` corretos, quantos se moveram para
`NaoDeterminado`, etc.

### 3. Análise específica do que mudou

Além da tabela, o relatório deve responder:

- **Distribuição dos vereditos E1 agora**: quantos `MesmoItem` vs.
  `Distintos/VizinhancaDisjunta`? Na 2ª medição era 114 `MesmoItem` / 2
  `Distintos` (suspeito). Agora deve inverter ou mudar bastante.
- **Os 29 casos `::fmt`** que a 2ª medição apontou como falsos `MesmoItem`:
  como são classificados agora? Devem virar `Distintos` (a correção é
  exatamente para isso). Confirmar.
- **O critério rígido ainda morde?** A 2ª medição estimou que relaxar o
  critério (`compartilhadas == 0` → "ambos com exclusivas") levaria a E1 de
  30% para ~75%. Com a chave corrigida, recalcular essa estimativa: quantos
  dos NaoDet atuais têm `exc_a > 0` e `exc_b > 0` (passariam sob critério
  relaxado)? Este número informa a próxima decisão (relaxar ou não).

### 4. Re-avaliar as hipóteses e os cenários do ADR-0004

Repetir a avaliação de H1/H2/H3 e dos três cenários (A/B/C) do ADR-0004,
agora com os números limpos. Especialmente: a correção do bug muda a
conclusão sobre o Cenário A (E1 resolve a maioria)?

---

## Restrições

- **Não modificar `lente_core`, `lente_infra`, `lente_investiga`.** Medição
  contra o estado pós-laudo 0008.
- **Não relaxar o critério ainda.** Esta medição usa o critério atual
  (`compartilhadas == 0`); só **estima** o impacto do relaxamento por
  contagem, como a 2ª medição fez. A decisão de relaxar é posterior.
- **Não tocar a pasta da 1ª medição.** A 3ª medição atualiza ou versiona
  o relatório dentro de `lab/medicao-colisoes/remedicao/`.
- **Sem alterações em documentos L0.** Decisões decorrentes ficam para o
  autor.

---

## Resultado esperado

- Programa re-executado, análise refeita com a chave corrigida.
- Relatório com a tabela de comparação tripla, a distribuição de vereditos
  E1 atualizada, a confirmação dos casos `::fmt`, e a estimativa recalculada
  do critério relaxado.
- Re-avaliação das hipóteses e dos cenários do ADR-0004.
- A interpretação fica com o autor; o relatório descreve.

Não é necessário laudo separado em `00_nucleo/lessons/` (é re-execução de
Arena, não nucleação de componente) — o próprio relatório atualizado é o
registro.

---

## A pergunta que esta medição finalmente responde

Depois de três medições, a pergunta central fica respondível com dado limpo:
**a identidade-por-nó, com a chave corrigida, faz a Estratégia 1 resolver a
maioria das colisões?** Se sim, o ADR-0004 está validado e o caminho para o
`lente_resolve` está aberto. Se não — se mesmo com a chave correta a maioria
fica indecidida — então a questão do critério rígido (relaxar ou não) passa
a ser a decisão central, e o relatório deve deixar claro o tamanho exato
desse efeito.
