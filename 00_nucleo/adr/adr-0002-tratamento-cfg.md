# ⚖️ ADR-0002: Tratamento de `#[cfg(...)]` no MVP

**Status**: `ACEITO`
**Data**: 2026-05-20
**Projecto**: `crystalline-dsm`
**Passo do roadmap relacionado**: 1.2 — Travessia de módulos por crate

---

## Contexto

A linguagem Rust permite compilação condicional via atributos
`#[cfg(...)]` e `#[cfg_attr(...)]`. Estes atributos podem aparecer
em cima de declarações de módulo (entre outros itens), e fazem com
que o módulo seja incluído ou excluído do build dependendo de
condições como sistema operacional, arquitectura, features
declaradas no `Cargo.toml`, ou contexto de compilação (`test`).

Exemplo:

```rust
#[cfg(target_os = "linux")]
mod epoll_backend;

#[cfg(feature = "json")]
mod json_export;
```

Para uma ferramenta de análise estática que produz um grafo de
dependências (e a DSM dele), isso introduz uma ambiguidade: o
mesmo código fonte pode produzir grafos diferentes dependendo das
condições activas no momento da compilação. Não existe "o" grafo
do projecto; existem vários grafos possíveis.

O `crystalline-dsm` precisa decidir como tratar isso. A decisão
afecta o desenho do parser de travessia de módulos (Passo 1.2) e
o contrato de previsibilidade da ferramenta.

---

## Alternativas consideradas

### Alternativa A — Ignorar todos os `#[cfg]`, mapear tudo

O parser lê toda declaração `mod foo;` encontrada, sem avaliar os
atributos `#[cfg]` em cima dela. O grafo resultante é a união de
todos os grafos possíveis sob qualquer configuração.

**Prós:**
- Implementação simples (nenhum parser de expressões cfg).
- Comportamento determinístico (mesmo input sempre produz mesmo
  output).
- Útil para entender arquitectura geral do projecto, incluindo
  módulos de plataformas que o desenvolvedor não está usando.

**Contras:**
- Pode mostrar dependências que nunca coexistem no mesmo binário
  compilado (ex: módulo Linux e Windows aparecem juntos).
- Para projectos com muita compilação condicional (libc, mio,
  crates cross-platform), o grafo pode ficar carregado e
  difícil de interpretar.

### Alternativa B — Avaliar `#[cfg]` baseado em features e contexto

A CLI aceita flags como `--features`, `--all-features`,
`--no-default-features`, `--target`, e o parser avalia cada
`#[cfg]` para decidir se o módulo entra no grafo.

**Prós:**
- Grafo realista, igual ao que o compilador produz para aquela
  configuração específica.
- Útil para análise de produção (entender o binário real).

**Contras:**
- Implementação significativamente mais complexa.
- Requer parser de expressões cfg (`all(unix, not(target_arch =
  "wasm32"))`, etc).
- Requer defaults para `target_os`, `target_arch`, etc (o do host?
  declarado pela CLI?).
- Não-determinístico entre execuções com flags diferentes.
- Dificulta versionamento da DSM em CI (qual configuração
  versionar?).

### Alternativa C — Híbrida: ignorar por defeito, opcionalmente filtrar

Comportamento por defeito = Alternativa A. Se o utilizador passar
`--features x,y`, o parser entra em modo Alternativa B.

**Prós:**
- Default simples e determinístico.
- Capacidade adicional para casos avançados.

**Contras:**
- Implementação contém os custos da Alternativa B.
- Adia o custo mas não elimina.

---

## Decisão

**Alternativa A: ignorar todos os `#[cfg]` no MVP.**

O parser de travessia de módulos do `crystalline-dsm` lê toda
declaração `mod foo;` encontrada no código, sem consultar os
atributos `#[cfg]` em cima dela. O grafo produzido representa a
união de todas as configurações possíveis.

### Justificação

1. **Simplicidade**: O custo de Alternativa B é alto (parser de
   cfg, defaults de plataforma, configuração de CLI). Para o MVP,
   esse custo não se justifica.

2. **Caso de uso principal**: O Typst (alvo principal do
   `crystalline-dsm`) tem pouquíssima compilação condicional, e o
   que tem é maioritariamente `#[cfg(test)]`. A perda de fidelidade
   pela escolha da Alternativa A é negligível neste caso.

3. **Determinismo**: Para uma ferramenta de análise arquitectural
   que pode ser executada em CI, output determinístico é mais
   valioso que fidelidade ao binário específico.

4. **Espaço para evolução**: Se em uso real ficar evidente que
   Alternativa B é necessária, a Alternativa C pode ser adoptada
   numa versão futura sem quebrar a interface actual.

---

## Consequências

### ✅ Positivas

- Parser do Passo 1.2 não precisa de avaliação de expressões cfg.
- CLI fica simples (sem flags `--features`, `--target`).
- Output reproduzível em qualquer máquina, qualquer plataforma.
- DSM versionável em CI sem ambiguidade de configuração.

### ❌ Negativas

- Projectos com módulos plataforma-específicos vão mostrar todas
  as plataformas no grafo, mesmo que apenas uma compile na
  máquina do utilizador.
- Módulos `#[cfg(test)]` aparecem no grafo de produção. Pode
  poluir a visualização.

### ⚙️ Mitigações

- Documentar claramente esta limitação no README e na saída da
  ferramenta (rodapé do HTML: "Análise inclui todos os módulos
  declarados, independente de `#[cfg]`").
- Considerar, em versão posterior, uma flag `--exclude-test` que
  filtra apenas o caso comum `#[cfg(test)]`. Custo baixo,
  benefício alto. **Fora do MVP**, mas registrado como melhoria
  futura.
- Sem nenhum dos critérios de reavaliação (abaixo) atingido, a
  decisão actual permanece. Não revisar por especulação ou
  preferência estética.

---

## Critérios de reavaliação

Esta ADR deve ser reaberta (via ADR sucessora) se **qualquer** dos
seguintes critérios for atingido:

1. **Adoção em projecto Rust embarcado**: o `crystalline-dsm` for
   adoptado por algum projecto `no_std`, `cortex-m`, `esp32` ou
   similar onde compilação condicional para hardware é central.

2. **Adoção em projecto cross-platform significativo**: o
   `crystalline-dsm` for adoptado por projecto Rust com mais de 5%
   do código sob `#[cfg(target_os = "...")]` ou
   `#[cfg(target_arch = "...")]`. Critério prático: contar linhas
   ou módulos afectados num levantamento manual.

3. **Pedido explícito de utilizador real**: pelo menos um
   utilizador (interno ou externo) reportar caso de uso concreto
   onde a Alternativa A produziu grafo enganoso o suficiente para
   bloquear o trabalho dele.

Sem nenhum destes critérios, a decisão não deve ser revisitada,
independentemente de dúvida pessoal ou pressão estética por
"completude".

---

## Verificação a fazer (não bloqueante)

O Passo 0.1 (Estudo Prévio) deixou em aberto: como o
`cargo-modules` trata `#[cfg]`? Verificar lendo o código fonte
quando conveniente. Se `cargo-modules` adopta a Alternativa B, vale
estudar a abordagem para futura referência. Esta verificação **não
bloqueia** a implementação do Passo 1.2 sob esta ADR.

---

## Referências

- The Cargo Book — Features:
  https://doc.rust-lang.org/cargo/reference/features.html
- The Rust Reference — Conditional compilation:
  https://doc.rust-lang.org/reference/conditional-compilation.html
- ADR-0001 — Criação da ferramenta.
- Estudo Prévio (Passo 0.1) — Seção 2 sobre tratamento de `#[cfg]`.
