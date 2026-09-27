//! Fronteira de arquivos; contrato: ../CONTRATO.md. Não altera a biblioteca pura.
use lente_impacto_snapshots::*;
use serde::Deserialize;
use serde_json::{Value, json};

#[derive(Deserialize, Default)]
#[serde(rename_all = "snake_case")]
enum Cobertura {
    Completa,
    Parcial,
    #[default]
    Desconhecida,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SnapshotDto {
    versao: String,
    grafo: Value,
    granularidade: String,
    natureza: String,
    #[serde(default)]
    completude: Cobertura,
    #[serde(default)]
    diagnosticos: Vec<String>,
}
impl SnapshotDto {
    fn converter(self) -> Result<Snapshot, String> {
        Ok(Snapshot {
            versao: self.versao,
            grafo: lente_infra::desserializar_grafo(&self.grafo.to_string())
                .map_err(|e| e.to_string())?,
            granularidade: self.granularidade,
            natureza: self.natureza,
            completude: match self.completude {
                Cobertura::Completa => Completude::Completa,
                Cobertura::Parcial => Completude::Parcial,
                Cobertura::Desconhecida => Completude::Desconhecida,
            },
            diagnosticos: self.diagnosticos,
        })
    }
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ParDto {
    identidade: String,
    antes: usize,
    depois: usize,
}
#[derive(Deserialize)]
#[serde(tag = "tipo", rename_all = "snake_case", deny_unknown_fields)]
enum MudancaDto {
    Remocao {
        antes: usize,
    },
    Adicao {
        depois: usize,
    },
    Pareada {
        identidade: String,
        alteracao_declarada: bool,
        renomeacao_declarada: bool,
    },
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Entrada {
    format_version: u32,
    antes: SnapshotDto,
    depois: SnapshotDto,
    correspondencias: Vec<ParDto>,
    mudancas: Vec<MudancaDto>,
}
fn cobertura(c: Completude) -> &'static str {
    match c {
        Completude::Completa => "completa",
        Completude::Parcial => "parcial",
        Completude::Desconhecida => "desconhecida",
    }
}
fn origem(o: &Origem) -> Value {
    json!({"lado":match o.lado { Lado::Antes => "antes", Lado::Depois => "depois" }, "versao":o.versao,"completude":cobertura(o.completude)})
}
fn elemento(e: &Elemento) -> Value {
    json!({"origem":origem(&e.origem),"id_local":e.id_local,"path":e.path.as_str(),"identidade":e.identidade})
}
fn alcance(a: &Alcance) -> Value {
    json!({"elemento":elemento(&a.elemento),"profundidade":a.profundidade})
}
fn impacto(i: &ImpactoSnapshot) -> Value {
    use lente_core::domain::raio::Classificacao::*;
    json!({"alvo":elemento(&i.alvo),"classificacao":match i.classificacao { Isolado=>"isolado", Folha=>"folha", Base=>"base", Intermediario=>"intermediario" },
        "entradas_diretas":i.entradas_diretas,"saidas_diretas":i.saidas_diretas,
        "consumidores":i.consumidores.iter().map(alcance).collect::<Vec<_>>(),"dependencias":i.dependencias.iter().map(alcance).collect::<Vec<_>>()})
}
fn unilateral(u: &ObservacaoUnilateral) -> Value {
    json!({"consumidor":alcance(&u.consumidor),"contraparte":u.contraparte.as_ref().map(elemento),
    "estado_outro_lado":match u.estado_outro_lado {
        EstadoOutroLado::SemCorrespondencia=>"sem_correspondencia",
        EstadoOutroLado::AlvoNaoComparavel=>"alvo_nao_comparavel",
        EstadoOutroLado::NaoAlcancadoNoGrafoCompleto=>"nao_alcancado_no_grafo_completo",
        EstadoOutroLado::NaoObservadoComCoberturaLimitada=>"nao_observado_com_cobertura_limitada",
    }})
}
fn relacao(r: &RelacaoObservada) -> Value {
    json!({"de":elemento(&r.de),"para":elemento(&r.para),"subtipo":match r.subtipo {Subtipo::Desconhecido=>"desconhecido",Subtipo::Import=>"import",Subtipo::Reference=>"reference"}})
}
fn topologia(t: &TopologiaIncidente) -> Value {
    json!({"status":match t.status {StatusTopologia::MesmasRelacoesObservadas=>"mesmas_relacoes_observadas",StatusTopologia::DiferencasNasRelacoesObservadas=>"diferencas_nas_relacoes_observadas",StatusTopologia::ComparacaoLimitada=>"comparacao_limitada"},
        "cobertura_limitada":t.cobertura_limitada,"endpoints_sem_par":t.endpoints_sem_par,
        "comuns":t.comuns.iter().map(|r|json!({"antes":relacao(&r.antes),"depois":relacao(&r.depois)})).collect::<Vec<_>>(),
        "somente_antes":t.somente_antes.iter().map(relacao).collect::<Vec<_>>(),"somente_depois":t.somente_depois.iter().map(relacao).collect::<Vec<_>>()})
}
fn mudanca(m: &Mudanca) -> Value {
    match m {
        Mudanca::Remocao { antes } => json!({"tipo":"remocao","antes":antes}),
        Mudanca::Adicao { depois } => json!({"tipo":"adicao","depois":depois}),
        Mudanca::Pareada {
            identidade,
            alteracao_declarada,
            renomeacao_declarada,
        } => {
            json!({"tipo":"pareada","identidade":identidade,"alteracao_declarada":alteracao_declarada,"renomeacao_declarada":renomeacao_declarada})
        }
    }
}
fn descricao(s: &DescricaoSnapshot) -> Value {
    json!({"origem":origem(&s.origem),"granularidade":s.granularidade,"natureza":s.natureza,"diagnosticos":s.diagnosticos})
}
fn analisar(texto: &str) -> Result<Value, String> {
    let e: Entrada = serde_json::from_str(texto).map_err(|e| e.to_string())?;
    if e.format_version != 1 {
        return Err("format_version deve ser 1".into());
    }
    let pares: Vec<_> = e
        .correspondencias
        .into_iter()
        .map(|p| Correspondencia {
            identidade: p.identidade,
            antes: p.antes,
            depois: p.depois,
        })
        .collect();
    let mudancas: Vec<_> = e
        .mudancas
        .into_iter()
        .map(|m| match m {
            MudancaDto::Remocao { antes } => Mudanca::Remocao { antes },
            MudancaDto::Adicao { depois } => Mudanca::Adicao { depois },
            MudancaDto::Pareada {
                identidade,
                alteracao_declarada,
                renomeacao_declarada,
            } => Mudanca::Pareada {
                identidade,
                alteracao_declarada,
                renomeacao_declarada,
            },
        })
        .collect();
    let r = comparar_impacto(
        &e.antes.converter()?,
        &e.depois.converter()?,
        &pares,
        &mudancas,
    )
    .map_err(|es| {
        es.iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join("; ")
    })?;
    Ok(
        json!({"format_version":1,"antes":descricao(&r.antes),"depois":descricao(&r.depois),
        "correspondencias":r.correspondencias.iter().map(|p|json!({"identidade":p.identidade,"antes":p.antes,"depois":p.depois})).collect::<Vec<_>>(),
        "sem_par_antes":r.sem_par_antes.iter().map(elemento).collect::<Vec<_>>(),"sem_par_depois":r.sem_par_depois.iter().map(elemento).collect::<Vec<_>>(),
        "mudancas":r.mudancas.iter().map(|m|json!({"mudanca":mudanca(&m.mudanca),"antes":m.antes.as_ref().map(impacto),"depois":m.depois.as_ref().map(impacto),
            "consumidores":{"comparacao_bilateral":m.consumidores.comparacao_bilateral,
                "somente_antes":m.consumidores.somente_antes.iter().map(unilateral).collect::<Vec<_>>(),
                "somente_depois":m.consumidores.somente_depois.iter().map(unilateral).collect::<Vec<_>>(),
                "ambos":m.consumidores.ambos.iter().map(|c|json!({"identidade":c.identidade,"antes":alcance(&c.antes),"depois":alcance(&c.depois)})).collect::<Vec<_>>()},
            "topologia_incidente":m.topologia_incidente.as_ref().map(topologia)})).collect::<Vec<_>>(),"limites":r.limites}),
    )
}
fn main() -> std::process::ExitCode {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    let resultado = if args.len() != 1 {
        Err("uso: lente-impacto-snapshots-cli ENTRADA.json".into())
    } else {
        std::fs::read_to_string(&args[0])
            .map_err(|e| e.to_string())
            .and_then(|s| analisar(&s))
    };
    match resultado {
        Ok(v) => {
            println!("{v}");
            std::process::ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("entrada_invalida: {e}");
            std::process::ExitCode::from(3)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn entrada() -> Value {
        let snapshot = json!({"versao":"v1","grafo":{"crate":"x","nodes":[],"edges":[]},"granularidade":"arquivo","natureza":"import"});
        json!({"format_version":1,"antes":snapshot,"depois":snapshot,"correspondencias":[],"mudancas":[]})
    }
    #[test]
    fn ausencia_de_completude_permanece_desconhecida() {
        let v = analisar(&entrada().to_string()).unwrap();
        assert_eq!(v["antes"]["origem"]["completude"], "desconhecida");
    }
    #[test]
    fn rejeita_campo_e_versao_desconhecidos() {
        let mut e = entrada();
        e["format_version"] = json!(2);
        assert!(analisar(&e.to_string()).is_err());
        e["format_version"] = json!(1);
        e["antes"]["completo"] = json!(true);
        assert!(analisar(&e.to_string()).is_err());
    }
}
