//! Experimento independente. Contrato local: ../README.md.
use lente_core::{
    domain::raio::explicar_caminho_impacto,
    entities::grafo::{Grafo, Kind, Relation, UsesKind},
};
use lente_estrutura::{
    DestinoRelacao, MotivoNaoAgregavel, ProblemaContencao, agregar_por_modulo_detalhado,
};
use serde_json::{Value, json};
use std::{
    collections::{HashMap, HashSet},
    env, fs,
};

fn problemas(ps: &[ProblemaContencao]) -> Vec<Value> {
    ps.iter()
        .map(|p| match p {
            ProblemaContencao::SemModulo => json!({"code":"sem_modulo"}),
            ProblemaContencao::NoAusente { id } => json!({"code":"no_ausente","id":id}),
            ProblemaContencao::MultiplosPais { item, pais } => {
                json!({"code":"multiplos_pais","item":item,"pais":pais})
            }
            ProblemaContencao::Ciclo { membros } => json!({"code":"ciclo_owns","membros":membros}),
        })
        .collect()
}
fn grafo(v: &Value) -> Result<Grafo, String> {
    let g = lente_infra::desserializar_grafo(&v.to_string()).map_err(|e| e.to_string())?;
    let mut ids = HashMap::new();
    let mut paths = HashSet::new();
    for n in &g.nodes {
        if ids.insert(n.id, &n.path).is_some() {
            return Err("IDs repetidos no snapshot".into());
        }
        if !paths.insert(&n.path) {
            return Err(
                "A consulta de caminho exige paths únicos: forneça a forma resolvida".into(),
            );
        }
        if n.id as u128 > 9_007_199_254_740_991 {
            return Err("ID excede inteiro exato no navegador".into());
        }
    }
    for a in &g.edges {
        if ids.get(&a.id_from) != Some(&&a.from) || ids.get(&a.id_to) != Some(&&a.to) {
            return Err("Endpoint ausente ou path inconsistente: snapshot não íntegro".into());
        }
    }
    Ok(g)
}
fn preparar(v: &Value) -> Result<Value, String> {
    if v["format_version"] != 1 {
        return Err("Versão de entrada incompatível".into());
    }
    let g = grafo(&v["graph"])?;
    let d = agregar_por_modulo_detalhado(&g);
    let mut metadata = v["metadata"].clone();
    if !metadata.is_object() {
        metadata = json!({});
    }
    for k in [
        "provider",
        "language",
        "granularity",
        "configuration",
        "execution",
        "semantic_coverage",
    ] {
        if !metadata[k].is_string() {
            metadata[k] = json!("não informado");
        }
    }
    if !matches!(
        metadata["execution"].as_str(),
        Some("complete" | "partial" | "failed")
    ) {
        metadata["execution"] = json!("unknown");
    }
    if !metadata["diagnostics"].is_array() {
        metadata["diagnostics"] = json!([]);
    }
    let relations: Vec<_> = d.relacoes.iter().map(|r| {
        let destination = match &r.destino {
            DestinoRelacao::Agregada{id_modulo_de,id_modulo_para} => json!({"kind":"aggregated","from":id_modulo_de,"to":id_modulo_para}),
            DestinoRelacao::InternaAbsorvida{id_modulo} => json!({"kind":"internal","module":id_modulo}),
            DestinoRelacao::NaoAgregavel(MotivoNaoAgregavel::ContencaoOwns) => json!({"kind":"excluded","reason":"owns_context"}),
            DestinoRelacao::NaoAgregavel(MotivoNaoAgregavel::ContencaoInvalida{origem,destino}) => json!({"kind":"excluded","reason":"invalid_containment","source_problems":problemas(origem),"target_problems":problemas(destino)}),
        };
        json!({"snapshot_index":r.indice_snapshot,"from_id":r.aresta.id_from,"to_id":r.aresta.id_to,
            "from":r.aresta.from.as_str(),"to":r.aresta.to.as_str(),
            "relation": match r.aresta.relation {Relation::Uses=>"uses",Relation::Owns=>"owns"},
            "subtype": match r.aresta.uses_kind {None=>None,Some(UsesKind::Import)=>Some("import"),Some(UsesKind::Reference)=>Some("reference")},
            "reference_position":Value::Null,"destination":destination})
    }).collect();
    let cells: Vec<_> = d.dependencias.iter().map(|c|json!({"from":c.id_de,"to":c.id_para,"weight":c.peso,"evidence_indices":c.evidencias})).collect();
    let mut modules: Vec<_> = g
        .nodes
        .iter()
        .filter(|n| matches!(n.kind, Kind::Mod | Kind::Crate))
        .collect();
    modules.sort_by_key(|n| (&n.path, n.id));
    let mut items: Vec<_> = g.nodes.iter().collect();
    items.sort_by_key(|n| (&n.path, n.id));
    Ok(json!({"format_version":1,"metadata":metadata,
        "aggregated_graph":{"nodes":modules.iter().map(|n|json!({"id":n.id,"path":n.path.as_str()})).collect::<Vec<_>>(),
          "edges":d.dependencias.iter().map(|c|json!({"from":c.id_de,"to":c.id_para,"relation":"uses"})).collect::<Vec<_>>()},
        "weight_semantics":"snapshot_uses_records","cells":cells,"relations":relations,
        "items":items.iter().map(|n|json!({"id":n.id,"path":n.path.as_str()})).collect::<Vec<_>>(),
        "excluded_items":d.itens_sem_contenedor.iter().map(|i|json!({"id":i.id,"path":i.path.as_str(),"problems":problemas(&i.problemas)})).collect::<Vec<_>>(),
        "limitations":["Peso conta registros Uses, inclusive paralelas; não conta locais de uso.",
            "Posições das referências não estão disponíveis. Posições de declaração não as substituem.",
            "Índices das evidências valem somente neste snapshot.",
            "Caminhos são consultados no grafo original de itens, não na projeção de módulos.",
            "Alcance estrutural não comprova quebra comportamental."],
        "source_graph":v["graph"]}))
}
fn consultar(v: &Value, consumidor: usize, alterado: usize) -> Result<Value, String> {
    if v["format_version"] != 1 {
        return Err("Versão de pacote incompatível".into());
    }
    let g = grafo(&v["source_graph"])?;
    let c = g
        .nodes
        .iter()
        .find(|n| n.id == consumidor)
        .ok_or("Consumidor inexistente")?;
    let a = g
        .nodes
        .iter()
        .find(|n| n.id == alterado)
        .ok_or("Item alterado inexistente")?;
    let path = explicar_caminho_impacto(&g, &a.path, &c.path).map_err(|e| e.to_string())?;
    Ok(match path {
        Some(p) => {
            json!({"status":"found","direction":"consumer_to_changed","hops":p.len()-1,"path":p.iter().map(|p|p.as_str()).collect::<Vec<_>>()})
        }
        None => json!({"status":"no_path","direction":"consumer_to_changed","path":[]}),
    })
}
fn run() -> Result<(), String> {
    let args: Vec<_> = env::args().collect();
    if args.len() < 3 {
        return Err("Uso: dsm-explicavel prepare entrada.json | query pacote.json consumidor_id alterado_id".into());
    }
    if args[1] == "query-stdin" && args.len() == 4 {
        let v: Value = serde_json::from_reader(std::io::stdin()).map_err(|e| e.to_string())?;
        let q = consultar(
            &v,
            args[2].parse().map_err(|_| "ID inválido")?,
            args[3].parse().map_err(|_| "ID inválido")?,
        )?;
        println!("{q}");
        return Ok(());
    }
    let v: Value = serde_json::from_str(&fs::read_to_string(&args[2]).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())?;
    let result = match args[1].as_str() {
        "prepare" => preparar(&v)?,
        "query" if args.len() == 5 => consultar(
            &v,
            args[3].parse().map_err(|_| "ID inválido")?,
            args[4].parse().map_err(|_| "ID inválido")?,
        )?,
        _ => return Err("Comando inválido".into()),
    };
    println!(
        "{}",
        serde_json::to_string_pretty(&result).map_err(|e| e.to_string())?
    );
    Ok(())
}
fn main() {
    if let Err(e) = run() {
        eprintln!("{e}");
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> Value {
        serde_json::from_str(include_str!("../fixtures/partial.json")).unwrap()
    }
    #[test]
    fn delegates_and_preserves_evidence() {
        let p = preparar(&fixture()).unwrap();
        let c = p["cells"]
            .as_array()
            .unwrap()
            .iter()
            .find(|c| c["from"] == 10 && c["to"] == 20)
            .unwrap();
        assert_eq!(c["weight"], 3);
        assert_eq!(c["evidence_indices"].as_array().unwrap().len(), 3);
        assert_eq!(
            p["relations"].as_array().unwrap().len(),
            fixture()["graph"]["edges"].as_array().unwrap().len()
        );
        assert!(
            p["relations"]
                .as_array()
                .unwrap()
                .iter()
                .all(|r| r["reference_position"].is_null())
        );
        assert_eq!(p["excluded_items"][0]["id"], 99);
    }
    #[test]
    fn queries_original_items_in_correct_direction() {
        let p = preparar(&fixture()).unwrap();
        let q = consultar(&p, 11, 31).unwrap();
        assert_eq!(q["hops"], 2);
        assert_eq!(q["path"][0], "app::ui::render <&>");
        assert_eq!(q["path"][2], "app::storage::save");
        assert_eq!(consultar(&p, 31, 11).unwrap()["status"], "no_path");
        assert_eq!(consultar(&p, 21, 21).unwrap()["hops"], 0);
        assert!(consultar(&p, 999, 21).is_err());
        assert_eq!(consultar(&p, 10, 30).unwrap()["status"], "no_path");
    }
    #[test]
    fn versions_missing_metadata_and_collisions() {
        let mut f = fixture();
        f["format_version"] = json!(2);
        assert!(preparar(&f).is_err());
        f = fixture();
        f.as_object_mut().unwrap().remove("metadata");
        assert_eq!(preparar(&f).unwrap()["metadata"]["execution"], "unknown");
        f["graph"]["nodes"][0]["path"] = f["graph"]["nodes"][1]["path"].clone();
        assert!(preparar(&f).is_err());
    }
    #[test]
    fn empty_and_deterministic() {
        let f = json!({"format_version":1,"graph":{"crate":"empty","nodes":[],"edges":[]}});
        assert!(
            preparar(&f).unwrap()["cells"]
                .as_array()
                .unwrap()
                .is_empty()
        );
        assert_eq!(preparar(&fixture()), preparar(&fixture()));
    }
}
