# Resolução de raízes de crates Rust no SCIP (L3)
Hash do Código: 36564b59

Para uma identidade de raiz ambígua do rust-analyzer, um uso literal de
`crate` só pode apontar à única raiz candidata que alcança o arquivo de
origem pela cadeia de módulos externos. Cada vínculo nessa cadeia combina
uma declaração sintática `mod` e um destino em arquivo distinto comprovado
pela identidade SCIP do módulo. Uma raiz candidata é alcançável a partir
de si mesma. Raízes concorrentes ou ausência de prova deixam o uso ambíguo.
Uma declaração `mod` com destinos concorrentes invalida a prova de
propriedade de seus candidatos.

## Verificação

- Uma cadeia indexada `lib.rs` → módulo → regra recupera `crate` na biblioteca
  quando binário e testes compartilham a mesma identidade SCIP de raiz.
- Um módulo compartilhado por duas raízes candidatas não escolhe destino.
- Um uso de raiz cujo token não é `crate` permanece ambíguo.
