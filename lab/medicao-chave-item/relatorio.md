# Medição — discriminância das chaves de identidade de item (par typst)

Arena do prompt 0077. **Números, sem conclusão.**

## Portão de sanidade (deve bater com o laudo 0076)

| Lado | nós pós-filtro | fantasmas | third-party removido |
|---|---|---|---|
| typst-original (antes) | 13392 | 448 | 434 |
| typst-crystalline (depois) | 3026 | 0 | 40 |

0076 esperava: fantasmas 448 / 0 · third-party 434 / 40.

Tempo de montagem dos dois grafos (cache morno): **12.10 s**

## Censo de itens

Itens (kinds de definição, exclui mod/crate/builtin e representantes de fantasma).

- typst-original (antes): **12590** itens · representantes de fantasma excluídos: 431
- typst-crystalline (depois): **2851** itens · representantes de fantasma excluídos: 0

Distribuição por kind:

| kind | antes | depois |
|---|---|---|
| const | 16 | 0 |
| enum | 345 | 71 |
| fn | 10382 | 2500 |
| macro | 19 | 2 |
| static | 7 | 0 |
| struct | 801 | 236 |
| trait | 117 | 8 |
| type | 903 | 34 |

## K1 — (kind, nome) — censo completo

- censo: antes 12590 · depois 2851
- chaves distintas: antes 4187 · depois 1351
- **pareáveis 1:1: 415**
- ambíguas: 232 chaves cobrindo 6991 itens
- sem-par: antes 3540 · depois 704

Top colisões (chave · antes×depois · amostra):

- `fn|fmt` — 796×259 · a: typst_bundle::Bundle::fmt, typst_bundle::BundleDocument::fmt, typst_bundle::BundleFile::fmt · d: typst_core::entities::args::Args::fmt, typst_core::entities::ast::code::BareImportError::fmt, typst_core::entities::ast::code::Conditional::fmt
- `fn|clone` — 734×246 · a: typst_bundle::Bundle::clone, typst_bundle::BundleDocument::clone, typst_bundle::BundleFile::clone · d: typst_core::entities::args::Args::clone, typst_core::entities::ast::code::BareImportError::clone, typst_core::entities::ast::code::Conditional::clone
- `fn|hash` — 608×152 · a: typst_bundle::PagedExtras::hash, typst_html::FrameElem::hash, typst_html::HtmlElem::hash · d: typst_core::entities::ast::code::BareImportError::hash, typst_core::entities::ast::code::Conditional::hash, typst_core::entities::ast::code::Contextual::hash
- `fn|eq` — 463×197 · a: typst_eval::flow::FlowEvent::eq, typst_html::convert::Whitespace::eq, typst_html::dom::HtmlAttr::eq · d: typst_core::entities::args::Args::eq, typst_core::entities::ast::code::BareImportError::eq, typst_core::entities::ast::code::Conditional::eq
- `fn|new` — 354×46 · a: typst_bundle::introspect::BundleIntrospector::new, typst_docs::html::Handler::new, typst_docs::html::Html::new · d: typst_core::entities::axes::Axes::new, typst_core::entities::bib_entry::BibEntry::new, typst_core::entities::corners::Corners::new
- `fn|into_value` — 202×1 · a: typst_html::dom::HtmlAttr::into_value, typst_html::dom::HtmlAttrs::into_value, typst_html::dom::HtmlTag::into_value · d: typst_core::entities::scope::Binding::into_value
- `fn|default` — 141×47 · a: test_wrapper::RegenCommand::default, typst::Library::default, typst_bundle::introspect::BundleIntrospectorBuilder::default · d: typst_core::entities::bib_store::BibStore::default, typst_core::entities::citation_form::CitationForm::default, typst_core::entities::corners::Corners::default
- `fn|from_untyped` — 82×82 · a: typst_syntax::ast::Arg::from_untyped, typst_syntax::ast::Args::from_untyped, typst_syntax::ast::Array::from_untyped · d: typst_core::entities::ast::code::Conditional::from_untyped, typst_core::entities::ast::code::Contextual::from_untyped, typst_core::entities::ast::code::DestructAssignment::from_untyped
- `fn|to_untyped` — 82×82 · a: typst_syntax::ast::Arg::to_untyped, typst_syntax::ast::Args::to_untyped, typst_syntax::ast::Array::to_untyped · d: typst_core::entities::ast::code::Conditional::to_untyped, typst_core::entities::ast::code::Contextual::to_untyped, typst_core::entities::ast::code::DestructAssignment::to_untyped
- `type|Output` — 152×8 · a: typst_eval::ast::Args::Output, typst_eval::ast::Binary::Output, typst_eval::ast::Expr::Output · d: typst_core::entities::layout_types::Abs::<Add>::Output, typst_core::entities::layout_types::Abs::<Neg>::Output, typst_core::entities::layout_types::Length::<Add>::Output

## K2 — (kind, nome) sem folhas de impl-de-trait (trait_/trait_ref)

- censo: antes 5979 · depois 1583
- chaves distintas: antes 4042 · depois 1268
- **pareáveis 1:1: 414**
- ambíguas: 185 chaves cobrindo 1668 itens
- sem-par: antes 3443 · depois 669

Top colisões (chave · antes×depois · amostra):

- `fn|new` — 354×46 · a: typst_bundle::introspect::BundleIntrospector::new, typst_docs::html::Handler::new, typst_docs::html::Html::new · d: typst_core::entities::axes::Axes::new, typst_core::entities::bib_entry::BibEntry::new, typst_core::entities::corners::Corners::new
- `fn|get` — 61×26 · a: typst_html::dom::HtmlAttrs::get, typst_kit::fonts::FontSlot::get, typst_kit::server::Bucket::get · d: typst_core::entities::ast::expr::Bool::get, typst_core::entities::ast::expr::Float::get, typst_core::entities::ast::expr::Ident::get
- `fn|is_empty` — 19×22 · a: typst_library::foundations::args::Args::is_empty, typst_library::foundations::array::Array::is_empty, typst_library::foundations::bytes::Bytes::is_empty · d: typst_core::entities::args::Args::is_empty, typst_core::entities::bib_store::BibStore::is_empty, typst_core::entities::content::Content::is_empty
- `fn|body` — 15×14 · a: typst_library::model::outline::OutlineEntry::body, typst_syntax::ast::Closure::body, typst_syntax::ast::CodeBlock::body · d: typst_core::entities::ast::code::Contextual::body, typst_core::entities::ast::code::ForLoop::body, typst_core::entities::ast::code::FuncReturn::body
- `fn|iter` — 24×5 · a: typst_html::typed::UnionType::iter, typst_layout::inline::line::Items::iter, typst_library::foundations::array::Array::iter · d: typst_core::entities::ast::code::ImportItemPath::iter, typst_core::entities::ast::code::ImportItems::iter, typst_core::entities::label_registry::LabelRegistry::iter
- `fn|len` — 11×18 · a: typst_library::foundations::args::Args::len, typst_library::foundations::array::Array::len, typst_library::foundations::bytes::Bytes::len · d: typst_core::entities::args::Args::len, typst_core::entities::ast::markup::RawDelim::len, typst_core::entities::bib_store::BibStore::len
- `fn|push` — 23×4 · a: typst_html::convert::Converter::push, typst_html::css::Properties::push, typst_html::dom::HtmlAttrs::push · d: typst_core::entities::font_book::FontBook::push, typst_core::entities::layout_types::Frame::push, typst_core::entities::style::Styles::push
- `fn|as_str` — 13×10 · a: typst_docs::html::Html::as_str, typst_library::foundations::bytes::Bytes::as_str, typst_library::foundations::str::Str::as_str · d: typst_core::entities::ast::expr::BinOp::as_str, typst_core::entities::ast::expr::Ident::as_str, typst_core::entities::ast::expr::UnOp::as_str
- `fn|finish` — 19×2 · a: typst_bundle::introspect::BundleIntrospectorBuilder::finish, typst_eval::call::CapturesVisitor::finish, typst_html::convert::Converter::finish · d: typst_core::rules::layout::Layouter::finish, typst_core::rules::parse::parser::Parser::finish
- `fn|contains` — 15×4 · a: typst_library::engine::Route::contains, typst_library::engine::__ComemoSurface::contains, typst_library::engine::__ComemoSurfaceMut::contains · d: typst_core::entities::syntax_set::SyntaxSet::contains, typst_core::entities::world_types::Route::contains, typst_core::entities::world_types::__ComemoSurface::contains

## K3 — (kind, pai-tipo::nome)

- censo: antes 12590 · depois 2851
- chaves distintas: antes 11475 · depois 2747
- **pareáveis 1:1: 1456**
- ambíguas: 120 chaves cobrindo 441 itens
- sem-par: antes 9899 · depois 1171

Top colisões (chave · antes×depois · amostra):

- `fn|__ComemoCall::clone` — 6×4 · a: typst_library::engine::__ComemoCall::clone#1, typst_library::engine::__ComemoCall::clone#2, typst_library::engine::__ComemoCall::clone#3 · d: typst_core::entities::sealed_positions::__ComemoCall::clone, typst_core::entities::world_types::__ComemoCall::clone#1, typst_core::entities::world_types::__ComemoCall::clone#2
- `fn|__ComemoCall::eq` — 6×4 · a: typst_library::engine::__ComemoCall::eq#1, typst_library::engine::__ComemoCall::eq#2, typst_library::engine::__ComemoCall::eq#3 · d: typst_core::entities::sealed_positions::__ComemoCall::eq, typst_core::entities::world_types::__ComemoCall::eq#1, typst_core::entities::world_types::__ComemoCall::eq#2
- `fn|__ComemoCall::hash` — 6×4 · a: typst_library::engine::__ComemoCall::hash#1, typst_library::engine::__ComemoCall::hash#2, typst_library::engine::__ComemoCall::hash#3 · d: typst_core::entities::sealed_positions::__ComemoCall::hash, typst_core::entities::world_types::__ComemoCall::hash#1, typst_core::entities::world_types::__ComemoCall::hash#2
- `struct|__ComemoCall` — 6×4 · a: typst_library::engine::__ComemoCall#1, typst_library::engine::__ComemoCall#2, typst_library::engine::__ComemoCall#3 · d: typst_core::entities::sealed_positions::__ComemoCall, typst_core::entities::world_types::__ComemoCall#1, typst_core::entities::world_types::__ComemoCall#2
- `struct|__ComemoSurface` — 6×4 · a: typst_library::engine::__ComemoSurface#1, typst_library::engine::__ComemoSurface#2, typst_library::engine::__ComemoSurface#3 · d: typst_core::entities::sealed_positions::__ComemoSurface, typst_core::entities::world_types::__ComemoSurface#1, typst_core::entities::world_types::__ComemoSurface#2
- `struct|__ComemoSurfaceMut` — 6×4 · a: typst_library::engine::__ComemoSurfaceMut#1, typst_library::engine::__ComemoSurfaceMut#2, typst_library::engine::__ComemoSurfaceMut#3 · d: typst_core::entities::sealed_positions::__ComemoSurfaceMut, typst_core::entities::world_types::__ComemoSurfaceMut#1, typst_core::entities::world_types::__ComemoSurfaceMut#2
- `type|Abs::Output` — 7×2 · a: typst_library::layout::abs::Abs::<Add>::Output, typst_library::layout::abs::Abs::<Div<Self>>::Output, typst_library::layout::abs::Abs::<Div<f64>>::Output · d: typst_core::entities::layout_types::Abs::<Add>::Output, typst_core::entities::layout_types::Abs::<Neg>::Output
- `type|Length::Output` — 6×2 · a: typst_library::layout::length::Length::<Add>::Output, typst_library::layout::length::Length::<Div>::Output, typst_library::layout::length::Length::<Mul>::Output · d: typst_core::entities::layout_types::Length::<Add>::Output, typst_core::entities::layout_types::Length::<Neg>::Output
- `fn|FontFlags::fmt` — 5×1 · a: typst_library::text::font::book::FontFlags::<Binary>::fmt, typst_library::text::font::book::FontFlags::<Debug>::fmt, typst_library::text::font::book::FontFlags::<LowerHex>::fmt · d: typst_core::entities::font_book::FontFlags::fmt
- `fn|Args::fmt` — 2×3 · a: typst_library::foundations::args::Args::fmt, typst_syntax::ast::Args::fmt · d: typst_core::entities::args::Args::fmt, typst_core::entities::ast::expr::Args::fmt, typst_shell::cli::Args::fmt

## K4 — (kind, trait_, pai-tipo::nome)

- censo: antes 12590 · depois 2851
- chaves distintas: antes 11709 · depois 2764
- **pareáveis 1:1: 1474**
- ambíguas: 107 chaves cobrindo 380 itens
- sem-par: antes 10128 · depois 1183

Top colisões (chave · antes×depois · amostra):

- `fn|Clone|__ComemoCall::clone` — 6×4 · a: typst_library::engine::__ComemoCall::clone#1, typst_library::engine::__ComemoCall::clone#2, typst_library::engine::__ComemoCall::clone#3 · d: typst_core::entities::sealed_positions::__ComemoCall::clone, typst_core::entities::world_types::__ComemoCall::clone#1, typst_core::entities::world_types::__ComemoCall::clone#2
- `fn|Hash|__ComemoCall::hash` — 6×4 · a: typst_library::engine::__ComemoCall::hash#1, typst_library::engine::__ComemoCall::hash#2, typst_library::engine::__ComemoCall::hash#3 · d: typst_core::entities::sealed_positions::__ComemoCall::hash, typst_core::entities::world_types::__ComemoCall::hash#1, typst_core::entities::world_types::__ComemoCall::hash#2
- `fn|PartialEq|__ComemoCall::eq` — 6×4 · a: typst_library::engine::__ComemoCall::eq#1, typst_library::engine::__ComemoCall::eq#2, typst_library::engine::__ComemoCall::eq#3 · d: typst_core::entities::sealed_positions::__ComemoCall::eq, typst_core::entities::world_types::__ComemoCall::eq#1, typst_core::entities::world_types::__ComemoCall::eq#2
- `struct||__ComemoCall` — 6×4 · a: typst_library::engine::__ComemoCall#1, typst_library::engine::__ComemoCall#2, typst_library::engine::__ComemoCall#3 · d: typst_core::entities::sealed_positions::__ComemoCall, typst_core::entities::world_types::__ComemoCall#1, typst_core::entities::world_types::__ComemoCall#2
- `struct||__ComemoSurface` — 6×4 · a: typst_library::engine::__ComemoSurface#1, typst_library::engine::__ComemoSurface#2, typst_library::engine::__ComemoSurface#3 · d: typst_core::entities::sealed_positions::__ComemoSurface, typst_core::entities::world_types::__ComemoSurface#1, typst_core::entities::world_types::__ComemoSurface#2
- `struct||__ComemoSurfaceMut` — 6×4 · a: typst_library::engine::__ComemoSurfaceMut#1, typst_library::engine::__ComemoSurfaceMut#2, typst_library::engine::__ComemoSurfaceMut#3 · d: typst_core::entities::sealed_positions::__ComemoSurfaceMut, typst_core::entities::world_types::__ComemoSurfaceMut#1, typst_core::entities::world_types::__ComemoSurfaceMut#2
- `fn|Debug|Args::fmt` — 2×3 · a: typst_library::foundations::args::Args::fmt, typst_syntax::ast::Args::fmt · d: typst_core::entities::args::Args::fmt, typst_core::entities::ast::expr::Args::fmt, typst_shell::cli::Args::fmt
- `fn|From|Paint::from` — 3×2 · a: typst_library::visualize::paint::Paint::<From<Gradient>>::from, typst_library::visualize::paint::Paint::<From<T>>::from, typst_library::visualize::paint::Paint::<From<Tiling>>::from · d: typst_core::entities::paint::Paint::<From<Color>>::from, typst_core::entities::paint::Paint::<From<Gradient>>::from
- `struct||Args` — 2×3 · a: typst_library::foundations::args::Args, typst_syntax::ast::Args · d: typst_core::entities::args::Args, typst_core::entities::ast::expr::Args, typst_shell::cli::Args
- `fn|Clone|Args::clone` — 2×2 · a: typst_library::foundations::args::Args::clone, typst_syntax::ast::Args::clone · d: typst_core::entities::args::Args::clone, typst_core::entities::ast::expr::Args::clone

## Síntese K1–K4

| Chave | censo a/d | pareáveis 1:1 | ambíguas (chaves/itens) | sem-par a/d |
|---|---|---|---|---|
| K1 | 12590/2851 | **415** | 232/6991 | 3540/704 |
| K2 | 5979/1583 | **414** | 185/1668 | 3443/669 |
| K3 | 12590/2851 | **1456** | 120/441 | 9899/1171 |
| K4 | 12590/2851 | **1474** | 107/380 | 10128/1183 |

_Sem conclusão — a escolha da chave fica com o autor._
