# Admissão de índices SCIP (L3)
Hash do Código: 28aaeafa

Recebe um ou mais índices binários SCIP do mesmo snapshot e a raiz local
explícita do código. Usa a biblioteca SCIP oficial. Lê somente documentos
relativos e arquivos regulares dentro da raiz canônica. Confere o texto
embutido quando presente. Quando o indexador não inclui texto, a fonte
local deve pertencer ao mesmo snapshot fornecido ao indexador.

Normaliza posições de UTF-8, UTF-16 e UTF-32 para bytes dentro da linha.
Para rust-analyzer, scip-clang, scip-go e scip-zig, admite UTF-8 quando o campo
de codificação está ausente. Usa identidade SCIP completa e limita
símbolos locais ao documento. Junta ocorrências idênticas e usa o papel
`Definition` para reconhecer destinos. Registra as ocorrências sem
declaração no índice. Arestas exigem identidade comum declarada pelo índice.
Para scip-python, scip-java, scip-typescript, scip-dotnet e scip-dart com
codificação ausente, normaliza os offsets UTF-16. Uma codificação explícita
suportada sempre prevalece. Rejeita uma codificação desconhecida e rejeita
a ausência de codificação quando não há padrão para o indexador.

## Verificação

```
Dado um índice com posição após um caractere multibyte Então o recorte marca o símbolo exato
Dado cada indexador com padrão documentado Então o documento sem codificação explícita resolve a mesma posição em bytes
Dado um documento com codificação explícita suportada Então os offsets seguem essa codificação mesmo quando o padrão do indexador difere
Dado um caminho absoluto ou fora da raiz Então rejeitar a entrada
Dado um símbolo local repetido em dois arquivos Então manter as identidades de cada documento
```

Para a exploração Go conserva à parte as ocorrências por símbolo e o índice de origem. Fornece evidência por uso, inclusive sem declaração no conjunto carregado.

## Prompts vinculados à fonte

Depois de reunidas as fontes para a DSM por arquivo, procurar exatamente uma
declaração `@prompt` no bloco inicial de comentários de cada fonte. Ler o
Markdown apontado relativamente à raiz do projeto, em `00_nucleo/prompts/`
ou `00_nucleo/prompt/`. Tanto o caminho declarado quanto o resolvido devem
pertencer a essas pastas na raiz canônica. Referências ausentes, ambíguas,
inválidas ou ilegíveis deixam o arquivo sem prompt e mantêm a geração da DSM.
O adaptador TextFold reutiliza a resolução confinada desses caminhos.

## Histórico de Revisões

| Data | Motivo | Arquivos afetados |
|------|--------|-------------------|
| 2026-11-26 | Cobertura parametrizada das codificações padrão e explícitas na admissão de documentos. | `03_infra/src/scip.rs` |
