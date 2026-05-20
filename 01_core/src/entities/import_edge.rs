/*
 * Crystalline Lineage
 * @prompt 00_nucleo/prompts/import_edge.md
 * @layer L1
 * @updated 2026-05-20
 */

use crate::entities::module_tree::NodeId;

/// Classificação de um import quanto à sua origem.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ImportKind {
    /// Import do mesmo crate que faz o uso.
    CurrentCrate,
    /// Import de outro crate dentro do mesmo workspace.
    WorkspaceCrate,
    /// Import de crate externo (crates.io ou similar).
    External,
    /// Import da biblioteca padrão (`std`, `core`, `alloc`).
    Stdlib,
    /// Caminho que não pôde ser classificado.
    Unresolved,
}

impl ImportKind {
    /// Retorna `true` para `CurrentCrate` e `WorkspaceCrate`.
    pub fn is_internal(&self) -> bool {
        matches!(self, ImportKind::CurrentCrate | ImportKind::WorkspaceCrate)
    }

    /// Retorna `true` para `External` e `Stdlib`.
    pub fn is_external(&self) -> bool {
        matches!(self, ImportKind::External | ImportKind::Stdlib)
    }
}

/// Aresta bruta de import entre módulos, produzida pelo extrator de imports (L₃).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ImportEdge {
    /// Nó que faz o import (módulo origem).
    pub from: NodeId,

    /// Identificador canónico do crate de origem.
    pub from_crate: String,

    /// Caminho lógico do módulo alvo, na forma canónica.
    pub target_module: String,

    /// Item específico importado. O segmento final do `use`.
    pub imported_item: String,

    /// Classificação do import.
    pub kind: ImportKind,

    /// O caminho do `use` na forma textual, exactamente como aparece no código fonte.
    pub raw_use_path: String,

    /// `true` se é glob import (`use a::b::*`).
    pub is_glob: bool,

    /// Alias usado em `use X as Y`. `None` se não houver alias.
    pub alias: Option<String>,

    /// `true` se é re-export (`pub use ...`).
    pub is_reexport: bool,
}

impl ImportEdge {
    /// Constrói uma `ImportEdge` com todos os campos.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        from: NodeId,
        from_crate: String,
        target_module: String,
        imported_item: String,
        kind: ImportKind,
        raw_use_path: String,
        is_glob: bool,
        alias: Option<String>,
        is_reexport: bool,
    ) -> Self {
        Self {
            from,
            from_crate,
            target_module,
            imported_item,
            kind,
            raw_use_path,
            is_glob,
            alias,
            is_reexport,
        }
    }

    /// Retorna `true` se o import é interno (CurrentCrate ou WorkspaceCrate).
    pub fn is_internal(&self) -> bool {
        self.kind.is_internal()
    }

    /// Retorna `true` se o import é externo (External ou Stdlib).
    pub fn is_external(&self) -> bool {
        self.kind.is_external()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_edge(kind: ImportKind) -> ImportEdge {
        ImportEdge::new(
            NodeId::test_new(0),
            "my_crate".to_string(),
            "target::module".to_string(),
            "Foo".to_string(),
            kind,
            "target::module::Foo".to_string(),
            false,
            None,
            false,
        )
    }

    #[test]
    fn test_construction_current_crate() {
        let edge = make_edge(ImportKind::CurrentCrate);
        assert_eq!(edge.from_crate, "my_crate");
        assert_eq!(edge.target_module, "target::module");
        assert_eq!(edge.imported_item, "Foo");
        assert_eq!(edge.kind, ImportKind::CurrentCrate);
    }

    #[test]
    fn test_construction_workspace_crate() {
        let edge = make_edge(ImportKind::WorkspaceCrate);
        assert_eq!(edge.kind, ImportKind::WorkspaceCrate);
    }

    #[test]
    fn test_construction_external() {
        let edge = make_edge(ImportKind::External);
        assert_eq!(edge.kind, ImportKind::External);
    }

    #[test]
    fn test_construction_stdlib() {
        let edge = make_edge(ImportKind::Stdlib);
        assert_eq!(edge.kind, ImportKind::Stdlib);
    }

    #[test]
    fn test_construction_unresolved() {
        let edge = make_edge(ImportKind::Unresolved);
        assert_eq!(edge.kind, ImportKind::Unresolved);
    }

    #[test]
    fn test_is_internal_is_external_on_edge() {
        assert!(make_edge(ImportKind::CurrentCrate).is_internal());
        assert!(!make_edge(ImportKind::CurrentCrate).is_external());

        assert!(make_edge(ImportKind::WorkspaceCrate).is_internal());
        assert!(!make_edge(ImportKind::WorkspaceCrate).is_external());

        assert!(!make_edge(ImportKind::External).is_internal());
        assert!(make_edge(ImportKind::External).is_external());

        assert!(!make_edge(ImportKind::Stdlib).is_internal());
        assert!(make_edge(ImportKind::Stdlib).is_external());

        assert!(!make_edge(ImportKind::Unresolved).is_internal());
        assert!(!make_edge(ImportKind::Unresolved).is_external());
    }

    #[test]
    fn test_is_internal_is_external_on_kind() {
        assert!(ImportKind::CurrentCrate.is_internal());
        assert!(!ImportKind::CurrentCrate.is_external());

        assert!(ImportKind::WorkspaceCrate.is_internal());
        assert!(!ImportKind::WorkspaceCrate.is_external());

        assert!(!ImportKind::External.is_internal());
        assert!(ImportKind::External.is_external());

        assert!(!ImportKind::Stdlib.is_internal());
        assert!(ImportKind::Stdlib.is_external());

        assert!(!ImportKind::Unresolved.is_internal());
        assert!(!ImportKind::Unresolved.is_external());
    }

    #[test]
    fn test_partial_eq() {
        let edge1 = make_edge(ImportKind::External);
        let edge2 = make_edge(ImportKind::External);
        assert_eq!(edge1, edge2);

        let edge3 = make_edge(ImportKind::Stdlib);
        assert_ne!(edge1, edge3);
    }

    #[test]
    fn test_hash_consistency() {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let edge1 = make_edge(ImportKind::External);
        let edge2 = make_edge(ImportKind::External);

        let mut h1 = DefaultHasher::new();
        let mut h2 = DefaultHasher::new();
        edge1.hash(&mut h1);
        edge2.hash(&mut h2);
        assert_eq!(h1.finish(), h2.finish());
    }

    #[test]
    fn test_glob_case() {
        let edge = ImportEdge::new(
            NodeId::test_new(0),
            "my_crate".to_string(),
            "a::b".to_string(),
            "*".to_string(),
            ImportKind::External,
            "a::b::*".to_string(),
            true,
            None,
            false,
        );
        assert!(edge.is_glob);
        assert_eq!(edge.imported_item, "*");
        assert!(edge.alias.is_none());
    }

    #[test]
    fn test_alias_case() {
        let edge = ImportEdge::new(
            NodeId::test_new(0),
            "my_crate".to_string(),
            "a".to_string(),
            "Foo".to_string(),
            ImportKind::External,
            "a::Foo".to_string(),
            false,
            Some("Bar".to_string()),
            false,
        );
        assert!(!edge.is_glob);
        assert_eq!(edge.imported_item, "Foo");
        assert_eq!(edge.alias, Some("Bar".to_string()));
    }

    #[test]
    fn test_reexport_case() {
        let edge = ImportEdge::new(
            NodeId::test_new(0),
            "my_crate".to_string(),
            "a".to_string(),
            "Foo".to_string(),
            ImportKind::External,
            "a::Foo".to_string(),
            false,
            None,
            true,
        );
        assert!(edge.is_reexport);
    }
}
