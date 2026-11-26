# Chamadas nativas para a Lente FFI (L3)
Hash do Código: e1618755

Com um vínculo FFI confirmado, procura compile_commands.json na raiz
ou em build. Usa apenas unidades C e C++ que estão nas fontes indexadas
e que possuem uma configuração inequívoca. Interpreta arguments ou
command como argumentos literais, sem executar o comando do banco.
Invoca Clang em modo de análise sintática, com opções de compilação
admitidas e sem arquivos de resposta, plugins ou saída de compilação.
Uma unidade sem configuração suportada ou que falha na análise não
acrescenta chamadas.

Extrai chamadas diretas por nome de funções e métodos estáticos, com
identificador visível na unidade. Converte o offset UTF-8 da AST nas
coordenadas de LocalFonte e confere a grafia do identificador na fonte.
Registra a posição da função envolvente. Ignora expansões de macro,
chamadas por ponteiro, despacho de instância, corpos de lambdas,
templates de função e posições em headers incluídos. O destino
normalmente vem do SCIP.
Também registra a posição das definições nativas com corpo no arquivo
principal, inclusive funções sem chamadas, para resolver os destinos
depois da fronteira FFI.
Para uma função externa, conserva também o nome de link resolvido
pelo Clang como evidência complementar para a implementação FFI
confirmada. Uma declaração estática não pode produzir esse vínculo.

## Verificação

```
Dada AST de chamada direta Então registrar o callee e a declaração envolvente.
Dado uso como valor ou chamada indireta Então não registrar uma chamada direta.
Dado argumento para plugin ou arquivo de resposta Então recusar a unidade.
Dada cada opção de controle de compilação ou saída Então removê-la com seu argumento, quando exigido.
Dada cada opção admitida Então preservá-la e consumir seu argumento apenas quando exigido.
Dada uma opção sem o argumento obrigatório Então recusar a unidade.
```

A admissão dos argumentos de compilação também é reutilizada pelo adaptador
de Explorar chamadas nativas. As duas projeções usam contratos separados.

## Histórico de Revisões

| Data | Motivo | Arquivos afetados |
|------|--------|-------------------|
| 2026-11-26 | Cobertura parametrizada das alternativas de admissão de opções de compilação. | `03_infra/src/scip_ffi/chamadas_nativas.rs` |
