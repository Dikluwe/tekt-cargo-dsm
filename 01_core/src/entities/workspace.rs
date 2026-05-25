/*
 * Crystalline Lineage
 * @prompt 00_nucleo/prompts/workspace_resolver.md
 * @prompt 00_nucleo/prompts/workspace_entity-revisao.md
 * @layer L1
 * @updated 2026-05-20
 */

use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum EntryKind {
    /// Crate com apenas `lib.rs`.
    Library { lib_path: PathBuf },

    /// Crate com apenas `main.rs`.
    Binary { main_path: PathBuf },

    /// Crate com `lib.rs` E `main.rs`.
    LibraryAndBinary {
        lib_path: PathBuf,
        main_path: PathBuf,
    },

    /// Crate com `lib.rs` marcado como `proc-macro = true`.
    /// Mesma estrutura física que `Library`, mas semântica
    /// diferente para análise de imports.
    ProcMacro { lib_path: PathBuf },

    /// Crate sem lib/bin, mas com targets de teste integrados.
    /// `test_paths` lista os ficheiros de teste descobertos
    /// (geralmente em `tests/*.rs`).
    /// Garantido: `test_paths` não está vazio.
    TestsOnly { test_paths: Vec<PathBuf> },

    /// Crate sem nenhum target compilável conhecido.
    /// Ex: membros de workspace usados apenas para configuração.
    /// Não tem caminho de entrada.
    NoSourceTarget,
}

impl EntryKind {
    /// Retorna o ficheiro de entrada primário, se houver:
    /// - Library, ProcMacro: `lib_path`.
    /// - Binary: `main_path`.
    /// - LibraryAndBinary: `lib_path` (preferência por lib).
    /// - TestsOnly: primeiro elemento de `test_paths`.
    /// - NoSourceTarget: None.
    pub fn primary_entry(&self) -> Option<&Path> {
        match self {
            EntryKind::Library { lib_path } => Some(lib_path),
            EntryKind::Binary { main_path } => Some(main_path),
            EntryKind::LibraryAndBinary { lib_path, .. } => Some(lib_path),
            EntryKind::ProcMacro { lib_path } => Some(lib_path),
            EntryKind::TestsOnly { test_paths } => test_paths.first().map(|p| p.as_path()),
            EntryKind::NoSourceTarget => None,
        }
    }

    /// `true` para variantes com código compilável tradicional:
    /// `Library`, `Binary`, `LibraryAndBinary`, `ProcMacro`.
    /// `false` para `TestsOnly` e `NoSourceTarget`.
    pub fn has_main_source(&self) -> bool {
        matches!(
            self,
            EntryKind::Library { .. }
                | EntryKind::Binary { .. }
                | EntryKind::LibraryAndBinary { .. }
                | EntryKind::ProcMacro { .. }
        )
    }

    /// `true` apenas para `TestsOnly`.
    pub fn is_tests_only(&self) -> bool {
        matches!(self, EntryKind::TestsOnly { .. })
    }

    /// `true` apenas para `NoSourceTarget`.
    pub fn is_empty(&self) -> bool {
        matches!(self, EntryKind::NoSourceTarget)
    }

    /// `true` apenas para `ProcMacro`.
    pub fn is_proc_macro(&self) -> bool {
        matches!(self, EntryKind::ProcMacro { .. })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct WorkspaceMember {
    /// Nome lógico do crate, conforme declarado em `Cargo.toml`.
    /// Exemplo: "crystalline-dsm-core".
    pub name: String,

    /// Caminho absoluto do diretório raiz do crate (onde está o `Cargo.toml`).
    /// Exemplo: "/home/user/projeto/01_core".
    pub crate_root: PathBuf,

    /// Tipo do crate, com caminhos de entrada embutidos.
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

    /// Itera sobre os membros que são bibliotecas (Library, LibraryAndBinary, ProcMacro).
    pub fn libraries(&self) -> impl Iterator<Item = &WorkspaceMember> {
        self.members.iter().filter(|m| {
            matches!(
                m.entry_kind,
                EntryKind::Library { .. }
                    | EntryKind::LibraryAndBinary { .. }
                    | EntryKind::ProcMacro { .. }
            )
        })
    }

    /// Itera sobre os membros que são binários (Binary ou LibraryAndBinary).
    pub fn binaries(&self) -> impl Iterator<Item = &WorkspaceMember> {
        self.members.iter().filter(|m| {
            matches!(
                m.entry_kind,
                EntryKind::Binary { .. } | EntryKind::LibraryAndBinary { .. }
            )
        })
    }

    /// Itera apenas membros com código tradicional (Library,
    /// Binary, LibraryAndBinary, ProcMacro).
    pub fn members_with_code(&self) -> impl Iterator<Item = &WorkspaceMember> {
        self.members
            .iter()
            .filter(|m| m.entry_kind.has_main_source())
    }

    /// Itera apenas membros do tipo TestsOnly.
    pub fn tests_only_members(&self) -> impl Iterator<Item = &WorkspaceMember> {
        self.members.iter().filter(|m| m.entry_kind.is_tests_only())
    }

    /// Itera apenas membros do tipo ProcMacro.
    pub fn proc_macro_members(&self) -> impl Iterator<Item = &WorkspaceMember> {
        self.members.iter().filter(|m| m.entry_kind.is_proc_macro())
    }

    /// Itera apenas membros NoSourceTarget.
    pub fn empty_members(&self) -> impl Iterator<Item = &WorkspaceMember> {
        self.members.iter().filter(|m| m.entry_kind.is_empty())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_mock_member(name: &str, kind: EntryKind) -> WorkspaceMember {
        let crate_root = PathBuf::from(format!("/abs/path/{}", name));
        WorkspaceMember {
            name: name.to_string(),
            crate_root,
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
            members: vec![create_mock_member(
                "crate_a",
                EntryKind::Library {
                    lib_path: PathBuf::from("/abs/path/crate_a/src/lib.rs"),
                },
            )],
        };
        assert_eq!(ws_one.member_count(), 1);
        assert_eq!(ws_one.members[0].name, "crate_a");

        // Workspace com 3 membros
        let ws_three = Workspace {
            root: PathBuf::from("/abs/path"),
            members: vec![
                create_mock_member(
                    "crate_a",
                    EntryKind::Library {
                        lib_path: PathBuf::from("/abs/path/crate_a/src/lib.rs"),
                    },
                ),
                create_mock_member(
                    "crate_b",
                    EntryKind::Binary {
                        main_path: PathBuf::from("/abs/path/crate_b/src/main.rs"),
                    },
                ),
                create_mock_member(
                    "crate_c",
                    EntryKind::LibraryAndBinary {
                        lib_path: PathBuf::from("/abs/path/crate_c/src/lib.rs"),
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
                create_mock_member(
                    "crate_a",
                    EntryKind::Library {
                        lib_path: PathBuf::from("/abs/path/crate_a/src/lib.rs"),
                    },
                ),
                create_mock_member(
                    "crate_b",
                    EntryKind::Binary {
                        main_path: PathBuf::from("/abs/path/crate_b/src/main.rs"),
                    },
                ),
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
            members: vec![create_mock_member(
                "crate_a",
                EntryKind::Library {
                    lib_path: PathBuf::from("/abs/path/crate_a/src/lib.rs"),
                },
            )],
        };
        assert_eq!(ws_1.member_count(), 1);

        let ws_3 = Workspace {
            root: PathBuf::from("/abs/path"),
            members: vec![
                create_mock_member(
                    "crate_a",
                    EntryKind::Library {
                        lib_path: PathBuf::from("/abs/path/crate_a/src/lib.rs"),
                    },
                ),
                create_mock_member(
                    "crate_b",
                    EntryKind::Binary {
                        main_path: PathBuf::from("/abs/path/crate_b/src/main.rs"),
                    },
                ),
                create_mock_member(
                    "crate_c",
                    EntryKind::Library {
                        lib_path: PathBuf::from("/abs/path/crate_c/src/lib.rs"),
                    },
                ),
            ],
        };
        assert_eq!(ws_3.member_count(), 3);
    }

    #[test]
    fn test_libraries_and_binaries_filtering() {
        let ws = Workspace {
            root: PathBuf::from("/abs/path"),
            members: vec![
                create_mock_member(
                    "crate_lib",
                    EntryKind::Library {
                        lib_path: PathBuf::from("/abs/path/crate_lib/src/lib.rs"),
                    },
                ),
                create_mock_member(
                    "crate_bin",
                    EntryKind::Binary {
                        main_path: PathBuf::from("/abs/path/crate_bin/src/main.rs"),
                    },
                ),
                create_mock_member(
                    "crate_both",
                    EntryKind::LibraryAndBinary {
                        lib_path: PathBuf::from("/abs/path/crate_both/src/lib.rs"),
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
            members: vec![create_mock_member(
                "crate_a",
                EntryKind::Library {
                    lib_path: PathBuf::from("/abs/path/crate_a/src/lib.rs"),
                },
            )],
        };

        let ws_b = Workspace {
            root: PathBuf::from("/abs/path"),
            members: vec![create_mock_member(
                "crate_a",
                EntryKind::Library {
                    lib_path: PathBuf::from("/abs/path/crate_a/src/lib.rs"),
                },
            )],
        };

        let ws_c = Workspace {
            root: PathBuf::from("/abs/path"),
            members: vec![create_mock_member(
                "crate_b",
                EntryKind::Library {
                    lib_path: PathBuf::from("/abs/path/crate_b/src/lib.rs"),
                },
            )],
        };

        assert_eq!(ws_a, ws_b);
        assert_ne!(ws_a, ws_c);
    }

    // --- Novos testes (ADR-0007) ---

    #[test]
    fn test_proc_macro_member() {
        let member = create_mock_member(
            "my_macros",
            EntryKind::ProcMacro {
                lib_path: PathBuf::from("/abs/path/my_macros/src/lib.rs"),
            },
        );
        assert!(member.entry_kind.is_proc_macro());
        assert!(member.entry_kind.has_main_source());
        assert_eq!(
            member.entry_kind.primary_entry(),
            Some(Path::new("/abs/path/my_macros/src/lib.rs"))
        );
        assert!(!member.entry_kind.is_tests_only());
        assert!(!member.entry_kind.is_empty());
    }

    #[test]
    fn test_tests_only_member() {
        let member = create_mock_member(
            "integration_tests",
            EntryKind::TestsOnly {
                test_paths: vec![
                    PathBuf::from("/abs/path/integration_tests/tests/foo.rs"),
                    PathBuf::from("/abs/path/integration_tests/tests/bar.rs"),
                ],
            },
        );
        assert!(member.entry_kind.is_tests_only());
        assert!(!member.entry_kind.has_main_source());
        assert_eq!(
            member.entry_kind.primary_entry(),
            Some(Path::new("/abs/path/integration_tests/tests/foo.rs"))
        );
        assert!(!member.entry_kind.is_proc_macro());
        assert!(!member.entry_kind.is_empty());
    }

    #[test]
    fn test_no_source_target_member() {
        let member = create_mock_member("config_only", EntryKind::NoSourceTarget);
        assert!(member.entry_kind.is_empty());
        assert!(!member.entry_kind.has_main_source());
        assert!(member.entry_kind.primary_entry().is_none());
        assert!(!member.entry_kind.is_tests_only());
        assert!(!member.entry_kind.is_proc_macro());
    }

    #[test]
    fn test_members_with_code() {
        let ws = Workspace {
            root: PathBuf::from("/abs/path"),
            members: vec![
                create_mock_member(
                    "lib_crate",
                    EntryKind::Library {
                        lib_path: PathBuf::from("/abs/path/lib_crate/src/lib.rs"),
                    },
                ),
                create_mock_member(
                    "test_crate",
                    EntryKind::TestsOnly {
                        test_paths: vec![PathBuf::from("/abs/path/test_crate/tests/foo.rs")],
                    },
                ),
                create_mock_member(
                    "macro_crate",
                    EntryKind::ProcMacro {
                        lib_path: PathBuf::from("/abs/path/macro_crate/src/lib.rs"),
                    },
                ),
                create_mock_member("empty_crate", EntryKind::NoSourceTarget),
            ],
        };

        let with_code: Vec<&WorkspaceMember> = ws.members_with_code().collect();
        assert_eq!(with_code.len(), 2);
        assert!(with_code.iter().any(|m| m.name == "lib_crate"));
        assert!(with_code.iter().any(|m| m.name == "macro_crate"));
    }

    #[test]
    fn test_tests_only_members() {
        let ws = Workspace {
            root: PathBuf::from("/abs/path"),
            members: vec![
                create_mock_member(
                    "lib_crate",
                    EntryKind::Library {
                        lib_path: PathBuf::from("/abs/path/lib_crate/src/lib.rs"),
                    },
                ),
                create_mock_member(
                    "test_crate",
                    EntryKind::TestsOnly {
                        test_paths: vec![PathBuf::from("/abs/path/test_crate/tests/foo.rs")],
                    },
                ),
            ],
        };

        let tests_only: Vec<&WorkspaceMember> = ws.tests_only_members().collect();
        assert_eq!(tests_only.len(), 1);
        assert_eq!(tests_only[0].name, "test_crate");
    }

    #[test]
    fn test_proc_macro_members() {
        let ws = Workspace {
            root: PathBuf::from("/abs/path"),
            members: vec![
                create_mock_member(
                    "lib_crate",
                    EntryKind::Library {
                        lib_path: PathBuf::from("/abs/path/lib_crate/src/lib.rs"),
                    },
                ),
                create_mock_member(
                    "macro_crate",
                    EntryKind::ProcMacro {
                        lib_path: PathBuf::from("/abs/path/macro_crate/src/lib.rs"),
                    },
                ),
            ],
        };

        let proc_macros: Vec<&WorkspaceMember> = ws.proc_macro_members().collect();
        assert_eq!(proc_macros.len(), 1);
        assert_eq!(proc_macros[0].name, "macro_crate");
    }

    #[test]
    fn test_empty_members() {
        let ws = Workspace {
            root: PathBuf::from("/abs/path"),
            members: vec![
                create_mock_member(
                    "lib_crate",
                    EntryKind::Library {
                        lib_path: PathBuf::from("/abs/path/lib_crate/src/lib.rs"),
                    },
                ),
                create_mock_member("empty_crate", EntryKind::NoSourceTarget),
            ],
        };

        let empties: Vec<&WorkspaceMember> = ws.empty_members().collect();
        assert_eq!(empties.len(), 1);
        assert_eq!(empties[0].name, "empty_crate");
    }
}
