# Laudo de Execução — Prompt 0051 (verificar a cegueira cross-crate do V3)

**Camada**: L5 (laudo)
**Data**: 2026-06-06
**Prompt executado**: `00_nucleo/0051-verificar_v3_cross_crate.md` (no clone do `tekt-linter`)
**Natureza**: verificação, não conserto. Confirmar/refutar a premissa da opção 3.
**Estado**: `EXECUTADO` — **premissa CONFIRMADA**. Nada do linter de produção mudou.

---

## Veredito (em uma linha)

**A afirmação de Claude estava CORRETA.** O `resolve_layer` é cego a cross-crate
(`use lente_*::` → `Layer::Unknown`), e o **V3 ignora explicitamente `Unknown`**,
logo **não dispara** em violação de direção entre crates. Quem guarda a fronteira
externa do L1 é o **V14**, contornado pela whitelist do 0050. A **opção 3 se
justifica**. O falso positivo do `Kind` (bónus) também foi confirmado.

---

## 1. Evidência de fonte (o mecanismo)

### `resolve_layer` — `03_infra/rs_parser.rs:288-302`

```rust
fn resolve_layer(path: &str, config: &CrystallineConfig) -> Layer {
    let path = path.trim_start_matches('{').trim();

    if !path.starts_with("crate::") && !path.starts_with("super::") {
        return Layer::Unknown;          // <-- todo use lente_*:: cai aqui
    }
    let segments: Vec<&str> = path.splitn(4, "::").collect();
    if let Some(module_name) = segments.get(1) {
        config.layer_for_module(module_name)   // só resolve o 2º segmento de crate::
    } else {
        Layer::Unknown
    }
}
```

Confirmado: qualquer path que não comece com `crate::`/`super::` — inclusive
crates first-party de outra camada (`lente_wiring::…`) — vira `Layer::Unknown`.
Os próprios testes do parser (`rs_parser.rs:940-952`) confirmam: `reqwest::Client`
e `std::fs::read` → `Unknown`.

### V3 (`ForbiddenImport`) — `01_core/rules/forbidden_import.rs:33-44`

```rust
fn is_forbidden(source: &Layer, target: &Layer) -> bool {
    if *target == Layer::Unknown {
        return false;                   // <-- O BURACO: Unknown nunca é violação
    }
    match source {
        Layer::L1 => matches!(target, Layer::L2 | Layer::L3 | Layer::L4 | Layer::Lab),
        Layer::L2 => matches!(target, Layer::L3 | Layer::L4 | Layer::Lab),
        Layer::L3 => matches!(target, Layer::L2 | Layer::L4 | Layer::Lab),
        Layer::L4 => matches!(target, Layer::Lab),
        Layer::L0 | Layer::Lab | Layer::Unknown => false,
    }
}
```

Este é o ponto central. O V3 age sobre a **camada resolvida** do import; como o
cross-crate sempre resolve para `Unknown`, o `early return false` o torna **cego
por construção** a violações de direção entre crates. O próprio doc-comment já o
diz: *"Layer::Unknown is never a violation (external crates)."*

### V14 (`ExternalTypeInContract`) — `01_core/rules/external_type_in_contract.rs:21-29`

```rust
pub fn check<'a, T: HasImports<'a>>(file: &T, allowed: &L1AllowedExternal) -> Vec<Violation<'a>> {
    if *file.layer() != Layer::L1 {     // <-- só guarda L1
        return vec![];
    }
    file.imports().iter()
        .filter(|import| import.target_layer == Layer::Unknown)   // <-- só Unknown
        .filter(|import| !allowed.is_allowed(package_name(import.path)))
        ...
}
```

Confirma o corolário: o que pega import cross-crate (`Unknown`) é o V14 — **mas só
em L1**. Em L3/L4 não há nenhuma rede: o V3 está cego e o V14 nem se aplica.

---

## 2. Teste empírico com controle (a prova)

Fixture Cristalina descartável (em `/tmp`, já removida), camada por diretório
via `[layers]`. Dois casos lado a lado + saída real do `crystalline-lint`:

| Caso | Arquivo | Import | V3 esperado | V3 observado |
|---|---|---|---|---|
| **cross-crate proibido** | `03_infra/cross.rs` (L3) | `use lente_wiring::Algo;` (L4) | silencioso | **silencioso** ✓ |
| **controle (resolvível)** | `01_core/control.rs` (L1) | `use crate::shell::Algo;` (L2) | dispara | **dispara** ✓ |

Saída literal do linter:

```
error: Inversão de gravidade: L1 não pode importar de L2 ('crate::shell::Algo') [V3]
   --> 01_core/control.rs:1
```
(O `03_infra/cross.rs` produziu apenas um V1 de header ausente — **nenhum V3**.)

**O controle disparou** → o V3 estava ativo e resolvendo. O teste é válido.
O contraste (mesmo pipeline, mesma rodada) isola a causa: não é "V3 quebrado/
desligado", é "V3 cego a cross-crate". Premissa confirmada.

---

## 3. Bónus — falso positivo do `Kind` (confirmado)

L1 com `use EnumLocal::*;` (glob de enum local). `package_name` pega o 1º segmento
(`EnumLocal`), o path não começa com `crate::` → `target_layer == Unknown` → o V14
o trata como pacote externo:

```
error: Dependência externa não autorizada em L1: 'EnumLocal' não está em
       [l1_allowed_external]. ... [V14]
```

Mecanismo: `external_type_in_contract.rs:48-59` (`package_name`) + `resolve_layer`
não distinguir glob de tipo local de import de crate externa. Ambos viram `Unknown`.

---

## 4. Consequência para a opção 3

A premissa se sustenta: corrigir o linter (opção 3) é o caminho que fecha o buraco
estrutural. O escopo permanece o que se inferiu — o V3 não cobre cross-crate por
nenhum outro mecanismo. Sub-itens candidatos da opção 3:

1. Ensinar o `resolve_layer` a mapear crates first-party (`lente_*`) para camadas
   (algum registo crate→layer), para o V3 voltar a enxergar a direção entre crates.
2. Tratar o falso positivo do `Kind` (distinguir glob de tipo local de import de
   crate externa antes de marcar `Unknown`).

Decisão de executar a opção 3 é passo seguinte e à parte — este laudo só confirma
a premissa.

---

## 5. Garantias do protocolo

- **Nada do linter de produção mudou** — `git status` no `tekt-linter` mostra apenas
  o prompt 0051 (untracked, pré-existente). Sem edição de `resolve_layer`, V3, V14.
- Fixture **descartável**, criada e removida em `/tmp`. Não ficou no repo.
- Conclusão tirada **com o controle disparando** — sem ele, "V3 não disparou" não
  provaria nada.
