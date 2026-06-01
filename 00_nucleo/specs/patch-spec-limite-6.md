# Patch para `00_nucleo/forma-organizada.md` — Limite 6

**Propósito**: registrar o Limite 6 — colisões em código gerado por macro que
a resolução automática (ADR-0004/0005) não alcança.

**Origem**: ADR-0005, Ajuste 5. Medição (`lab/medicao-colisoes/remedicao/relatorio.md`)
mostrou que 10 das 384 colisões (2,6%) não são resolvíveis pela cascata.

**Como aplicar**: você lê e edita a spec do repositório (que tem os Limites
1–5 e a nota de evolução, que eu não tenho aqui), adicionando o Limite 6 na
mesma estrutura dos atuais, preservando tudo que já existe.

---

## Limite 6 — Colisões de path em código gerado por macro

Texto sugerido (ajuste o estilo para casar com os Limites 1–5 existentes):

> **Limite 6 — Colisões em código gerado por macro não são resolvíveis
> automaticamente.**
>
> A forma organizada usa `id` para distinguir nós com mesmo `path` (Mudança
> 3), e o mecanismo de resolução de colisões (ADR-0004, validado pelo
> ADR-0005) decide a esmagadora maioria das colisões pela vizinhança no grafo.
> Resta uma classe que não é resolvível: colisões onde o nó colidente é um
> **módulo gerado por macro** (não um tipo), cujas cópias compartilham a
> aresta `Owns` do módulo-pai e não têm `impl <Trait> for <tipo>` literal no
> código-fonte.
>
> Exemplo medido: `typst_macros::util::kw::<nome>` — vários nós com o mesmo
> path gerados por uma macro, no mesmo módulo. A resolução por vizinhança não
> dispara (há aresta compartilhada com o módulo-pai), e a resolução por
> código-fonte não dispara (não existe o `impl` literal — é macro-gerado).
>
> Consequência: para esses casos, a lente reporta a colisão como não
> resolvida, com diagnóstico claro, em vez de inventar uma distinção. O
> usuário vê que aqueles nós são ambíguos e que a ambiguidade vem de
> geração por macro.
>
> Magnitude observada: 10 de 384 colisões (2,6%) nos 17 crates do typst,
> todas concentradas em `typst_macros`. Crates sem macros geradoras de nomes
> colidentes não têm esse limite.

---

## Nota de evolução (opcional, acrescentar à nota existente)

Se a spec já tem a Nota de Evolução sobre subtipos de `uses`, vale acrescentar
uma linha conectando:

> O Limite 6 (colisões em código gerado por macro) compartilha raiz com a
> família de ambiguidades que a identidade-por-nó resolveu: são casos onde a
> fonte não expõe a distinção que existiria no código semântico. A resolução
> para os outros casos veio do fork (identidade-por-nó). Para o Limite 6, a
> evolução possível seria o fork identificar nós originados por expansão de
> macro — caminho futuro, não trabalho do primeiro passo. O sintoma que
> dispararia a reavaliação: crates onde colisões de macro sejam frequentes o
> bastante para inviabilizar o uso da lente (não foi o caso no typst, onde
> são 2,6% concentrados em um crate).

---

## O que NÃO mudar

- Os Limites 1–5 existentes.
- A Mudança 3 (identidade por `id`) já aplicada.
- Os Critérios de Verificação.
- A estrutura geral da spec.
