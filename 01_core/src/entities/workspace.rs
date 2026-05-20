/*
 * Crystalline Lineage
 * @prompt 00_nucleo/prompts/workspace_resolver.md
 * @layer L1
 * @updated 2026-05-20
 */

use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum EntryKind {
    /// Apenas `lib.rs` (crate biblioteca).
    Library,

    /// Apenas `main.rs` (crate binário).
    Binary,

    /// Tem ambos. Neste caso, `entry_point` aponta para `lib.rs`
    /// (preferência para análise estrutural).
    LibraryAndBinary { main_path: PathBuf },
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct WorkspaceMember {
    /// Nome lógico do crate, conforme declarado em `Cargo.toml`.
    /// Exemplo: "crystalline-dsm-core".
    pub name: String,

    /// Caminho absoluto do diretório raiz do crate (onde está o `Cargo.toml`).
    /// Exemplo: "/home/user/projeto/01_core".
    pub crate_root: PathBuf,

    /// Caminho absoluto do ficheiro de entrada do crate.
    /// Para libs: ".../src/lib.rs".
    /// Para binários: ".../src/main.rs".
    pub entry_point: PathBuf,

    /// Tipo do ponto de entrada: biblioteca, binário, ou ambos.
    pub entry_kind: EntryKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Workspace {
    /// Caminho absoluto da raiz do workspace (onde está o `Cargo.toml` do workspace).
    pub root: PathBuf,

    /// Lista de crates membros do workspace.
    pub members: Vec<WorkspaceMember>,
}

impl Workspace {
    /// Procura um membro pelo nome. Retorna `None` se não encontrado.
    pub fn find_member(&self, name: &str) -> Option<&WorkspaceMember> {
        self.members.iter().find(|m| m.name == name)
    }

    /// Quantidade total de membros.
    pub fn member_count(&self) -> usize {
        self.members.len()
    }

    /// Itera sobre os membros que são bibliotecas (Library ou LibraryAndBinary).
    pub fn libraries(&self) -> impl Iterator<Item = &WorkspaceMember> {
        self.members.iter().filter(|m| {
            matches!(
                m.entry_kind,
                EntryKind::Library | EntryKind::LibraryAndBinary { .. }
            )
        })
    }

    /// Itera sobre os membros que são binários (Binary ou LibraryAndBinary).
    pub fn binaries(&self) -> impl Iterator<Item = &WorkspaceMember> {
        self.members.iter().filter(|m| {
            matches!(
                m.entry_kind,
                EntryKind::Binary | EntryKind::LibraryAndBinary { .. }
            )
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_mock_member(name: &str, kind: EntryKind) -> WorkspaceMember {
        let crate_root = PathBuf::from(format!("/abs/path/{}", name));
        let entry_point = match &kind {
            EntryKind::Library | EntryKind::LibraryAndBinary { .. } => {
                crate_root.join("src/lib.rs")
            }
            EntryKind::Binary => crate_root.join("src/main.rs"),
        };
        WorkspaceMember {
            name: name.to_string(),
            crate_root,
            entry_point,
            entry_kind: kind,
        }
    }

    #[test]
    fn test_workspace_construction() {
        // Workspace vazio
        let ws_empty = Workspace {
            root: PathBuf::from("/abs/path"),
            members: vec![],
        };
        assert_eq!(ws_empty.member_count(), 0);

        // Workspace com 1 membro
        let ws_one = Workspace {
            root: PathBuf::from("/abs/path"),
            members: vec![create_mock_member("crate_a", EntryKind::Library)],
        };
        assert_eq!(ws_one.member_count(), 1);
        assert_eq!(ws_one.members[0].name, "crate_a");

        // Workspace com 3 membros
        let ws_three = Workspace {
            root: PathBuf::from("/abs/path"),
            members: vec![
                create_mock_member("crate_a", EntryKind::Library),
                create_mock_member("crate_b", EntryKind::Binary),
                create_mock_member(
                    "crate_c",
                    EntryKind::LibraryAndBinary {
                        main_path: PathBuf::from("/abs/path/crate_c/src/main.rs"),
                    },
                ),
            ],
        };
        assert_eq!(ws_three.member_count(), 3);
    }

    #[test]
    fn test_find_member() {
        let ws = Workspace {
            root: PathBuf::from("/abs/path"),
            members: vec![
                create_mock_member("crate_a", EntryKind::Library),
                create_mock_member("crate_b", EntryKind::Binary),
            ],
        };

        assert!(ws.find_member("crate_a").is_some());
        assert_eq!(ws.find_member("crate_a").unwrap().name, "crate_a");
        assert!(ws.find_member("crate_b").is_some());
        assert!(ws.find_member("crate_c").is_none());
    }

    #[test]
    fn test_member_count() {
        let ws_0 = Workspace {
            root: PathBuf::from("/abs/path"),
            members: vec![],
        };
        assert_eq!(ws_0.member_count(), 0);

        let ws_1 = Workspace {
            root: PathBuf::from("/abs/path"),
            members: vec![create_mock_member("crate_a", EntryKind::Library)],
        };
        assert_eq!(ws_1.member_count(), 1);

        let ws_3 = Workspace {
            root: PathBuf::from("/abs/path"),
            members: vec![
                create_mock_member("crate_a", EntryKind::Library),
                create_mock_member("crate_b", EntryKind::Binary),
                create_mock_member("crate_c", EntryKind::Library),
            ],
        };
        assert_eq!(ws_3.member_count(), 3);
    }

    #[test]
    fn test_libraries_and_binaries_filtering() {
        let ws = Workspace {
            root: PathBuf::from("/abs/path"),
            members: vec![
                create_mock_member("crate_lib", EntryKind::Library),
                create_mock_member("crate_bin", EntryKind::Binary),
                create_mock_member(
                    "crate_both",
                    EntryKind::LibraryAndBinary {
                        main_path: PathBuf::from("/abs/path/crate_both/src/main.rs"),
                    },
                ),
            ],
        };

        // Deve filtrar bibliotecas (crate_lib e crate_both)
        let libs: Vec<&WorkspaceMember> = ws.libraries().collect();
        assert_eq!(libs.len(), 2);
        assert!(libs.iter().any(|m| m.name == "crate_lib"));
        assert!(libs.iter().any(|m| m.name == "crate_both"));

        // Deve filtrar binários (crate_bin e crate_both)
        let bins: Vec<&WorkspaceMember> = ws.binaries().collect();
        assert_eq!(bins.len(), 2);
        assert!(bins.iter().any(|m| m.name == "crate_bin"));
        assert!(bins.iter().any(|m| m.name == "crate_both"));
    }

    #[test]
    fn test_partial_eq() {
        let ws_a = Workspace {
            root: PathBuf::from("/abs/path"),
            members: vec![create_mock_member("crate_a", EntryKind::Library)],
        };

        let ws_b = Workspace {
            root: PathBuf::from("/abs/path"),
            members: vec![create_mock_member("crate_a", EntryKind::Library)],
        };

        let ws_c = Workspace {
            root: PathBuf::from("/abs/path"),
            members: vec![create_mock_member("crate_b", EntryKind::Library)],
        };

        assert_eq!(ws_a, ws_b);
        assert_ne!(ws_a, ws_c);
    }
}
