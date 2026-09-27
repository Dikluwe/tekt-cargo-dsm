# Resultados locais — 2026-09-26

Área alterada por este trabalho: somente `lab/avaliacao-adaptadores/`.
Os adaptadores foram executados como estão, sem instalação, downloads, recompilação
ou modificação. Dependências locais da Lente são utilizadas pelo avaliador isolado.

## Verificação

- 16 testes Rust do executor aprovados, nenhum ignorado.
- Clippy em todos os targets com `-D warnings`: aprovado.
- `cargo fmt --check`: aprovado.
- 36 cenários executáveis aprovados: 6 por adaptador.
- 3 repetições por cenário, total de 108 amostras preservadas.
- 6 comparações de configurações distintas aprovadas.
- 1 cenário explicitamente não suportado: avaliação de target/optimize no Zig.
- Nenhuma avaliação executada reprovada. Exit global 2 significa avaliação
  incompleta por essa capacidade não suportada, conforme a política documentada.

## Ambiente e evidência

Linux x86_64; execução sequencial, build de desenvolvimento do avaliador.
Versões observadas:

- Go: go version go1.26.5 linux/amd64
- Node: v20.20.2
- Python: Python 3.12.3
- TypeScript fixado: 5.9.3
- Zig: 0.15.2
- libclang: Ubuntu clang version 18.1.3 (1ubuntu1)

Relatórios completos desta rodada (artefatos locais ignorados por Git):

- [Relatório legível](runs/run-1790469406538662702-2582310-0/relatorio.md)
- [JSON com todas as amostras](runs/run-1790469406538662702-2582310-0/relatorio.json)

O JSON contém hashes das ferramentas/fixtures declaradas, configurações, versões,
argv, códigos de retorno originais, metadados, contagens, medições de cada etapa,
medianas e MAD, e caminhos para stdout/stderr. A primeira amostra não é cache frio.
Nenhum tempo foi apresentado como parsing puro ou fase interna isolada do adaptador.
Os tempos não estabelecem ranking entre linguagens nem promessa geral de latência.

A cadeia em cada linguagem foi validada contra as duas arestas declaradas e
profundidade 2 calculada pelo núcleo. Isso não equipara imports, includes,
referências semânticas e candidatos sintáticos. Casos parciais foram aprovados
somente com código e metadados/diagnósticos explicitamente esperados.

Limites e reprodução estão no README. Próximos passos: cenários maiores por
semântica, ambientes fixados, medições com mais amostras e implementação de
encerramento de árvores por Job Objects antes de habilitar Windows. Sem mudanças
nos adaptadores para contornar resultados de avaliação.
