# Prompt 0079 — mapa declarativo de correspondência no `--comparar`

**Estado:** PROPOSTO — requer confirmação humana antes da materialização.

**Camadas afetadas:** L1 (`lente_comparacao`), L2 (CLI e saída), L3
(leitura/validação sintática do arquivo) e L4 (orquestração).

**Baseline congelado:** `98d8f9e7a3882beb3fd0a294a95df9332058f76c`.

## Intenção

Permitir que o `--comparar` reconheça mecanicamente movimentos, divisões e
consolidações que não podem ser pareados pela chave K4 ou pelo path físico,
sem renomear o sistema analisado e sem transformar correspondência estrutural
em prova de paridade funcional.

O mapa também deve servir como inventário documental verificável: pode declarar
que um conjunto do lado `depois` cobre um conjunto do lado `antes`, registrar o
estado alegado dessa cobertura e apontar para evidências externas. A lente
publica a alegação e valida sua consistência mecânica; não julga o conteúdo das
evidências.

## Interface pública

O modo `--comparar` recebe uma opção aditiva:

```text
--mapa-correspondencia <arquivo.toml>
```

A ausência da opção preserva bit a bit o contrato atual. O arquivo usa TOML 1.0
e começa com:

```toml
tekt_comparison_map = 1
```

### Regras de prefixo

Movimentos sistemáticos são declarados por prefixos exatos, com fronteira de
segmento `::`:

```toml
[[prefix]]
antes = "typst_syntax::ast"
depois = "typst_core::entities::ast"
```

Uma regra pareia somente quando o sufixo restante é igual dos dois lados. Ela
não usa similaridade textual. Prefixos sobrepostos que produzam destinos
diferentes tornam o item `Unknown` e geram diagnóstico; ordem no arquivo não
resolve conflito.

### Correspondências explícitas

Relações que não são simples movimento usam conjuntos de paths completos:

```toml
[[correspondencia]]
id = "eval-apply-binary"
antes = ["typst_eval::ops::apply_binary"]
depois = [
  "typst_core::compiler::eval::operators::arithmetic::apply_binary",
  "typst_core::compiler::eval::operators::equality::apply_binary",
  "typst_core::compiler::eval::operators::ordering::apply_binary",
]
relacao = "divisao"
alegacao = "declarada-fechada"
evidencias = ["00_nucleo/diagnosticos/typst-p1210-rebaseline-paridade-funcional.md"]
nota = "A implementação cristalina divide o dispatcher vanilla por domínio."
```

`relacao` é enum fechado:

- `um-para-um`;
- `divisao` (`1:N`);
- `consolidacao` (`N:1`);
- `grupo` (`N:M`).

`alegacao` é enum fechado e documental:

- `declarada-fechada` — o autor afirma cobertura completa;
- `parcial` — existe cobertura conhecida, mas incompleta;
- `ausente` — falta equivalente no lado depois;
- `fora-de-escopo` — ausência aceita pelo escopo documentado;
- `unknown` — ainda não há julgamento.

Nenhum valor significa “verificado pela lente”. `declarada-fechada` deve sair
na UI/JSON com origem `mapa`, nunca como paridade funcional provada.

## Precedência e política de `Unknown`

1. Correspondência explícita válida tem precedência sobre prefixo e K4.
2. Prefixo válido tem precedência sobre o pareamento automático atual.
3. O pareamento automático atual trata apenas o resíduo.
4. Um path não pode pertencer a duas correspondências explícitas distintas.
5. Path declarado inexistente, cardinalidade incompatível com `relacao`, id
   duplicado, sobreposição conflitante ou um lado não extraído produz
   `Unknown` com testemunha; nunca é aceito silenciosamente.
6. Uma correspondência só consome os paths que existirem no censo member-only.
   Fantasmas e third-party permanecem fora e são reportados como hoje.
7. Reordenação das tabelas TOML não muda o resultado.

## Resultado observável

O JSON preserva os campos existentes e adiciona:

```text
mapa_correspondencia:
  caminho
  sha256
  regras_prefixo
  correspondencias
  diagnosticos

itens:
  pareados_automaticos
  pareados_por_prefixo
  correspondencias_declaradas
  ambiguos
  unknown
  sem_par_antes
  sem_par_depois
```

Cada correspondência declarada repete `id`, conjuntos de paths, `relacao`,
`alegacao`, `evidencias`, `nota` e um estado mecânico:

- `aplicada`;
- `unknown`.

A saída textual apresenta contagens separadas e uma seção de diagnósticos. Não
soma `declarada-fechada` com pareamento automático numa nota única.

## Separação de responsabilidades

- **L3:** lê bytes, calcula SHA-256, interpreta TOML e produz DTO validado
  sintaticamente. Nenhum acesso a arquivo entra em L1.
- **L1:** valida paths/cardinalidade/conflitos contra os dois grafos e aplica as
  regras de forma pura e determinística.
- **L4:** injeta o mapa opcional no pipeline `comparar`.
- **L2:** expõe a flag e serializa resultados/diagnósticos.

## Contrato discriminatório antes da implementação

Casos positivos:

1. movimento por prefixo pareia suffixos iguais;
2. correspondência `1:N` remove todos os membros válidos do resíduo;
3. ausência do mapa reproduz exatamente o resultado anterior;
4. a alegação e as evidências reaparecem sem alteração semântica na saída;
5. duas execuções e duas ordenações do mesmo mapa geram o mesmo resultado.

Mutações que o contrato deve rejeitar:

1. usar `starts_with` sem fronteira `::`;
2. deixar K4 sobrescrever correspondência explícita;
3. aceitar path inexistente como `aplicada`;
4. resolver conflito pela última regra do arquivo;
5. promover `declarada-fechada` a veredito funcional;
6. aceitar `relacao = "um-para-um"` com cardinalidade `1:N`;
7. permitir que o mesmo path seja consumido por duas relações;
8. alterar o resultado sem mapa.

Caso opaco obrigatório: regra que referencia crate não extraído por falha deve
resultar em `Unknown`, não em `ausente`, `aplicada` ou sucesso.

O gate discriminatório exige rejeição de 100% das mutações válidas antes do
selo do contrato.

## Gates finais

1. testes unitários puros do parser DTO→domínio e do pareamento;
2. testes A/B sem mapa e com mapa mínimo;
3. mutações negativas acima, score `1.0`;
4. `cargo test --workspace`;
5. linter cristalino do repositório;
6. rodada real vanilla × cristalino, com comparação das contagens antes/depois;
7. recibo contendo hashes do baseline, prompt, mapa, binário e saídas.

## Limites do veredito

O mapa documenta e mecaniza correspondências. Ele pode reduzir falsos
“sem-par” e tornar o resíduo acionável, mas não demonstra equivalência de
linguagem, output ou comportamento. O veredito funcional continua pertencendo
às sondas vanilla × cristalino referenciadas em `evidencias`.

