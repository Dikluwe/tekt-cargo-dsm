//! Arena (prompt 0077) — discriminância das chaves de identidade de item no par
//! typst. **Mede, não decide.** Consome a lente como biblioteca; censo idêntico
//! ao da produção (montar_grafo_workspace → filtrar_stdlib → filtrar_nao_membros).
//!
//! Uso: `medicao-chave-item <raiz_antes> <raiz_depois>`
//!
//! Saída: relatório markdown em stdout (redirecionar para relatorio.md).

use std::collections::{BTreeMap, BTreeSet, HashMap};

use lente_core::entities::grafo::{Grafo, Kind, No, Relation};
use lente_filtro::{filtrar_nao_membros, filtrar_stdlib};
use lente_infra::enumerar_membros;
use lente_wiring::montar_grafo_workspace;

fn kind_txt(k: Kind) -> &'static str {
    match k {
        Kind::Crate => "crate",
        Kind::Mod => "mod",
        Kind::Fn => "fn",
        Kind::Struct => "struct",
        Kind::Union => "union",
        Kind::Enum => "enum",
        Kind::Variant => "variant",
        Kind::Const => "const",
        Kind::Static => "static",
        Kind::Trait => "trait",
        Kind::Type => "type",
        Kind::Builtin => "builtin",
        Kind::Macro => "macro",
    }
}

/// Item = nó nomeável de definição; exclui `mod`/`crate` (nível módulo já medido
/// no 0076) e `builtin` (primitivos, não-itens do usuário).
fn e_item(k: Kind) -> bool {
    matches!(
        k,
        Kind::Fn
            | Kind::Struct
            | Kind::Union
            | Kind::Enum
            | Kind::Variant
            | Kind::Const
            | Kind::Static
            | Kind::Trait
            | Kind::Type
            | Kind::Macro
    )
}

fn e_tipo(k: Kind) -> bool {
    matches!(k, Kind::Struct | Kind::Enum | Kind::Union | Kind::Trait)
}

struct Lado {
    nome: String,
    grafo: Grafo,
    fantasmas: BTreeSet<String>,
    nos_pos_filtro: usize,
    fantasmas_n: usize,
    third_party_removido: usize,
}

fn medir_lado(raiz: &std::path::Path) -> Lado {
    let gw = montar_grafo_workspace(raiz).unwrap_or_else(|e| {
        eprintln!("FALHA ao montar workspace de {}: {}", raiz.display(), e);
        std::process::exit(1);
    });
    let membros = enumerar_membros(raiz).unwrap_or_default();
    let nomes: Vec<String> = membros.iter().map(|m| m.nome.clone()).collect();
    let sem_sysroot = filtrar_stdlib(&gw.grafo);
    let antes = sem_sysroot.nodes.len();
    let so_membros = filtrar_nao_membros(&sem_sysroot, &nomes);
    let third_party = antes - so_membros.nodes.len();
    let fantasmas: BTreeSet<String> = gw.fantasmas.iter().map(|f| f.path.as_str().to_string()).collect();
    let nome = raiz
        .canonicalize()
        .ok()
        .and_then(|p| p.file_name().map(|s| s.to_string_lossy().to_string()))
        .unwrap_or_else(|| "?".to_string());
    Lado {
        nome,
        nos_pos_filtro: so_membros.nodes.len(),
        fantasmas_n: gw.fantasmas.len(),
        third_party_removido: third_party,
        grafo: so_membros,
        fantasmas,
    }
}

/// Mapa child_id → nó-pai (pela aresta `Owns`: id_from=pai, id_to=filho).
fn pais(grafo: &Grafo) -> HashMap<usize, usize> {
    let mut m = HashMap::new();
    for a in &grafo.edges {
        if a.relation == Relation::Owns {
            m.insert(a.id_to, a.id_from);
        }
    }
    m
}

/// Nome qualificado por **pai-tipo** (Counter::get) quando o pai é tipo; senão
/// só o nome (qualificar por módulo reintroduziria a dependência de path do 0076).
fn nome_qualificado(no: &No, pais: &HashMap<usize, usize>, por_id: &HashMap<usize, &No>) -> String {
    if let Some(p) = pais.get(&no.id).and_then(|pid| por_id.get(pid)) {
        if e_tipo(p.kind) {
            return format!("{}::{}", p.name, no.name);
        }
    }
    no.name.clone()
}

/// A chave de um item segundo K (1..=4). `None` = item fora do censo daquela
/// chave (K2 exclui folhas de impl-de-trait).
fn chave(no: &No, qual: &str, k: u8) -> Option<String> {
    let kd = kind_txt(no.kind);
    match k {
        1 => Some(format!("{kd}\u{1}{}", no.name)),
        2 => {
            if no.trait_.is_some() || no.trait_ref.is_some() {
                None
            } else {
                Some(format!("{kd}\u{1}{}", no.name))
            }
        }
        3 => Some(format!("{kd}\u{1}{qual}")),
        4 => Some(format!("{kd}\u{1}{}\u{1}{qual}", no.trait_.as_deref().unwrap_or(""))),
        _ => None,
    }
}

struct ResultadoChave {
    censo_antes: usize,
    censo_depois: usize,
    chaves_antes: usize,
    chaves_depois: usize,
    pareaveis: usize,
    ambiguas_chaves: usize,
    ambiguas_itens: usize,
    sem_par_antes: usize,
    sem_par_depois: usize,
    top: Vec<(String, usize, usize, Vec<String>, Vec<String>)>, // chave, n_a, n_b, amostra_a, amostra_b
}

fn medir_chave(
    itens_a: &[&No],
    itens_b: &[&No],
    qa: &HashMap<usize, String>,
    qb: &HashMap<usize, String>,
    k: u8,
) -> ResultadoChave {
    let monta = |itens: &[&No], q: &HashMap<usize, String>| -> BTreeMap<String, Vec<String>> {
        let mut m: BTreeMap<String, Vec<String>> = BTreeMap::new();
        for no in itens {
            let qual = q.get(&no.id).cloned().unwrap_or_else(|| no.name.clone());
            if let Some(ch) = chave(no, &qual, k) {
                m.entry(ch).or_default().push(no.path.as_str().to_string());
            }
        }
        for v in m.values_mut() {
            v.sort();
        }
        m
    };
    let ma = monta(itens_a, qa);
    let mb = monta(itens_b, qb);
    let censo_antes: usize = ma.values().map(|v| v.len()).sum();
    let censo_depois: usize = mb.values().map(|v| v.len()).sum();

    let mut pareaveis = 0;
    let mut ambiguas_chaves = 0;
    let mut ambiguas_itens = 0;
    let mut sem_par_antes = 0;
    let chaves: BTreeSet<&String> = ma.keys().chain(mb.keys()).collect();
    for ch in &chaves {
        let na = ma.get(*ch).map(|v| v.len()).unwrap_or(0);
        let nb = mb.get(*ch).map(|v| v.len()).unwrap_or(0);
        if na > 0 && nb > 0 {
            if na == 1 && nb == 1 {
                pareaveis += 1;
            } else {
                ambiguas_chaves += 1;
                ambiguas_itens += na + nb;
            }
        } else if na > 0 {
            sem_par_antes += 1;
        }
    }
    let sem_par_depois = mb.keys().filter(|ch| !ma.contains_key(*ch)).count();

    // top-10 colisões: chaves com mais itens no total (presentes nos dois lados).
    let mut comuns: Vec<(String, usize, usize)> = chaves
        .iter()
        .filter_map(|ch| {
            let na = ma.get(*ch).map(|v| v.len()).unwrap_or(0);
            let nb = mb.get(*ch).map(|v| v.len()).unwrap_or(0);
            if na > 0 && nb > 0 {
                Some(((*ch).clone(), na, nb))
            } else {
                None
            }
        })
        .collect();
    comuns.sort_by(|a, b| (b.1 + b.2).cmp(&(a.1 + a.2)).then(a.0.cmp(&b.0)));
    let top: Vec<_> = comuns
        .into_iter()
        .take(10)
        .map(|(ch, na, nb)| {
            let amostra_a: Vec<String> = ma.get(&ch).map(|v| v.iter().take(3).cloned().collect()).unwrap_or_default();
            let amostra_b: Vec<String> = mb.get(&ch).map(|v| v.iter().take(3).cloned().collect()).unwrap_or_default();
            (ch.replace('\u{1}', "|"), na, nb, amostra_a, amostra_b)
        })
        .collect();

    ResultadoChave {
        censo_antes,
        censo_depois,
        chaves_antes: ma.len(),
        chaves_depois: mb.len(),
        pareaveis,
        ambiguas_chaves,
        ambiguas_itens,
        sem_par_antes,
        sem_par_depois,
        top,
    }
}

/// Censo de itens de um lado: kinds de definição, exclui representantes de fantasma.
fn censo(lado: &Lado) -> Vec<&No> {
    lado.grafo
        .nodes
        .iter()
        .filter(|n| e_item(n.kind) && !lado.fantasmas.contains(n.path.as_str()))
        .collect()
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 3 {
        eprintln!("uso: {} <raiz_antes> <raiz_depois>", args[0]);
        std::process::exit(2);
    }
    let t0 = std::time::Instant::now();
    let a = medir_lado(std::path::Path::new(&args[1]));
    let b = medir_lado(std::path::Path::new(&args[2]));
    let tempo = t0.elapsed().as_secs_f64();

    println!("# Medição — discriminância das chaves de identidade de item (par typst)\n");
    println!("Arena do prompt 0077. **Números, sem conclusão.**\n");

    println!("## Portão de sanidade (deve bater com o laudo 0076)\n");
    println!("| Lado | nós pós-filtro | fantasmas | third-party removido |");
    println!("|---|---|---|---|");
    println!("| {} (antes) | {} | {} | {} |", a.nome, a.nos_pos_filtro, a.fantasmas_n, a.third_party_removido);
    println!("| {} (depois) | {} | {} | {} |", b.nome, b.nos_pos_filtro, b.fantasmas_n, b.third_party_removido);
    println!("\n0076 esperava: fantasmas 448 / 0 · third-party 434 / 40.\n");
    println!("Tempo de montagem dos dois grafos (cache morno): **{:.2} s**\n", tempo);

    // Censo de itens por lado (exclui mod/crate/builtin e representantes de fantasma).
    let itens_a = censo(&a);
    let itens_b = censo(&b);

    let excl_a = a.grafo.nodes.iter().filter(|n| e_item(n.kind) && a.fantasmas.contains(n.path.as_str())).count();
    let excl_b = b.grafo.nodes.iter().filter(|n| e_item(n.kind) && b.fantasmas.contains(n.path.as_str())).count();

    println!("## Censo de itens\n");
    println!("Itens (kinds de definição, exclui mod/crate/builtin e representantes de fantasma).\n");
    println!("- {} (antes): **{}** itens · representantes de fantasma excluídos: {}", a.nome, itens_a.len(), excl_a);
    println!("- {} (depois): **{}** itens · representantes de fantasma excluídos: {}\n", b.nome, itens_b.len(), excl_b);

    let dist = |itens: &[&No]| -> BTreeMap<&'static str, usize> {
        let mut m: BTreeMap<&'static str, usize> = BTreeMap::new();
        for n in itens {
            *m.entry(kind_txt(n.kind)).or_insert(0) += 1;
        }
        m
    };
    println!("Distribuição por kind:\n");
    println!("| kind | antes | depois |");
    println!("|---|---|---|");
    let da = dist(&itens_a);
    let db = dist(&itens_b);
    let kinds: BTreeSet<&&str> = da.keys().chain(db.keys()).collect();
    for k in kinds {
        println!("| {} | {} | {} |", k, da.get(*k).unwrap_or(&0), db.get(*k).unwrap_or(&0));
    }
    println!();

    // Qualificadores (pai-tipo) por lado.
    let qual_de = |lado: &Lado, itens: &[&No]| -> HashMap<usize, String> {
        let pais = pais(&lado.grafo);
        let por_id: HashMap<usize, &No> = lado.grafo.nodes.iter().map(|n| (n.id, n)).collect();
        itens.iter().map(|n| (n.id, nome_qualificado(n, &pais, &por_id))).collect()
    };
    let qa = qual_de(&a, &itens_a);
    let qb = qual_de(&b, &itens_b);

    // As quatro chaves.
    let nomes_k = [
        ("K1", "(kind, nome) — censo completo"),
        ("K2", "(kind, nome) sem folhas de impl-de-trait (trait_/trait_ref)"),
        ("K3", "(kind, pai-tipo::nome)"),
        ("K4", "(kind, trait_, pai-tipo::nome)"),
    ];
    let mut sintese: Vec<(u8, ResultadoChave)> = Vec::new();
    for k in 1u8..=4 {
        sintese.push((k, medir_chave(&itens_a, &itens_b, &qa, &qb, k)));
    }

    for (i, (_k, r)) in sintese.iter().enumerate() {
        let (nome, def) = nomes_k[i];
        println!("## {} — {}\n", nome, def);
        println!("- censo: antes {} · depois {}", r.censo_antes, r.censo_depois);
        println!("- chaves distintas: antes {} · depois {}", r.chaves_antes, r.chaves_depois);
        println!("- **pareáveis 1:1: {}**", r.pareaveis);
        println!("- ambíguas: {} chaves cobrindo {} itens", r.ambiguas_chaves, r.ambiguas_itens);
        println!("- sem-par: antes {} · depois {}\n", r.sem_par_antes, r.sem_par_depois);
        if !r.top.is_empty() {
            println!("Top colisões (chave · antes×depois · amostra):\n");
            for (ch, na, nb, sa, sb) in &r.top {
                println!("- `{}` — {}×{} · a: {} · d: {}", ch, na, nb, sa.join(", "), sb.join(", "));
            }
            println!();
        }
    }

    println!("## Síntese K1–K4\n");
    println!("| Chave | censo a/d | pareáveis 1:1 | ambíguas (chaves/itens) | sem-par a/d |");
    println!("|---|---|---|---|---|");
    for (i, (_k, r)) in sintese.iter().enumerate() {
        println!(
            "| {} | {}/{} | **{}** | {}/{} | {}/{} |",
            nomes_k[i].0, r.censo_antes, r.censo_depois, r.pareaveis, r.ambiguas_chaves, r.ambiguas_itens, r.sem_par_antes, r.sem_par_depois
        );
    }
    println!("\n_Sem conclusão — a escolha da chave fica com o autor._");
}
