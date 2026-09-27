# Verificação real de navegador — 2026-09-26

Vista executada no navegador integrado, com servidor local real e APIs Rust.
Inspeção visual por screenshots da página, não apenas por leitura do HTML.

| Cenário | Procedimento | Resultado observado |
|---|---|---|
| Seleção | Clicar ui → service na fixture partial | Peso 3; import #11 e referências #9/#10; IDs/paths originais. |
| Direção | Ler cabeçalhos e selecionar ui → storage | Linha consome coluna; célula vazia nessa direção, com ressalva sobre o snapshot. |
| Teclado | Na célula ui → service, Direita + Enter | Foco e seleção foram para ui → storage; evidências anteriores removidas. |
| Teclado | Esquerda + Espaço, depois Tab | Seleção voltou a ui → service; Tab saiu da matriz para o primeiro botão de evidência. Apenas uma célula com tabindex=0. |
| Evidência → consulta | Clicar “Investigar cadeia destes itens” na referência #9 | Seletores preenchidos com 11 → 21; caminho direto, um salto. |
| Cadeia | Consumidor 11, alterado 31 | render → load → save, dois saltos, direção consumidor → alterado. |
| Escape | Expandir exclusões e diagnósticos | `<script>`, `</script><img…>`, `<&>`, Unicode e aspas aparecem literalmente; zero elementos img e apenas o script local /app.js. |
| Posição | Abrir as três evidências | “Local de uso: não disponível”; declaração do nó explicitamente não substitui referência. |
| Parcial | Rolar até os diagnósticos | Aviso parcial permanece fixado a 8px do topo; cobertura semântica continua declarada. |
| Exclusões | Expandir cobertura | 1 interna, 10 não agregáveis (incluindo Owns), 1 item órfão; motivos legíveis. |
| Paginação | Fixture pagination; selecionar peso 45 | Zero cards antes da seleção; 20 após seleção; 40 e 45 após duas ativações. Botão escondido ao final. |
| Vazio | Fixture empty | Zero módulos; “Sem células para selecionar”; consulta desabilitada; completude desconhecida, nunca completa. |

## Reprodução

Prepare cada fixture com o binário conforme README. Para comparar sessões, use
portas distintas (8765/8766/8767). Nenhuma instalação de ferramenta de navegador
é necessária para o roteiro manual. A verificação de UI não integra os testes
Python: os sete testes Python exercitam backend/HTTP; quatro testes Rust exercitam
preparação/consulta. As ações acima foram realizadas no navegador nesta entrega.

A captura `../artifacts/dsm-explicavel.png` registra a matriz, evidências, cadeia
e painel de limites. A inspeção também incluiu a página vazia e a seção de
exclusões expandida. Não foi feita auditoria completa com leitor de tela nem
validação de escala em grandes workspaces. O CSS tem layout responsivo, mas esta
rodada de inspeção foi em desktop.
