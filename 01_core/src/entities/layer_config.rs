/*
 * Crystalline Lineage
 * @prompt 00_nucleo/prompts/layer_config_detector.md
 * @layer L1
 * @updated 2026-05-25
 */

use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Layer {
    L0,
    L1,
    L2,
    L3,
    L4,
    Lab,
}

impl Layer {
    /// Retorna `true` se um nó nesta camada pode depender de um
    /// nó na camada `target`, segundo a topologia cristalina.
    pub fn can_depend_on(self, target: Layer) -> bool {
        use Layer::*;
        match self {
            // L0 é prompts/ADRs; não tem código que importa.
            // Tratado como não-restritivo aqui (não deveria
            // aparecer no grafo de imports Rust).
            L0 => true,
            L1 => matches!(target, L1),
            L2 => matches!(target, L1 | L2),
            L3 => matches!(target, L1 | L3),
            L4 => matches!(target, L1 | L2 | L3 | L4),
            // lab pode importar qualquer coisa.
            Lab => true,
        }
    }

    /// Parse a partir do nome usado no `[layers]` do toml.
    /// "L0".."L4" e "lab" (case-insensitive para "lab").
    pub fn from_config_key(key: &str) -> Option<Layer> {
        match key {
            "L0" => Some(Layer::L0),
            "L1" => Some(Layer::L1),
            "L2" => Some(Layer::L2),
            "L3" => Some(Layer::L3),
            "L4" => Some(Layer::L4),
            "lab" | "Lab" | "LAB" => Some(Layer::Lab),
            _ => None,
        }
    }

    /// Nome canónico para exibição.
    pub fn as_str(self) -> &'static str {
        match self {
            Layer::L0 => "L0",
            Layer::L1 => "L1",
            Layer::L2 => "L2",
            Layer::L3 => "L3",
            Layer::L4 => "L4",
            Layer::Lab => "lab",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LayerConfig {
    /// Mapa de crate_name (formato canónico, ex:
    /// "crystalline_dsm_core") para a camada.
    crate_to_layer: HashMap<String, Layer>,
}

impl LayerConfig {
    /// Constrói a partir de um mapa pré-computado.
    pub fn new(crate_to_layer: HashMap<String, Layer>) -> Self {
        Self { crate_to_layer }
    }

    /// Retorna a camada de um crate, se conhecida.
    /// `None` para crates não mapeados.
    pub fn layer_of_crate(&self, crate_name: &str) -> Option<Layer> {
        self.crate_to_layer.get(crate_name).copied()
    }

    /// Quantidade de crates mapeados.
    pub fn len(&self) -> usize {
        self.crate_to_layer.len()
    }

    /// `true` se vazio.
    pub fn is_empty(&self) -> bool {
        self.crate_to_layer.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_layer_can_depend_on_l1() {
        assert!(Layer::L1.can_depend_on(Layer::L1));
        assert!(!Layer::L1.can_depend_on(Layer::L2));
        assert!(!Layer::L1.can_depend_on(Layer::L3));
        assert!(!Layer::L1.can_depend_on(Layer::L4));
        assert!(!Layer::L1.can_depend_on(Layer::Lab));
    }

    #[test]
    fn test_layer_can_depend_on_l2() {
        assert!(Layer::L2.can_depend_on(Layer::L1));
        assert!(Layer::L2.can_depend_on(Layer::L2));
        assert!(!Layer::L2.can_depend_on(Layer::L3));
        assert!(!Layer::L2.can_depend_on(Layer::L4));
        assert!(!Layer::L2.can_depend_on(Layer::Lab));
    }

    #[test]
    fn test_layer_can_depend_on_l3() {
        assert!(Layer::L3.can_depend_on(Layer::L1));
        assert!(!Layer::L3.can_depend_on(Layer::L2));
        assert!(Layer::L3.can_depend_on(Layer::L3));
        assert!(!Layer::L3.can_depend_on(Layer::L4));
        assert!(!Layer::L3.can_depend_on(Layer::Lab));
    }

    #[test]
    fn test_layer_can_depend_on_l4() {
        assert!(Layer::L4.can_depend_on(Layer::L1));
        assert!(Layer::L4.can_depend_on(Layer::L2));
        assert!(Layer::L4.can_depend_on(Layer::L3));
        assert!(Layer::L4.can_depend_on(Layer::L4));
        assert!(!Layer::L4.can_depend_on(Layer::Lab));
    }

    #[test]
    fn test_layer_can_depend_on_lab() {
        assert!(Layer::Lab.can_depend_on(Layer::L1));
        assert!(Layer::Lab.can_depend_on(Layer::L2));
        assert!(Layer::Lab.can_depend_on(Layer::L3));
        assert!(Layer::Lab.can_depend_on(Layer::L4));
        assert!(Layer::Lab.can_depend_on(Layer::Lab));
    }

    #[test]
    fn test_from_config_key() {
        assert_eq!(Layer::from_config_key("L0"), Some(Layer::L0));
        assert_eq!(Layer::from_config_key("L1"), Some(Layer::L1));
        assert_eq!(Layer::from_config_key("L2"), Some(Layer::L2));
        assert_eq!(Layer::from_config_key("L3"), Some(Layer::L3));
        assert_eq!(Layer::from_config_key("L4"), Some(Layer::L4));
        assert_eq!(Layer::from_config_key("lab"), Some(Layer::Lab));
        assert_eq!(Layer::from_config_key("Lab"), Some(Layer::Lab));
        assert_eq!(Layer::from_config_key("LAB"), Some(Layer::Lab));
        assert_eq!(Layer::from_config_key("L5"), None);
        assert_eq!(Layer::from_config_key("xyz"), None);
    }

    #[test]
    fn test_layer_config_layer_of_crate() {
        let mut map = HashMap::new();
        map.insert("crate_a".to_string(), Layer::L1);
        map.insert("crate_b".to_string(), Layer::L2);

        let config = LayerConfig::new(map);
        assert_eq!(config.len(), 2);
        assert!(!config.is_empty());
        assert_eq!(config.layer_of_crate("crate_a"), Some(Layer::L1));
        assert_eq!(config.layer_of_crate("crate_b"), Some(Layer::L2));
        assert_eq!(config.layer_of_crate("crate_c"), None);
    }
}
