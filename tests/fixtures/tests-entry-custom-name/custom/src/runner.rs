// Entry point do target [[test]] com nome não-canónico.
// Declara `mod helper;` — deve resolver para src/helper.rs
// (sibling), não para src/runner/helper.rs.
mod helper;

#[test]
fn dummy() {
    let _ = helper::h();
}
