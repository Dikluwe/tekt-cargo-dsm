# Harness Doxygen com executáveis reais

Contrato: [CONTRATO.md](CONTRATO.md). Comandos completos de build, execução,
CLI/MCP e DSM: [guia operacional](../../docs/doxygen.md).

```sh
python3 lab/doxygen-integracao/test_integracao.py
python3 lab/doxygen-integracao/test_integracao.py --real-doxygen --output-dir /tmp/doxygen-rodada-nova
```

O primeiro usa temporário exclusivo e o remove ao terminar. O segundo conserva
artefatos e `results.json` em diretório novo para inspeção. Nunca conta ferramenta
ausente como aprovação. `--real-doxygen` autoriza só a fixture controlada, aceita
somente a versão instalada 1.9.8, sem instalação ou downloads.

`fixtures/xml` é manuscrito, baseado no perfil oficial 1.9.8 documentado no
importador. `graph.json` é um grafo estrutural sintético no formato real da Lente;
`catalog-evidence.json` fornece nomes/localizações deliberadamente explícitos,
independentes do inventário, sem inferência de path opaco. `real.hpp` é código
C++ mínimo próprio, fornecido ao Doxygen somente no teste real. Não é compilado
nem executado. A associação real usa a declaração conhecida na linha 4.

Checks: todos os quatro binários, projeção sobre grafo, hashes dos bytes, matching
confirmado/ambíguo/insuficiente, sobrecargas, texto vazio, Unicode, pendências,
parcialidade, métrica ausente versus zero, repetição determinística, saída existente,
XML inválido, erro schema/digest/ferramenta ausente em CLI/MCP, DSM incompatível.
Nenhum JSON substituto simula a saída de uma ferramenta.

Resultados executados: [RESULTADOS.json](RESULTADOS.json) e
[laudo completo](../../00_nucleo/lessons/2026-09-26-integracao-doxygen.md), incluindo
contagens, verificação visual e pendências globais preservadas.
