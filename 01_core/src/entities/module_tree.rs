/*
 * Crystalline Lineage
 * @prompt 00_nucleo/prompts/module_tree.md
 * @layer L1
 * @updated 2026-05-20
 */

use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ModuleNode {
    /// Identificador canónico completo.
    /// Ex: "crystalline_dsm_core::entities::workspace".
    pub canonical_path: String,

    /// Nome do crate ao qual este módulo pertence.
    pub crate_name: String,

    /// Caminho lógico do módulo dentro do crate, segmento por segmento.
    /// Ex: ["entities", "workspace"].
    pub module_path: Vec<String>,

    /// Ficheiro físico que contém este módulo.
    pub source_file: PathBuf,

    /// `true` se o módulo é declarado inline (`mod foo { ... }`).
    pub is_inline: bool,

    /// `true` se o módulo foi declarado com atributo `#[path = "..."]`.
    pub has_custom_path: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NodeId(pub(crate) usize);

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum TreeError {
    #[error("NodeId inválido para esta árvore: {0:?}")]
    InvalidParent(NodeId),

    #[error("Já existe um módulo com este nome ({name}) como filho do nó pai")]
    DuplicateChild { name: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModuleTree {
    /// Nome do crate.
    pub crate_name: String,

    /// Todos os nós da árvore, indexados por `NodeId`.
    nodes: Vec<ModuleNode>,

    /// Para cada nó, a lista de filhos directos.
    children: Vec<Vec<NodeId>>,

    /// Para cada nó (excepto raiz), o `NodeId` do pai.
    parents: Vec<Option<NodeId>>,
}

impl ModuleTree {
    /// Cria uma nova árvore com apenas o nó raiz.
    pub fn new(crate_name: String, root_file: PathBuf) -> Self {
        let root_node = ModuleNode {
            canonical_path: crate_name.clone(),
            crate_name: crate_name.clone(),
            module_path: Vec::new(),
            source_file: root_file,
            is_inline: false,
            has_custom_path: false,
        };

        Self {
            crate_name,
            nodes: vec![root_node],
            children: vec![Vec::new()],
            parents: vec![None],
        }
    }

    /// Adiciona um nó filho a um nó existente.
    pub fn add_child(
        &mut self,
        parent: NodeId,
        module_name: String,
        source_file: PathBuf,
        is_inline: bool,
        has_custom_path: bool,
    ) -> Result<NodeId, TreeError> {
        if parent.0 >= self.nodes.len() {
            return Err(TreeError::InvalidParent(parent));
        }

        // Verifica se já existe um filho direto com o mesmo nome
        for &child_id in &self.children[parent.0] {
            let child_node = &self.nodes[child_id.0];
            if child_node.module_path.last().map(|s| s.as_str()) == Some(&module_name) {
                return Err(TreeError::DuplicateChild { name: module_name });
            }
        }

        let parent_node = &self.nodes[parent.0];
        let child_canonical_path = format!("{}::{}", parent_node.canonical_path, module_name);

        let mut child_module_path = parent_node.module_path.clone();
        child_module_path.push(module_name);

        let child_node = ModuleNode {
            canonical_path: child_canonical_path,
            crate_name: self.crate_name.clone(),
            module_path: child_module_path,
            source_file,
            is_inline,
            has_custom_path,
        };

        let new_id = NodeId(self.nodes.len());
        self.nodes.push(child_node);
        self.children.push(Vec::new());
        self.parents.push(Some(parent));

        self.children[parent.0].push(new_id);

        Ok(new_id)
    }

    /// Retorna o `NodeId` do nó raiz. Sempre `NodeId(0)`.
    pub fn root(&self) -> NodeId {
        NodeId(0)
    }

    /// Retorna referência ao nó pelo seu ID.
    pub fn node(&self, id: NodeId) -> &ModuleNode {
        &self.nodes[id.0]
    }

    /// Retorna lista de filhos directos do nó.
    pub fn children(&self, id: NodeId) -> &[NodeId] {
        &self.children[id.0]
    }

    /// Retorna o pai do nó, ou `None` se for a raiz.
    pub fn parent(&self, id: NodeId) -> Option<NodeId> {
        self.parents[id.0]
    }

    /// Itera sobre todos os nós em ordem de inserção (BFS-friendly).
    pub fn all_nodes(&self) -> impl Iterator<Item = (NodeId, &ModuleNode)> {
        self.nodes.iter().enumerate().map(|(i, n)| (NodeId(i), n))
    }

    /// Quantidade total de nós (incluindo raiz).
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    /// Procura um nó pelo `canonical_path`.
    pub fn find_by_canonical_path(&self, path: &str) -> Option<NodeId> {
        self.nodes
            .iter()
            .position(|n| n.canonical_path == path)
            .map(NodeId)
    }

    /// Itera os nós em pré-ordem (raiz primeiro, depois filhos recursivamente).
    pub fn iter_preorder(&self) -> impl Iterator<Item = (NodeId, &ModuleNode)> {
        let mut order = Vec::new();
        self.dfs_preorder(self.root(), &mut order);
        order.into_iter().map(move |id| (id, &self.nodes[id.0]))
    }

    /// Itera os nós em pós-ordem (filhos primeiro, raiz por último).
    pub fn iter_postorder(&self) -> impl Iterator<Item = (NodeId, &ModuleNode)> {
        let mut order = Vec::new();
        self.dfs_postorder(self.root(), &mut order);
        order.into_iter().map(move |id| (id, &self.nodes[id.0]))
    }

    fn dfs_preorder(&self, id: NodeId, result: &mut Vec<NodeId>) {
        result.push(id);
        for &child in &self.children[id.0] {
            self.dfs_preorder(child, result);
        }
    }

    fn dfs_postorder(&self, id: NodeId, result: &mut Vec<NodeId>) {
        for &child in &self.children[id.0] {
            self.dfs_postorder(child, result);
        }
        result.push(id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_creates_tree_with_root() {
        let root_file = PathBuf::from("src/lib.rs");
        let tree = ModuleTree::new("my_crate".to_string(), root_file.clone());

        assert_eq!(tree.node_count(), 1);
        let root_id = tree.root();
        assert_eq!(root_id, NodeId(0));

        let root_node = tree.node(root_id);
        assert_eq!(root_node.canonical_path, "my_crate");
        assert_eq!(root_node.crate_name, "my_crate");
        assert!(root_node.module_path.is_empty());
        assert_eq!(root_node.source_file, root_file);
        assert!(!root_node.is_inline);
        assert!(!root_node.has_custom_path);
    }

    #[test]
    fn test_add_child_on_root() {
        let mut tree = ModuleTree::new("my_crate".to_string(), PathBuf::from("src/lib.rs"));
        let root = tree.root();

        let child_file = PathBuf::from("src/a.rs");
        let child_id = tree
            .add_child(root, "a".to_string(), child_file.clone(), false, false)
            .expect("Falha ao adicionar filho");

        assert_eq!(tree.node_count(), 2);
        let child_node = tree.node(child_id);
        assert_eq!(child_node.canonical_path, "my_crate::a");
        assert_eq!(child_node.module_path, vec!["a"]);
        assert_eq!(child_node.source_file, child_file);
        assert!(!child_node.is_inline);
        assert!(!child_node.has_custom_path);
    }

    #[test]
    fn test_add_child_in_depth() {
        let mut tree = ModuleTree::new("my_crate".to_string(), PathBuf::from("src/lib.rs"));
        let root = tree.root();

        let a = tree
            .add_child(root, "a".to_string(), PathBuf::from("src/a.rs"), false, false)
            .unwrap();
        let b = tree
            .add_child(a, "b".to_string(), PathBuf::from("src/a/b.rs"), false, false)
            .unwrap();
        let c = tree
            .add_child(
                b,
                "c".to_string(),
                PathBuf::from("src/a/b/c.rs"),
                false,
                false,
            )
            .unwrap();

        assert_eq!(tree.node(c).canonical_path, "my_crate::a::b::c");
        assert_eq!(tree.node(c).module_path, vec!["a", "b", "c"]);
    }

    #[test]
    fn test_inspection_methods() {
        let mut tree = ModuleTree::new("my_crate".to_string(), PathBuf::from("src/lib.rs"));
        let root = tree.root();

        let a = tree
            .add_child(root, "a".to_string(), PathBuf::from("src/a.rs"), false, false)
            .unwrap();

        assert_eq!(tree.children(root), &[a]);
        assert_eq!(tree.parent(a), Some(root));
        assert_eq!(tree.parent(root), None);
    }

    #[test]
    fn test_duplicate_child_error() {
        let mut tree = ModuleTree::new("my_crate".to_string(), PathBuf::from("src/lib.rs"));
        let root = tree.root();

        tree.add_child(root, "a".to_string(), PathBuf::from("src/a.rs"), false, false)
            .unwrap();
        let err = tree
            .add_child(root, "a".to_string(), PathBuf::from("src/b.rs"), false, false)
            .unwrap_err();

        assert_eq!(err, TreeError::DuplicateChild { name: "a".to_string() });
    }

    #[test]
    fn test_invalid_parent_error() {
        let mut tree = ModuleTree::new("my_crate".to_string(), PathBuf::from("src/lib.rs"));
        let invalid_id = NodeId(999);

        let err = tree
            .add_child(
                invalid_id,
                "a".to_string(),
                PathBuf::from("src/a.rs"),
                false,
                false,
            )
            .unwrap_err();

        assert_eq!(err, TreeError::InvalidParent(invalid_id));
    }

    #[test]
    fn test_traversals_order() {
        // Árvore:
        //      root
        //     /    \
        //    a      b
        //   /
        //  c
        let mut tree = ModuleTree::new("my_crate".to_string(), PathBuf::from("src/lib.rs"));
        let root = tree.root();

        let a = tree
            .add_child(root, "a".to_string(), PathBuf::from("src/a.rs"), false, false)
            .unwrap();
        let b = tree
            .add_child(root, "b".to_string(), PathBuf::from("src/b.rs"), false, false)
            .unwrap();
        let c = tree
            .add_child(a, "c".to_string(), PathBuf::from("src/a/c.rs"), false, false)
            .unwrap();

        // Pre-order: root, a, c, b
        let pre_order: Vec<NodeId> = tree.iter_preorder().map(|(id, _)| id).collect();
        assert_eq!(pre_order, vec![root, a, c, b]);

        // Post-order: c, a, b, root
        let post_order: Vec<NodeId> = tree.iter_postorder().map(|(id, _)| id).collect();
        assert_eq!(post_order, vec![c, a, b, root]);
    }

    #[test]
    fn test_find_by_canonical_path() {
        let mut tree = ModuleTree::new("my_crate".to_string(), PathBuf::from("src/lib.rs"));
        let root = tree.root();

        let a = tree
            .add_child(root, "a".to_string(), PathBuf::from("src/a.rs"), false, false)
            .unwrap();

        assert_eq!(tree.find_by_canonical_path("my_crate"), Some(root));
        assert_eq!(tree.find_by_canonical_path("my_crate::a"), Some(a));
        assert_eq!(tree.find_by_canonical_path("my_crate::b"), None);
    }

    #[test]
    fn test_partial_eq() {
        let mut tree1 = ModuleTree::new("my_crate".to_string(), PathBuf::from("src/lib.rs"));
        let mut tree2 = ModuleTree::new("my_crate".to_string(), PathBuf::from("src/lib.rs"));

        assert_eq!(tree1, tree2);

        let root1 = tree1.root();
        let root2 = tree2.root();

        tree1
            .add_child(root1, "a".to_string(), PathBuf::from("src/a.rs"), false, false)
            .unwrap();
        tree2
            .add_child(root2, "a".to_string(), PathBuf::from("src/a.rs"), false, false)
            .unwrap();

        assert_eq!(tree1, tree2);

        tree2
            .add_child(root2, "b".to_string(), PathBuf::from("src/b.rs"), false, false)
            .unwrap();
        assert_ne!(tree1, tree2);
    }

    #[test]
    fn test_inline_module_handling() {
        let mut tree = ModuleTree::new("my_crate".to_string(), PathBuf::from("src/lib.rs"));
        let root = tree.root();

        let a = tree
            .add_child(root, "a".to_string(), PathBuf::from("src/lib.rs"), true, false)
            .unwrap();

        let node_a = tree.node(a);
        assert_eq!(node_a.source_file, PathBuf::from("src/lib.rs"));
        assert!(node_a.is_inline);
        assert!(!node_a.has_custom_path);
    }
}
