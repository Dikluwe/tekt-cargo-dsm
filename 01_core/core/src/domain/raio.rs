//! Crystalline Lineage
//! @prompt 00_nucleo/prompts/raio.md
//! @prompt-hash 26c89869
//! @layer L1
//! @updated 2026-09-26
//! Spec:    00_nucleo/specs/forma-organizada.md
//! ADRs:    00_nucleo/adr/0002-modelagem-do-grafo.md
//! Camada:  L1 — Núcleo. Apenas stdlib. Sem I/O.
//!
//! Cálculo do raio de impacto estrutural de um nó no grafo.
//!
//! Sobre `Uses` (consequência funcional): grau de entrada/saída, montante
//! (quem sente) e jusante (do que depende), com profundidade.
//! Sobre `Owns` (contenção hierárquica): pai e filhos diretos, expostos como
//! contexto — não propagam consequência.
//!
//! `kind` e `visibility` recebidos via `Grafo` não entram no cálculo desta
//! versão (reservados para refinamento interno futuro, sem mudar interface).
//!
//! Limite 4 da spec: as arestas `Uses` vindas de `import` saem do módulo, não
//! do item que de fato usa. O raio reflete esse piso — não inventa
//! granularidade que o grafo não tem.

use core::error::Error;
use core::fmt;
use std::collections::{HashMap, HashSet, VecDeque};

use crate::entities::grafo::{Grafo, Path, Relation};

/// Classificação hierárquica do nó pela topologia `Uses` (extremos puros).
///
/// Sem thresholds arbitrários: a classificação se prende a zero/não-zero nas
/// duas direções. Nuances de "muitos" vs "poucos" se leem nos campos de
/// contagem do `Raio` (`uses_entrada`, `uses_saida`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Classificacao {
    /// Nenhuma `Uses` entrando nem saindo.
    Isolado,
    /// Nenhuma `Uses` entrando; pode ter `Uses` saindo. Ninguém depende dele.
    /// Mexer aqui tem raio contido.
    Folha,
    /// Tem `Uses` entrando; nenhuma `Uses` saindo. Dependem dele, ele não
    /// depende. Mexer aqui tem raio grande.
    Base,
    /// Tem `Uses` entrando e saindo.
    Intermediario,
}

/// Raio de impacto estrutural de um nó no grafo.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Raio {
    pub alvo: Path,
    pub classificacao: Classificacao,
    /// Vizinhos diretos por `Uses` entrando (grau de entrada).
    pub uses_entrada: usize,
    /// Vizinhos diretos por `Uses` saindo (grau de saída).
    pub uses_saida: usize,
    /// Quem sente: cada nó que depende do alvo via `Uses`, com a profundidade
    /// (menor número de saltos a partir do alvo). O alvo **não** está aqui.
    pub montante: HashMap<Path, usize>,
    /// Do que depende: cada nó de que o alvo depende via `Uses`, com
    /// profundidade. O alvo **não** está aqui.
    pub jusante: HashMap<Path, usize>,
    /// Contexto hierárquico (não consequência): módulo que contém o alvo via
    /// `Owns`, se houver.
    pub owns_pai: Option<Path>,
    /// Contexto hierárquico: filhos diretos do alvo via `Owns`. Ordenados por
    /// path para determinismo de testes.
    pub owns_filhos: Vec<Path>,
}

impl Raio {
    /// Maior profundidade alcançada no montante (0 se vazio).
    pub fn profundidade_maxima_montante(&self) -> usize {
        self.montante.values().copied().max().unwrap_or(0)
    }

    /// Maior profundidade alcançada no jusante (0 se vazio).
    pub fn profundidade_maxima_jusante(&self) -> usize {
        self.jusante.values().copied().max().unwrap_or(0)
    }
}

/// Erro do cálculo do raio.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ErroRaio {
    /// O `path` solicitado não corresponde a nenhum nó do grafo.
    AlvoInexistente(Path),
}

impl fmt::Display for ErroRaio {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ErroRaio::AlvoInexistente(p) => {
                write!(f, "alvo inexistente no grafo: {}", p)
            }
        }
    }
}

impl Error for ErroRaio {}

/// Falha de consulta de caminho: identifica qual endpoint não existe.
/// Não altera o contrato de erro de [`calcular_raio`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ErroCaminhoImpacto {
    ItemAlteradoInexistente(Path),
    ConsumidorInexistente(Path),
}

impl fmt::Display for ErroCaminhoImpacto {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ItemAlteradoInexistente(path) => {
                write!(f, "item alterado inexistente no grafo: {}", path)
            }
            Self::ConsumidorInexistente(path) => {
                write!(f, "consumidor inexistente no grafo: {}", path)
            }
        }
    }
}

impl Error for ErroCaminhoImpacto {}

/// Explica uma dependência estrutural por um único caminho mínimo de `Uses`.
///
/// Recebe primeiro o item alterado, mas retorna os paths na direção das arestas:
/// **consumidor → dependências intermediárias → item alterado**, incluindo ambas
/// as pontas. O número de saltos é `caminho.len() - 1`. Para pontas distintas,
/// equivale à profundidade do consumidor no montante de [`calcular_raio`].
///
/// `Ok(None)` indica dois endpoints existentes sem caminho nessa direção.
/// Endpoints ausentes são erros distintos; se ambos faltam, o item alterado tem
/// precedência. Consultar o mesmo nó existente retorna `Some(vec![no])`, caminho
/// reflexivo de zero saltos, mesmo com ciclos. Isso não inclui o alvo no próprio
/// montante do raio, cujo contrato permanece inalterado.
///
/// BFS com vizinhos ordenados por `Path::Ord` escolhe a menor sequência
/// lexicográfica entre os caminhos de menor número de saltos. Independe da ordem
/// dos nós, das arestas e dos IDs. Visita cada path uma vez e guarda apenas um
/// predecessor por path: não enumera todos os caminhos; termina com ciclos.
/// Arestas paralelas são deduplicadas; `Uses` inclui todos os subtipos presentes
/// no grafo recebido; `Owns` nunca propaga impacto.
///
/// Exige a forma resolvida: paths únicos e endpoints íntegros, conforme a spec.
/// É evidência de alcance estrutural, **não prova de quebra comportamental**.
pub fn explicar_caminho_impacto(
    grafo: &Grafo,
    item_alterado: &Path,
    consumidor: &Path,
) -> Result<Option<Vec<Path>>, ErroCaminhoImpacto> {
    if !grafo.nodes.iter().any(|n| &n.path == item_alterado) {
        return Err(ErroCaminhoImpacto::ItemAlteradoInexistente(
            item_alterado.clone(),
        ));
    }
    if !grafo.nodes.iter().any(|n| &n.path == consumidor) {
        return Err(ErroCaminhoImpacto::ConsumidorInexistente(
            consumidor.clone(),
        ));
    }
    if consumidor == item_alterado {
        return Ok(Some(vec![consumidor.clone()]));
    }

    let indices = Indices::construir(grafo);
    let mut visitados = HashSet::from([consumidor.clone()]);
    let mut anteriores: HashMap<Path, Path> = HashMap::new();
    let mut fila = VecDeque::from([consumidor.clone()]);
    while let Some(atual) = fila.pop_front() {
        if &atual == item_alterado {
            let mut caminho = vec![atual];
            while caminho.last() != Some(consumidor) {
                let anterior = &anteriores[caminho.last().expect("caminho não vazio")];
                caminho.push(anterior.clone());
            }
            caminho.reverse();
            return Ok(Some(caminho));
        }
        if let Some(vizinhos) = indices.uses_saida.get(&atual) {
            for vizinho in vizinhos {
                if visitados.insert(vizinho.clone()) {
                    anteriores.insert(vizinho.clone(), atual.clone());
                    fila.push_back(vizinho.clone());
                }
            }
        }
    }
    Ok(None)
}

/// Índices internos. Não expostos.
struct Indices {
    /// Para cada path, vizinhos que apontam para ele via `Uses` (entrada).
    uses_entrada: HashMap<Path, Vec<Path>>,
    /// Para cada path, vizinhos para onde ele aponta via `Uses` (saída).
    uses_saida: HashMap<Path, Vec<Path>>,
    /// Pai por `Owns` (cada nó tem no máximo um pai na árvore de contenção).
    owns_pai: HashMap<Path, Path>,
    /// Filhos diretos por `Owns`.
    owns_filhos: HashMap<Path, Vec<Path>>,
}

impl Indices {
    fn construir(grafo: &Grafo) -> Self {
        let mut uses_entrada: HashMap<Path, Vec<Path>> = HashMap::new();
        let mut uses_saida: HashMap<Path, Vec<Path>> = HashMap::new();
        let mut owns_pai: HashMap<Path, Path> = HashMap::new();
        let mut owns_filhos: HashMap<Path, Vec<Path>> = HashMap::new();

        for aresta in &grafo.edges {
            match aresta.relation {
                Relation::Uses => {
                    uses_saida
                        .entry(aresta.from.clone())
                        .or_default()
                        .push(aresta.to.clone());
                    uses_entrada
                        .entry(aresta.to.clone())
                        .or_default()
                        .push(aresta.from.clone());
                }
                Relation::Owns => {
                    // Owns: from = pai, to = filho.
                    owns_pai.insert(aresta.to.clone(), aresta.from.clone());
                    owns_filhos
                        .entry(aresta.from.clone())
                        .or_default()
                        .push(aresta.to.clone());
                }
            }
        }

        // O grau conta vizinhos, não relações paralelas (Import/Reference).
        for vizinhos in uses_entrada
            .values_mut()
            .chain(uses_saida.values_mut())
            .chain(owns_filhos.values_mut())
        {
            vizinhos.sort();
            vizinhos.dedup();
        }

        Self {
            uses_entrada,
            uses_saida,
            owns_pai,
            owns_filhos,
        }
    }
}

/// Calcula o raio de impacto do nó-alvo.
///
/// Erro se o alvo não existir entre os nós do grafo.
pub fn calcular_raio(grafo: &Grafo, alvo: &Path) -> Result<Raio, ErroRaio> {
    if !grafo.nodes.iter().any(|n| &n.path == alvo) {
        return Err(ErroRaio::AlvoInexistente(alvo.clone()));
    }

    let indices = Indices::construir(grafo);

    let uses_entrada = indices
        .uses_entrada
        .get(alvo)
        .map(Vec::as_slice)
        .unwrap_or(&[])
        .len();
    let uses_saida = indices
        .uses_saida
        .get(alvo)
        .map(Vec::as_slice)
        .unwrap_or(&[])
        .len();

    let classificacao = match (uses_entrada, uses_saida) {
        (0, 0) => Classificacao::Isolado,
        (0, _) => Classificacao::Folha,
        (_, 0) => Classificacao::Base,
        (_, _) => Classificacao::Intermediario,
    };

    let montante = bfs_profundidades(alvo, &indices.uses_entrada);
    let jusante = bfs_profundidades(alvo, &indices.uses_saida);

    let owns_pai = indices.owns_pai.get(alvo).cloned();
    let mut owns_filhos = indices
        .owns_filhos
        .get(alvo)
        .cloned()
        .unwrap_or_default();
    owns_filhos.sort();

    Ok(Raio {
        alvo: alvo.clone(),
        classificacao,
        uses_entrada,
        uses_saida,
        montante,
        jusante,
        owns_pai,
        owns_filhos,
    })
}

/// BFS partindo do alvo na direção do índice dado, retornando profundidades.
/// O alvo não entra no resultado. Termina com ciclos (visitados).
fn bfs_profundidades(
    alvo: &Path,
    vizinhos: &HashMap<Path, Vec<Path>>,
) -> HashMap<Path, usize> {
    let mut profundidades: HashMap<Path, usize> = HashMap::new();
    let mut visitados: HashSet<Path> = HashSet::new();
    let mut fila: VecDeque<(Path, usize)> = VecDeque::new();

    visitados.insert(alvo.clone());
    fila.push_back((alvo.clone(), 0));

    while let Some((no, prof)) = fila.pop_front() {
        if let Some(vs) = vizinhos.get(&no) {
            for v in vs {
                if visitados.insert(v.clone()) {
                    profundidades.insert(v.clone(), prof + 1);
                    fila.push_back((v.clone(), prof + 1));
                }
            }
        }
    }

    profundidades
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entities::grafo::{Aresta, Kind, Modificadores, No, Visibility};

    /// Hash determinístico do path → id. Paths distintos nos testes deste
    /// módulo são suficientemente curtos para nunca colidirem.
    fn id_de(path: &str) -> usize {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut h = DefaultHasher::new();
        path.hash(&mut h);
        h.finish() as usize
    }

    /// Constrói um nó pub mod simples (kind/descritor irrelevantes p/ cálculo).
    fn no(path: &str) -> No {
        No {
            id: id_de(path),
            path: Path::from(path),
            name: path.rsplit("::").next().unwrap_or(path).to_string(),
            kind: Kind::Mod,
            modificadores: Modificadores::default(),
            visibility: Visibility::Pub,
            crate_name: "t".to_string(),
            trait_: None,
            trait_ref: None,
            cfg: None,
            macro_kind: None,
            is_non_exhaustive: false,
            position: None,
        }
    }

    fn aresta(from: &str, to: &str, relation: Relation) -> Aresta {
        Aresta {
            from: Path::from(from),
            id_from: id_de(from),
            to: Path::from(to),
            id_to: id_de(to),
            relation,
            uses_kind: None,
        }
    }

    fn grafo_com(nodes: Vec<&str>, edges: Vec<(&str, &str, Relation)>) -> Grafo {
        Grafo {
            crate_name: "t".to_string(),
            nodes: nodes.into_iter().map(no).collect(),
            edges: edges
                .into_iter()
                .map(|(f, t, r)| aresta(f, t, r))
                .collect(),
        }
    }

    /// Verifica a consulta contra o contrato independente de profundidades do raio.
    fn conferir_explicacao(
        grafo: &Grafo,
        alterado: &str,
        consumidor: &str,
        esperado: Option<Vec<&str>>,
    ) {
        let alterado = Path::from(alterado);
        let consumidor = Path::from(consumidor);
        let antes = grafo.clone();
        let caminho = explicar_caminho_impacto(grafo, &alterado, &consumidor).unwrap();
        assert_eq!(
            caminho,
            esperado.map(|ps| ps.into_iter().map(Path::from).collect())
        );
        assert_eq!(grafo, &antes, "consulta não altera a entrada");
        let raio = calcular_raio(grafo, &alterado).unwrap();
        match caminho {
            Some(caminho) => {
                assert_eq!(caminho.first(), Some(&consumidor));
                assert_eq!(caminho.last(), Some(&alterado));
                if alterado == consumidor {
                    assert_eq!(caminho.len(), 1);
                    assert!(!raio.montante.contains_key(&consumidor));
                } else {
                    assert_eq!(raio.montante.get(&consumidor), Some(&(caminho.len() - 1)));
                }
                for passo in caminho.windows(2) {
                    assert!(grafo.edges.iter().any(|e| e.relation == Relation::Uses
                        && e.from == passo[0]
                        && e.to == passo[1]));
                }
            }
            None => assert!(!raio.montante.contains_key(&consumidor)),
        }
    }

    #[test]
    fn explicacao_cadeia_respeita_direcao_e_profundidade() {
        let g = grafo_com(
            vec!["A", "B", "C"],
            vec![("C", "B", Relation::Uses), ("B", "A", Relation::Uses)],
        );
        conferir_explicacao(&g, "A", "C", Some(vec!["C", "B", "A"]));
        conferir_explicacao(&g, "C", "A", None);
    }

    #[test]
    fn explicacao_diamante_desempata_por_path_nao_ordem_ou_id() {
        let mut g = grafo_com(
            vec!["fim", "z", "a", "inicio"],
            vec![
                ("inicio", "z", Relation::Uses),
                ("z", "fim", Relation::Uses),
                ("inicio", "a", Relation::Uses),
                ("a", "fim", Relation::Uses),
            ],
        );
        conferir_explicacao(&g, "fim", "inicio", Some(vec!["inicio", "a", "fim"]));
        g.nodes.reverse();
        g.edges.reverse();
        // Renumera os IDs mantendo a integridade referencial da fixture.
        for n in &mut g.nodes {
            n.id = usize::MAX - n.id;
        }
        for e in &mut g.edges {
            e.id_from = usize::MAX - e.id_from;
            e.id_to = usize::MAX - e.id_to;
        }
        conferir_explicacao(&g, "fim", "inicio", Some(vec!["inicio", "a", "fim"]));
    }

    #[test]
    fn explicacao_desempate_considera_sequencia_desde_consumidor() {
        // Escolher a partir do alvo invertido favoreceria b→c, incorretamente.
        let g = grafo_com(
            vec!["inicio", "a", "z", "b", "c", "fim"],
            vec![
                ("inicio", "b", Relation::Uses),
                ("b", "c", Relation::Uses),
                ("c", "fim", Relation::Uses),
                ("inicio", "a", Relation::Uses),
                ("a", "z", Relation::Uses),
                ("z", "fim", Relation::Uses),
            ],
        );
        conferir_explicacao(&g, "fim", "inicio", Some(vec!["inicio", "a", "z", "fim"]));
    }

    #[test]
    fn explicacao_menor_distancia_precede_ordem_lexicografica() {
        let g = grafo_com(
            vec!["inicio", "a", "b", "fim"],
            vec![
                ("inicio", "a", Relation::Uses),
                ("a", "b", Relation::Uses),
                ("b", "fim", Relation::Uses),
                ("inicio", "fim", Relation::Uses),
            ],
        );
        conferir_explicacao(&g, "fim", "inicio", Some(vec!["inicio", "fim"]));
    }

    #[test]
    fn explicacao_ciclo_termina_com_e_sem_destino_alcancavel() {
        let g = grafo_com(
            vec!["A", "B", "C", "isolado"],
            vec![
                ("A", "B", Relation::Uses),
                ("B", "A", Relation::Uses),
                ("B", "C", Relation::Uses),
                ("C", "B", Relation::Uses),
            ],
        );
        conferir_explicacao(&g, "C", "A", Some(vec!["A", "B", "C"]));
        conferir_explicacao(&g, "isolado", "A", None);
        conferir_explicacao(&g, "A", "isolado", None);
    }

    #[test]
    fn explicacao_endpoints_ausentes_sao_erros_distintos() {
        let g = grafo_com(vec!["existe"], vec![]);
        let ausente = Path::from("ausente");
        let existe = Path::from("existe");
        let erro_alterado = explicar_caminho_impacto(&g, &ausente, &existe).unwrap_err();
        let erro_consumidor = explicar_caminho_impacto(&g, &existe, &ausente).unwrap_err();
        assert_eq!(
            erro_alterado,
            ErroCaminhoImpacto::ItemAlteradoInexistente(ausente.clone())
        );
        assert_eq!(
            erro_consumidor,
            ErroCaminhoImpacto::ConsumidorInexistente(ausente.clone())
        );
        assert!(
            erro_alterado
                .to_string()
                .contains("item alterado inexistente")
        );
        assert!(
            erro_consumidor
                .to_string()
                .contains("consumidor inexistente")
        );
        assert!(erro_consumidor.to_string().contains("ausente"));
        assert_eq!(
            explicar_caminho_impacto(&g, &ausente, &Path::from("outro")),
            Err(erro_alterado.clone())
        );
        assert_eq!(
            explicar_caminho_impacto(&g, &ausente, &ausente),
            Err(erro_alterado)
        );
    }

    #[test]
    fn explicacao_mesmo_endpoint_e_reflexiva_mesmo_com_ciclo() {
        let mut g = grafo_com(vec!["A", "B"], vec![]);
        conferir_explicacao(&g, "A", "A", Some(vec!["A"]));
        g.edges = vec![
            aresta("A", "B", Relation::Uses),
            aresta("B", "A", Relation::Uses),
            aresta("A", "A", Relation::Uses),
        ];
        conferir_explicacao(&g, "A", "A", Some(vec!["A"]));
    }

    #[test]
    fn explicacao_arestas_paralelas_nao_duplicam_passos() {
        use crate::entities::grafo::UsesKind;
        let mut g = grafo_com(
            vec!["A", "B", "C"],
            vec![
                ("C", "B", Relation::Uses),
                ("C", "B", Relation::Uses),
                ("B", "A", Relation::Uses),
                ("B", "A", Relation::Uses),
            ],
        );
        g.edges[0].uses_kind = Some(UsesKind::Import);
        g.edges[1].uses_kind = Some(UsesKind::Reference);
        conferir_explicacao(&g, "A", "C", Some(vec!["C", "B", "A"]));
    }

    #[test]
    fn explicacao_owns_nao_cria_caminho_nem_atalho() {
        let mut g = grafo_com(vec!["A", "B", "C"], vec![("C", "A", Relation::Owns)]);
        conferir_explicacao(&g, "A", "C", None);
        g.edges.extend([
            aresta("C", "B", Relation::Uses),
            aresta("B", "A", Relation::Uses),
        ]);
        conferir_explicacao(&g, "A", "C", Some(vec!["C", "B", "A"]));
    }

    #[test]
    fn explicacao_concorda_com_raio_para_todos_os_pares() {
        let g = grafo_com(
            vec!["a", "b", "c", "d", "e", "isolado"],
            vec![
                ("a", "b", Relation::Uses),
                ("b", "c", Relation::Uses),
                ("c", "a", Relation::Uses),
                ("a", "d", Relation::Uses),
                ("d", "e", Relation::Uses),
                ("c", "e", Relation::Uses),
                ("a", "isolado", Relation::Owns),
                ("a", "b", Relation::Uses),
            ],
        );
        for alterado in &g.nodes {
            let raio = calcular_raio(&g, &alterado.path).unwrap();
            for consumidor in &g.nodes {
                let caminho =
                    explicar_caminho_impacto(&g, &alterado.path, &consumidor.path).unwrap();
                let saltos = caminho.as_ref().map(|c| c.len() - 1);
                let esperado = if consumidor.path == alterado.path {
                    Some(0)
                } else {
                    raio.montante.get(&consumidor.path).copied()
                };
                assert_eq!(saltos, esperado, "{} → {}", consumidor.path, alterado.path);
            }
        }
    }

    #[test]
    fn arestas_paralelas_contam_um_unico_vizinho() {
        use crate::entities::grafo::UsesKind;
        let mut g = grafo_com(
            vec!["A", "B"],
            vec![
                ("B", "A", Relation::Uses),
                ("B", "A", Relation::Uses),
                ("A", "B", Relation::Owns),
                ("A", "B", Relation::Owns),
            ],
        );
        g.edges[0].uses_kind = Some(UsesKind::Import);
        g.edges[1].uses_kind = Some(UsesKind::Reference);
        let a = calcular_raio(&g, &Path::from("A")).unwrap();
        let b = calcular_raio(&g, &Path::from("B")).unwrap();
        assert_eq!(a.uses_entrada, 1);
        assert_eq!(b.uses_saida, 1);
        assert_eq!(a.montante.len(), 1);
        assert_eq!(a.owns_filhos, vec![Path::from("B")]);
    }

    #[test]
    fn montante_inclui_direto_e_indireto_com_profundidade() {
        // B uses A, C uses B. Raio de A: B em prof 1, C em prof 2.
        let g = grafo_com(
            vec!["A", "B", "C"],
            vec![("B", "A", Relation::Uses), ("C", "B", Relation::Uses)],
        );
        let r = calcular_raio(&g, &Path::from("A")).unwrap();
        assert_eq!(r.montante.get(&Path::from("B")), Some(&1));
        assert_eq!(r.montante.get(&Path::from("C")), Some(&2));
        assert_eq!(r.montante.len(), 2);
        assert_eq!(r.profundidade_maxima_montante(), 2);
    }

    #[test]
    fn folha_tem_montante_vazio() {
        // C usa B usa A. C é folha: ninguém depende de C.
        let g = grafo_com(
            vec!["A", "B", "C"],
            vec![("B", "A", Relation::Uses), ("C", "B", Relation::Uses)],
        );
        let r = calcular_raio(&g, &Path::from("C")).unwrap();
        assert!(r.montante.is_empty());
        assert_eq!(r.classificacao, Classificacao::Folha);
        assert_eq!(r.uses_entrada, 0);
        assert_eq!(r.uses_saida, 1);
    }

    #[test]
    fn base_e_classificado_quando_muitos_dependem_e_ele_nao_depende() {
        // X, Y, Z dependem de A; A não depende de ninguém. A é base.
        let g = grafo_com(
            vec!["A", "X", "Y", "Z"],
            vec![
                ("X", "A", Relation::Uses),
                ("Y", "A", Relation::Uses),
                ("Z", "A", Relation::Uses),
            ],
        );
        let r = calcular_raio(&g, &Path::from("A")).unwrap();
        assert_eq!(r.classificacao, Classificacao::Base);
        assert_eq!(r.uses_entrada, 3);
        assert_eq!(r.uses_saida, 0);
        assert_eq!(r.montante.len(), 3);
        assert!(r.jusante.is_empty());
    }

    #[test]
    fn owns_nao_propaga_consequencia_e_aparece_como_contexto() {
        // M owns I, sem Uses. Raio de I: consequência vazia, owns_pai = M.
        let g = grafo_com(
            vec!["M", "I"],
            vec![("M", "I", Relation::Owns)],
        );
        let r = calcular_raio(&g, &Path::from("I")).unwrap();
        assert!(r.montante.is_empty());
        assert!(r.jusante.is_empty());
        assert_eq!(r.owns_pai, Some(Path::from("M")));
        assert_eq!(r.uses_entrada, 0);
        assert_eq!(r.uses_saida, 0);
        assert_eq!(r.classificacao, Classificacao::Isolado);
    }

    #[test]
    fn owns_filhos_aparecem_ordenados() {
        let g = grafo_com(
            vec!["M", "M::z", "M::a", "M::m"],
            vec![
                ("M", "M::z", Relation::Owns),
                ("M", "M::a", Relation::Owns),
                ("M", "M::m", Relation::Owns),
            ],
        );
        let r = calcular_raio(&g, &Path::from("M")).unwrap();
        assert_eq!(
            r.owns_filhos,
            vec![
                Path::from("M::a"),
                Path::from("M::m"),
                Path::from("M::z"),
            ]
        );
    }

    #[test]
    fn ciclo_termina_e_inclui_alcance() {
        // A uses B, B uses A. Raio de A: B no montante (B depende de A).
        let g = grafo_com(
            vec!["A", "B"],
            vec![("A", "B", Relation::Uses), ("B", "A", Relation::Uses)],
        );
        let r = calcular_raio(&g, &Path::from("A")).unwrap();
        assert_eq!(r.montante.get(&Path::from("B")), Some(&1));
        assert_eq!(r.jusante.get(&Path::from("B")), Some(&1));
        // alvo não entra em si mesmo nem por ciclo
        assert!(!r.montante.contains_key(&Path::from("A")));
        assert!(!r.jusante.contains_key(&Path::from("A")));
        assert_eq!(r.classificacao, Classificacao::Intermediario);
    }

    #[test]
    fn alvo_inexistente_retorna_erro() {
        let g = grafo_com(vec!["A"], vec![]);
        let err = calcular_raio(&g, &Path::from("Z")).unwrap_err();
        assert_eq!(err, ErroRaio::AlvoInexistente(Path::from("Z")));
        // Display contém o path
        assert!(format!("{}", err).contains("Z"));
    }

    #[test]
    fn cadeia_longa_reporta_profundidade_correta() {
        // n0 -> n1 -> n2 -> n3 -> n4 (n_{i+1} usa n_i). Raio de n0:
        // montante {n1:1, n2:2, n3:3, n4:4}.
        let g = grafo_com(
            vec!["n0", "n1", "n2", "n3", "n4"],
            vec![
                ("n1", "n0", Relation::Uses),
                ("n2", "n1", Relation::Uses),
                ("n3", "n2", Relation::Uses),
                ("n4", "n3", Relation::Uses),
            ],
        );
        let r = calcular_raio(&g, &Path::from("n0")).unwrap();
        assert_eq!(r.montante.get(&Path::from("n1")), Some(&1));
        assert_eq!(r.montante.get(&Path::from("n2")), Some(&2));
        assert_eq!(r.montante.get(&Path::from("n3")), Some(&3));
        assert_eq!(r.montante.get(&Path::from("n4")), Some(&4));
        assert_eq!(r.profundidade_maxima_montante(), 4);
    }

    #[test]
    fn grafo_de_um_no_so_da_raio_vazio() {
        let g = grafo_com(vec!["solo"], vec![]);
        let r = calcular_raio(&g, &Path::from("solo")).unwrap();
        assert!(r.montante.is_empty());
        assert!(r.jusante.is_empty());
        assert_eq!(r.classificacao, Classificacao::Isolado);
        assert_eq!(r.owns_pai, None);
        assert!(r.owns_filhos.is_empty());
    }

    #[test]
    fn no_isolado_em_grafo_com_outros_e_isolado() {
        // Outros nós existem e têm Uses, mas X não participa de nenhuma.
        let g = grafo_com(
            vec!["A", "B", "X"],
            vec![("A", "B", Relation::Uses)],
        );
        let r = calcular_raio(&g, &Path::from("X")).unwrap();
        assert_eq!(r.classificacao, Classificacao::Isolado);
        assert_eq!(r.uses_entrada, 0);
        assert_eq!(r.uses_saida, 0);
    }

    #[test]
    fn caminho_mais_curto_e_o_reportado() {
        // n0 -> A, A -> n2, n0 -> n2 (dois caminhos para n2: 1 ou 2 saltos).
        // BFS deve achar o de 1 salto.
        let g = grafo_com(
            vec!["n0", "A", "n2"],
            vec![
                ("n0", "A", Relation::Uses),
                ("A", "n2", Relation::Uses),
                ("n0", "n2", Relation::Uses),
            ],
        );
        let r = calcular_raio(&g, &Path::from("n0")).unwrap();
        assert_eq!(r.jusante.get(&Path::from("n2")), Some(&1));
    }
}
