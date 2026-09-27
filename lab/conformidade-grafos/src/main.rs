//! Crystalline Lineage
//! @prompt lab/conformidade-grafos/CONTRATO.md
//! @layer Lab
//! @updated 2026-09-26

use std::collections::{BTreeMap, BTreeSet};
use std::ffi::OsString;
use std::path::PathBuf;
use std::process::ExitCode;

use lente_core::domain::raio::calcular_raio;
use lente_core::entities::grafo::{Grafo, Path};
use serde_json::{Value, json};

const LIMITE: &str = "Limite: aprovação estrutural não comprova completude semântica do extrator nem equivalência comportamental.";
const USO: &str = "Uso: conformidade-grafos [--analitico] [--mapa mapa.json] [--] grafo.json [outros.json ...]\n\
    --mapa implica --analitico e exige exatamente dois grafos; mapa: [[id_esquerda,id_direita], ...].\n\
    Sem flags: valida somente a forma crua; paths repetidos são permitidos.\n\
    Saídas: 0 aprovado, 1 falha de verificação, 2 uso inválido.";

#[derive(Debug)]
struct Opcoes {
    arquivos: Vec<PathBuf>,
    analitico: bool,
    mapa: Option<PathBuf>,
}

fn argumentos(args: impl IntoIterator<Item = OsString>) -> Result<Option<Opcoes>, String> {
    let mut op = Opcoes {
        arquivos: vec![],
        analitico: false,
        mapa: None,
    };
    let mut args = args.into_iter();
    let mut literais = false;
    while let Some(arg) = args.next() {
        if literais {
            op.arquivos.push(arg.into());
            continue;
        }
        match arg.to_str() {
            Some("--") => literais = true,
            Some("--help" | "-h") => return Ok(None),
            Some("--analitico") => op.analitico = true,
            Some("--mapa") => {
                if op.mapa.is_some() {
                    return Err("--mapa repetido".into());
                }
                op.mapa = Some(args.next().ok_or("falta o arquivo de --mapa")?.into());
                op.analitico = true;
            }
            Some(s) if s.starts_with('-') => return Err(format!("opção desconhecida: {s}")),
            _ => op.arquivos.push(arg.into()),
        }
    }
    if op.arquivos.is_empty() {
        return Err("informe ao menos um arquivo JSON".into());
    }
    if op.mapa.is_some() && op.arquivos.len() != 2 {
        return Err("--mapa exige exatamente dois grafos".into());
    }
    Ok(Some(op))
}

fn paths_repetidos(g: &Grafo) -> BTreeSet<&Path> {
    let mut vistos = BTreeSet::new();
    g.nodes
        .iter()
        .filter_map(|n| (!vistos.insert(&n.path)).then_some(&n.path))
        .collect()
}

fn exigir_paths_unicos(g: &Grafo) -> Result<(), String> {
    let repetidos = paths_repetidos(g);
    if repetidos.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "análise requer paths únicos; colisões legítimas na forma crua: {repetidos:?}"
        ))
    }
}

// É correspondência fornecida pelo chamador, não detecção de isomorfismo.
fn correspondencia(
    a: &Grafo,
    b: &Grafo,
    pares: &[[usize; 2]],
) -> Result<BTreeMap<usize, usize>, String> {
    let ids_a: BTreeSet<_> = a.nodes.iter().map(|n| n.id).collect();
    let ids_b: BTreeSet<_> = b.nodes.iter().map(|n| n.id).collect();
    let mut inverso = BTreeMap::new();
    let mut usados_a = BTreeSet::new();
    for &[esquerda, direita] in pares {
        if !ids_a.contains(&esquerda) || !ids_b.contains(&direita) {
            return Err(format!(
                "mapa referencia ID inexistente: {esquerda} → {direita}"
            ));
        }
        if !usados_a.insert(esquerda) || inverso.insert(direita, esquerda).is_some() {
            return Err("mapa não é bijetivo: ID repetido".into());
        }
    }
    if usados_a != ids_a || inverso.keys().copied().collect::<BTreeSet<_>>() != ids_b {
        return Err("mapa deve cobrir todos os nós dos dois grafos".into());
    }
    Ok(inverso)
}

// Normaliza resultados dos algoritmos existentes. Nenhum BFS/SCC/ranking local.
fn assinatura(g: &Grafo, ids: &BTreeMap<usize, usize>) -> Result<Value, String> {
    exigir_paths_unicos(g)?;
    let por_path: BTreeMap<_, _> = g.nodes.iter().map(|n| (&n.path, ids[&n.id])).collect();
    let mut nos = BTreeMap::new();
    for n in &g.nodes {
        let r = calcular_raio(g, &n.path).map_err(|e| e.to_string())?;
        let montante: BTreeMap<_, _> = r.montante.iter().map(|(p, d)| (por_path[p], *d)).collect();
        let jusante: BTreeMap<_, _> = r.jusante.iter().map(|(p, d)| (por_path[p], *d)).collect();
        let filhos: BTreeSet<_> = r.owns_filhos.iter().map(|p| por_path[p]).collect();
        nos.insert(
            ids[&n.id],
            json!({
                "classificacao": format!("{:?}", r.classificacao),
                "entrada": r.uses_entrada, "saida": r.uses_saida,
                "montante": montante, "jusante": jusante,
                "profundidade_montante": r.profundidade_maxima_montante(),
                "profundidade_jusante": r.profundidade_maxima_jusante(),
                "pai": r.owns_pai.as_ref().map(|p| por_path[p]), "filhos": filhos
            }),
        );
    }
    let mut arestas: Vec<_> = g
        .edges
        .iter()
        .map(|e| {
            (
                ids[&e.id_from],
                ids[&e.id_to],
                format!("{:?}", e.relation),
                format!("{:?}", e.uses_kind),
            )
        })
        .collect();
    arestas.sort(); // Preserva multiplicidade; não converter para set.
    let mut ciclos: Vec<Vec<_>> = lente_estrutura::detectar_ciclos(g)
        .into_iter()
        .map(|c| {
            let mut ids: Vec<_> = c.modulos.iter().map(|p| por_path[p]).collect();
            ids.sort();
            ids
        })
        .collect();
    ciclos.sort();
    let ranking: BTreeMap<_, _> = lente_ranking::rankear(g, g.nodes.len())
        .into_iter()
        .map(|r| {
            (
                por_path[&r.path],
                (r.impacto, format!("{:?}", r.classificacao)),
            )
        })
        .collect();
    Ok(json!({"nos": nos, "arestas": arestas, "ciclos": ciclos, "ranking": ranking}))
}

fn comparar(a: &Grafo, b: &Grafo, pares: &[[usize; 2]]) -> Result<(), String> {
    let inverso = correspondencia(a, b, pares)?;
    let identidade = a.nodes.iter().map(|n| (n.id, n.id)).collect();
    let sa = assinatura(a, &identidade)?;
    let sb = assinatura(b, &inverso)?;
    for campo in ["arestas", "nos", "ciclos", "ranking"] {
        if sa[campo] != sb[campo] {
            return Err(format!(
                "divergência em {campo} após correspondência explícita\nesquerda: {}\ndireita: {}",
                sa[campo], sb[campo]
            ));
        }
    }
    Ok(())
}

fn executar(op: Opcoes) -> (u8, String) {
    let mut relatorio = String::new();
    let mut falhou = false;
    let mut grafos = Vec::new();
    for arquivo in &op.arquivos {
        let resultado = std::fs::read_to_string(arquivo)
            .map_err(|e| format!("leitura: {e}"))
            .and_then(|s| {
                lente_infra::desserializar_grafo(&s).map_err(|e| format!("forma crua: {e}"))
            });
        match resultado {
            Err(e) => {
                falhou = true;
                relatorio.push_str(&format!("FALHA {arquivo:?}: {e}\n"));
            }
            Ok(g) => {
                relatorio.push_str(&format!(
                    "OK {arquivo:?}: forma crua aceita; {} nós, {} arestas, {} paths repetidos.\n",
                    g.nodes.len(),
                    g.edges.len(),
                    paths_repetidos(&g).len()
                ));
                if op.analitico {
                    let identidade = g.nodes.iter().map(|n| (n.id, n.id)).collect();
                    match assinatura(&g, &identidade) {
                        Ok(a) => relatorio.push_str(&format!(
                            "  OK análise (precondição de paths únicos):\n{}\n",
                            serde_json::to_string_pretty(&a).expect("Value serializável")
                        )),
                        Err(e) => {
                            falhou = true;
                            relatorio.push_str(&format!("  FALHA {e}\n"));
                        }
                    }
                } else {
                    relatorio.push_str(
                        "  Análise não solicitada; a forma resolvida não foi certificada.\n",
                    );
                }
                grafos.push(g);
            }
        }
    }
    if let Some(mapa) = op.mapa {
        let resultado = std::fs::read_to_string(&mapa)
            .map_err(|e| format!("mapa {mapa:?}: {e}"))
            .and_then(|s| {
                serde_json::from_str::<Vec<[usize; 2]>>(&s)
                    .map_err(|e| format!("mapa {mapa:?}: {e}"))
            })
            .and_then(|pares| {
                if grafos.len() != 2 {
                    return Err("comparação indisponível: um grafo falhou na validação crua".into());
                }
                comparar(&grafos[0], &grafos[1], &pares)
            });
        match resultado {
            Ok(()) => relatorio.push_str("OK correspondência: relações, contagens, profundidades, ciclos e pontuações de ranking equivalentes.\n"),
            Err(e) => { falhou = true; relatorio.push_str(&format!("FALHA comparação: {e}\n")); }
        }
    }
    relatorio.push_str(LIMITE);
    relatorio.push('\n');
    (u8::from(falhou), relatorio)
}

fn main() -> ExitCode {
    match argumentos(std::env::args_os().skip(1)) {
        Ok(None) => {
            println!("{USO}\n{LIMITE}");
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("{e}\n{USO}");
            ExitCode::from(2)
        }
        Ok(Some(op)) => {
            let (codigo, texto) = executar(op);
            print!("{texto}");
            ExitCode::from(codigo)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn arquivo(nome: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("fixtures")
            .join(nome)
    }
    fn grafo(nome: &str) -> Grafo {
        lente_infra::desserializar_grafo(&std::fs::read_to_string(arquivo(nome)).unwrap()).unwrap()
    }
    fn op(nomes: &[&str], analitico: bool) -> Opcoes {
        Opcoes {
            arquivos: nomes.iter().map(|n| arquivo(n)).collect(),
            analitico,
            mapa: None,
        }
    }

    #[test]
    fn cadeia_tem_profundidades_e_ranking_conhecidos() {
        let g = grafo("cadeia.json");
        let r = calcular_raio(&g, &Path::from("origem::c")).unwrap();
        assert_eq!(r.uses_entrada, 1);
        assert_eq!(r.montante[&Path::from("origem::b")], 1);
        assert_eq!(r.montante[&Path::from("origem::a")], 2);
        assert!(r.jusante.is_empty());
        assert_eq!(
            lente_ranking::rankear(&g, 3)
                .iter()
                .map(|r| r.impacto)
                .collect::<Vec<_>>(),
            [2, 1, 0]
        );
        assert!(lente_estrutura::detectar_ciclos(&g).is_empty());
    }

    #[test]
    fn ciclo_termina_e_nao_inclui_alvo_no_proprio_raio() {
        let g = grafo("ciclo.json");
        let r = calcular_raio(&g, &g.nodes[0].path).unwrap();
        assert_eq!(r.montante.len(), 2);
        assert_eq!(r.profundidade_maxima_montante(), 2);
        assert_eq!(r.profundidade_maxima_jusante(), 2);
        assert!(!r.montante.contains_key(&r.alvo));
        assert_eq!(lente_estrutura::detectar_ciclos(&g)[0].modulos.len(), 3);
    }

    #[test]
    fn isolado_e_paralelas() {
        let g = grafo("isolado.json");
        let r = calcular_raio(&g, &g.nodes[0].path).unwrap();
        assert_eq!(format!("{:?}", r.classificacao), "Isolado");
        assert_eq!(r.profundidade_maxima_montante(), 0);
        let g = grafo("paralelas.json");
        assert_eq!(g.edges.len(), 2);
        let r = calcular_raio(&g, &g.nodes[0].path).unwrap();
        assert_eq!(r.uses_saida, 1);
        assert_eq!(r.jusante.len(), 1);
    }

    #[test]
    fn invalidos_sao_rejeitados_pelo_adaptador() {
        for nome in [
            "id-duplicado.json",
            "endpoint-ausente.json",
            "endpoint-texto-incompativel.json",
            "enum-invalido.json",
        ] {
            let s = std::fs::read_to_string(arquivo(nome)).unwrap();
            assert!(lente_infra::desserializar_grafo(&s).is_err(), "{nome}");
            assert_eq!(executar(op(&[nome], false)).0, 1);
        }
        let base: Value =
            serde_json::from_str(&std::fs::read_to_string(arquivo("cadeia.json")).unwrap())
                .unwrap();
        for campo in ["kind", "visibility"] {
            let mut v = base.clone();
            v["nodes"][0][campo] = json!("invalido");
            assert!(lente_infra::desserializar_grafo(&v.to_string()).is_err());
        }
        for (id, texto) in [("id_from", "from"), ("id_to", "to")] {
            let mut v = base.clone();
            v["edges"][0][id] = json!(999);
            assert!(lente_infra::desserializar_grafo(&v.to_string()).is_err());
            let mut v = base.clone();
            v["edges"][0][texto] = json!("outro");
            assert!(lente_infra::desserializar_grafo(&v.to_string()).is_err());
        }
    }

    #[test]
    fn colisao_de_paths_e_valida_na_forma_crua_mas_impede_analise() {
        assert_eq!(executar(op(&["paths-repetidos.json"], false)).0, 0);
        let (codigo, texto) = executar(op(&["paths-repetidos.json"], true));
        assert_eq!(codigo, 1);
        assert!(texto.contains("paths únicos"));
    }

    #[test]
    fn isomorfos_renomeados_e_reordenados_conferem() {
        let a = grafo("cadeia.json");
        let b = grafo("cadeia-renomeada.json");
        comparar(&a, &b, &[[1, 90], [2, 70], [3, 10]]).unwrap();
        assert!(comparar(&a, &b, &[[1, 10], [2, 70], [3, 90]]).is_err());
        let mut op = op(&["cadeia.json", "cadeia-renomeada.json"], false);
        op.mapa = Some(arquivo("mapa-cadeia.json"));
        assert_eq!(executar(op).0, 0);
    }

    #[test]
    fn renomeacao_de_ciclo_preserva_pontuacoes_mesmo_mudando_desempate() {
        let a = grafo("ciclo.json");
        let mut b = a.clone();
        let nomes = BTreeMap::from([
            (1, Path::from("novo::z")),
            (2, Path::from("novo::y")),
            (3, Path::from("novo::x")),
        ]);
        for n in &mut b.nodes {
            n.path = nomes[&n.id].clone();
            n.name = n.path.to_string();
            n.id += 10;
        }
        for e in &mut b.edges {
            e.from = nomes[&e.id_from].clone();
            e.to = nomes[&e.id_to].clone();
            e.id_from += 10;
            e.id_to += 10;
        }
        b.nodes.reverse();
        b.edges.reverse();
        assert_eq!(lente_ranking::rankear(&b, 3)[0].path, nomes[&3]);
        comparar(&a, &b, &[[1, 11], [2, 12], [3, 13]]).unwrap();
    }

    #[test]
    fn mapa_exige_bijecao_total_e_ids_existentes() {
        let a = grafo("cadeia.json");
        let b = grafo("cadeia-renomeada.json");
        for mapa in [
            vec![],
            vec![[1, 90]],
            vec![[1, 90], [1, 70], [3, 10]],
            vec![[1, 90], [2, 90], [3, 10]],
            vec![[1, 90], [2, 70], [3, 999]],
        ] {
            assert!(comparar(&a, &b, &mapa).is_err());
        }
    }

    #[test]
    fn comparacao_preserva_multiarestas_e_relacao() {
        let a = grafo("paralelas.json");
        let mut b = a.clone();
        b.edges.pop();
        assert!(comparar(&a, &b, &[[1, 1], [2, 2]]).is_err());
        let mut b = a.clone();
        b.edges[0].relation = lente_core::entities::grafo::Relation::Owns;
        assert!(comparar(&a, &b, &[[1, 1], [2, 2]]).is_err());
    }

    #[test]
    fn falha_nao_esconde_resultados_de_outros_arquivos_e_relatorio_e_estavel() {
        let (codigo, texto) = executar(op(&["nao-existe.json", "cadeia.json"], true));
        assert_eq!(codigo, 1);
        assert!(texto.contains("OK análise"));
        assert!(texto.contains(LIMITE));
        assert_eq!(
            executar(op(&["ciclo.json"], true)),
            executar(op(&["ciclo.json"], true))
        );
    }

    #[test]
    fn argumentos_e_mapa_invalido() {
        let parse = |a: &[&str]| argumentos(a.iter().map(OsString::from));
        assert!(parse(&[]).is_err());
        assert!(parse(&["--mapa"]).is_err());
        assert!(parse(&["--mapa", "m.json", "a.json"]).is_err());
        assert!(parse(&["--outra"]).is_err());
        assert!(parse(&["--help"]).unwrap().is_none());
        assert!(
            parse(&["--mapa", "m.json", "a.json", "b.json"])
                .unwrap()
                .unwrap()
                .analitico
        );
        assert_eq!(
            parse(&["--", "--nome.json"]).unwrap().unwrap().arquivos[0],
            PathBuf::from("--nome.json")
        );
        let mut op = op(&["cadeia.json", "cadeia-renomeada.json"], true);
        op.mapa = Some(arquivo("cadeia.json"));
        assert_eq!(executar(op).0, 1);
    }
}

#[cfg(test)]
mod propriedades;
