# Resolução de FFI no índice SCIP (L3)
Hash do Código: 4e6645b2

Opcionalmente recebe um artefato ELF final do mesmo build das fontes
indexadas. Usa GNU `nm` para obter símbolos de código globais definidos
e `llvm-dwarfdump` para confirmar o subprograma no mesmo endereço.
O artefato é lido como dado. Os arquivos C/C++ precisam estar no índice
e na raiz selecionada.

Analisa a sintaxe Rust para reconhecer funções em blocos de ABI C
ou C-unwind e extrair o nome de link, inclusive `#[link_name]` literal.
O identificador Rust precisa coincidir com uma declaração SCIP pela
faixa exata. O nome e o endereço do símbolo de link exigem uma definição
DWARF inequívoca, com nome, arquivo e linha. A linha da fonte precisa ter
um único identificador correspondente ao nome nativo. Isso permite abrir
a implementação quando o índice Clang contém o arquivo mas omite o símbolo.

Uma chamada direta exige que a posição SCIP de um uso do símbolo FFI
seja o identificador final do callee em uma expressão de chamada Rust.
Registra a função Rust envolvente quando identificável. Referências de
valor, chamadas por ponteiro e ocorrências sem confirmação continuam
disponíveis como usos indexados, sem rótulo de chamada direta.
Envia ao domínio a posição da declaração da função envolvente e o
identificador do callee. Usa os limites de closures e blocos async
para evitar atribuir chamadas ao corpo de uma função que apenas
constrói esses valores.
Quando faltam dados, o vínculo original permanece restrito ao SCIP.
Os locais de chamada diretos de Rust e das unidades verificadas pelo
adaptador Clang alimentam o mesmo índice de predecessores FFI.
As posições dos corpos nativos permitem seguir chamadas diretas
depois da implementação de entrada.
As pontes ctypes, JNI, P/Invoke, Deno e Dart usam o mesmo vínculo
após validação da biblioteca e do símbolo exportado. O candidato cgo
exige wrapper de cgo e definição global no mesmo ELF. Zig exige
a definição global de ABI C no ELF e confirmação pela mesma posição DWARF.
Node-API exige o export do inicializador de addon e um callback
identificado por código, endereço e posição DWARF, inclusive se local.

Essa projeção cobre implementações C e C++ exportadas com ABI C e
declarações Rust presentes na fonte. Ficam fora do recorte a resolução
em tempo de execução, atributos de link condicionais, declarações
sintetizadas por macro e símbolos nativos sem localização inequívoca.

## Verificação

```
Dado extern Rust com link_name Quando o ELF confirma a definição C++ indexada Então ligar o uso à implementação
Dado símbolo ambíguo, indefinido ou sem linha Quando resolver Então conservar somente o SCIP
Dado um objeto relocável Quando informar como artefato Então rejeitar o arquivo
Dado o uso de uma função estrangeira Quando sua posição for o callee de uma chamada Então registrar o local e o nome da função envolvente
```
