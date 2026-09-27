//! Crystalline Lineage
//! @prompt lab/doxygen-importador/CONTRATO.md
//! @updated 2026-09-26
//! Importação local de XML existente; nenhum extrator ou código-fonte é executado.
#[cfg(not(target_os = "linux"))]
compile_error!("este experimento suporta somente Linux");
use roxmltree::{Document, Node, ParsingOptions};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    os::{
        fd::{AsRawFd, FromRawFd},
        unix::fs::OpenOptionsExt,
    },
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};
const FILE_LIMIT: usize = 4 * 1024 * 1024;
const TOTAL_LIMIT: usize = 32 * 1024 * 1024;
const NODE_LIMIT: u32 = 100_000;
const FIELD_LIMIT: usize = 64 * 1024;
const OUTPUT_LIMIT: usize = 16 * 1024 * 1024;
static NEXT: AtomicU64 = AtomicU64::new(0);
#[derive(Debug, Serialize)]
struct Fatal {
    code: &'static str,
    message: String,
}
type Result<T> = std::result::Result<T, Fatal>;
fn fail(code: &'static str, message: impl Into<String>) -> Fatal {
    Fatal {
        code,
        message: message.into(),
    }
}
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Diagnostic {
    code: String,
    message: String,
    symbol_id: Option<String>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Producer {
    name: String,
    version: Option<String>,
    configuration_digest: Option<String>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Location {
    file: String,
    line: u32,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Symbol {
    id: String,
    kind: String,
    name: String,
    qualified_name: Option<String>,
    signature: Option<String>,
    location: Option<Location>,
    brief: String,
    details: String,
    documentation_url: Option<String>,
}
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Relation {
    from: String,
    to: String,
    kind: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct PendingRelation {
    from: String,
    to: String,
    kind: String,
    reason: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Inventory {
    schema: String,
    version: u32,
    producer: Producer,
    coverage: String,
    symbols: Vec<Symbol>,
    relations: Vec<Relation>,
    pending_relations: Vec<PendingRelation>,
    diagnostics: Vec<Diagnostic>,
}
struct Root {
    dir: File,
    total: usize,
}
impl Root {
    fn open(path: &Path) -> Result<Self> {
        let canonical = path
            .canonicalize()
            .map_err(|e| fail("input_directory", e.to_string()))?;
        let dir = OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW)
            .open(canonical)
            .map_err(|e| fail("input_directory", e.to_string()))?;
        Ok(Self { dir, total: 0 })
    }
    fn read(&mut self, name: &str) -> Result<Option<String>> {
        if name.is_empty() || name == "." || name == ".." || name.contains(['/', '\\', '\0']) {
            return Err(fail(
                "path_forbidden",
                format!("nome de arquivo não permitido: {name}"),
            ));
        }
        let c = std::ffi::CString::new(name).map_err(|_| fail("path_forbidden", "NUL em nome"))?;
        // SAFETY: live directory fd and NUL-terminated CString. No pointers retained.
        let fd = unsafe {
            libc::openat(
                self.dir.as_raw_fd(),
                c.as_ptr(),
                libc::O_RDONLY | libc::O_CLOEXEC | libc::O_NOFOLLOW | libc::O_NONBLOCK,
            )
        };
        if fd < 0 {
            let e = std::io::Error::last_os_error();
            if e.kind() == std::io::ErrorKind::NotFound {
                return Ok(None);
            }
            return Err(fail("input_read_forbidden", format!("{name}: {e}")));
        }
        // SAFETY: fd was freshly opened and ownership is transferred exactly once.
        let file = unsafe { File::from_raw_fd(fd) };
        let info = file
            .metadata()
            .map_err(|e| fail("input_read", e.to_string()))?;
        if !info.is_file() {
            return Err(fail("input_not_regular", name));
        }
        if info.len() > FILE_LIMIT as u64 {
            return Err(fail(
                "resource_limit",
                format!("{name}: arquivo excede 4 MiB"),
            ));
        }
        let mut bytes = Vec::new();
        file.take(FILE_LIMIT as u64 + 1)
            .read_to_end(&mut bytes)
            .map_err(|e| fail("input_read", e.to_string()))?;
        self.total += bytes.len();
        if bytes.len() > FILE_LIMIT || self.total > TOTAL_LIMIT {
            return Err(fail("resource_limit", "limite de bytes XML excedido"));
        }
        String::from_utf8(bytes)
            .map(Some)
            .map_err(|_| fail("invalid_xml", "XML deve ser UTF-8"))
    }
}
fn parse<'a>(text: &'a str, root: &str, file: &str) -> Result<Document<'a>> {
    if text.contains("<!DOCTYPE") || text.contains("<!ENTITY") {
        return Err(fail(
            "dtd_forbidden",
            format!("{file}: DTD/entidades proibidos"),
        ));
    }
    let doc = Document::parse_with_options(
        text,
        ParsingOptions {
            allow_dtd: false,
            nodes_limit: NODE_LIMIT,
            entity_resolver: None,
        },
    )
    .map_err(|e| {
        let code = match e {
            roxmltree::Error::NodesLimitReached
            | roxmltree::Error::AttributesLimitReached
            | roxmltree::Error::NamespacesLimitReached => "resource_limit",
            roxmltree::Error::DtdDetected => "dtd_forbidden",
            _ => "invalid_xml",
        };
        fail(code, format!("{file}: {e}"))
    })?;
    if doc.root_element().tag_name().name() != root {
        return Err(fail(
            "incompatible_xml_schema",
            format!("{file}: raiz esperada {root}"),
        ));
    }
    for n in doc.descendants().filter(|n| n.is_element()) {
        for attr in n.attributes() {
            let max = if matches!(attr.name(), "id" | "refid") {
                512
            } else {
                FIELD_LIMIT
            };
            if attr.value().len() > max {
                return Err(fail("resource_limit", "atributo XML excede limite"));
            }
        }
        if n.tag_name().namespace().is_some() {
            return Err(fail(
                "incompatible_xml_schema",
                format!("{file}: namespace de elemento não suportado"),
            ));
        }
        if n.ancestors().take(130).count() > 129 {
            return Err(fail(
                "resource_limit",
                format!("{file}: profundidade XML excede 128"),
            ));
        }
    }
    if let Some(v) = doc.root_element().attribute("version")
        && v != "1.9.8"
    {
        return Err(fail(
            "incompatible_xml_version",
            format!("{file}: versão {v}; perfil suportado 1.9.8"),
        ));
    }
    Ok(doc)
}
fn child<'a, 'b>(n: Node<'a, 'b>, tag: &str) -> Option<Node<'a, 'b>> {
    n.children().find(|c| c.has_tag_name(tag))
}
fn raw(n: Node<'_, '_>) -> Result<String> {
    let value: String = n
        .descendants()
        .filter(|c| c.is_text())
        .filter_map(|c| c.text())
        .collect();
    if value.len() > FIELD_LIMIT {
        return Err(fail("resource_limit", "campo XML excede 64 KiB"));
    }
    Ok(value)
}
fn field(n: Node<'_, '_>, tag: &str) -> Result<Option<String>> {
    let mut values = n.children().filter(|c| c.has_tag_name(tag));
    let first = values.next();
    if values.next().is_some() {
        return Err(fail("invalid_document", format!("campo {tag} repetido")));
    }
    first.map(raw).transpose()
}
fn required(n: Node<'_, '_>, attr: &str) -> Result<String> {
    n.attribute(attr)
        .filter(|s| !s.is_empty())
        .map(str::to_owned)
        .ok_or_else(|| {
            fail(
                "invalid_document",
                format!("{} sem atributo {attr}", n.tag_name().name()),
            )
        })
}
#[derive(Default)]
struct Acc {
    symbols: BTreeMap<String, Symbol>,
    relations: BTreeSet<Relation>,
    diagnostics: BTreeSet<Diagnostic>,
    refs: BTreeSet<(String, Option<String>)>,
    partial: bool,
    definitions: usize,
}
impl Acc {
    fn diag(
        &mut self,
        code: &str,
        message: impl Into<String>,
        id: Option<&str>,
        loss: bool,
    ) -> Result<()> {
        self.partial |= loss;
        self.diagnostics.insert(Diagnostic {
            code: code.into(),
            message: message.into(),
            symbol_id: id.map(str::to_owned),
        });
        if self.diagnostics.len() > 20_000 {
            return Err(fail("resource_limit", "mais de 20000 diagnósticos"));
        }
        Ok(())
    }
    fn insert(&mut self, s: Symbol) -> Result<()> {
        self.definitions += 1;
        if self.definitions > 50_000 {
            return Err(fail("resource_limit", "mais de 50000 definições"));
        }
        if self.symbols.contains_key(&s.id) {
            return Err(fail(
                "duplicate_definition",
                format!("ID repetido: {}", s.id),
            ));
        }
        self.symbols.insert(s.id.clone(), s);
        Ok(())
    }
}
fn text(n: Node<'_, '_>, acc: &mut Acc, id: &str) -> Result<String> {
    fn render(n: Node<'_, '_>, out: &mut String, acc: &mut Acc, id: &str) -> Result<()> {
        if n.is_text() {
            out.push_str(n.text().unwrap_or(""));
        } else if n.is_element() {
            let tag = n.tag_name().name();
            if matches!(
                tag,
                "htmlonly"
                    | "script"
                    | "style"
                    | "iframe"
                    | "object"
                    | "svg"
                    | "dot"
                    | "dotfile"
                    | "msc"
                    | "mscfile"
                    | "plantuml"
                    | "image"
                    | "latexonly"
            ) {
                acc.diag(
                    "unsupported_element",
                    format!("Marcação documental {tag} omitida."),
                    Some(id),
                    true,
                )?;
                return Ok(());
            }
            let block = matches!(
                tag,
                "para"
                    | "title"
                    | "listitem"
                    | "simplesect"
                    | "parameteritem"
                    | "parameterdescription"
                    | "varlistentry"
                    | "term"
                    | "row"
                    | "codeline"
                    | "sect1"
                    | "sect2"
                    | "sect3"
                    | "sect4"
            );
            if block && !out.ends_with("- ") {
                out.push('\n');
            }
            if tag == "listitem" {
                out.push_str("- ");
            }
            match tag {
                "sp" => out.push(' '),
                "linebreak" => out.push('\n'),
                "briefdescription"
                | "detaileddescription"
                | "inbodydescription"
                | "para"
                | "title"
                | "bold"
                | "emphasis"
                | "computeroutput"
                | "ref"
                | "ulink"
                | "itemizedlist"
                | "orderedlist"
                | "listitem"
                | "simplesect"
                | "parameterlist"
                | "parameteritem"
                | "parameternamelist"
                | "parametername"
                | "parameterdescription"
                | "variablelist"
                | "varlistentry"
                | "term"
                | "table"
                | "row"
                | "entry"
                | "programlisting"
                | "codeline"
                | "highlight"
                | "verbatim"
                | "sect1"
                | "sect2"
                | "sect3"
                | "sect4"
                | "superscript"
                | "subscript"
                | "anchor"
                | "internal" => (),
                _ => acc.diag(
                    "unsupported_element",
                    format!("Marcação {tag}: somente texto preservado."),
                    Some(id),
                    true,
                )?,
            }
            for c in n.children() {
                render(c, out, acc, id)?;
            }
            if block || tag == "entry" {
                out.push('\n');
            }
        }
        if out.len() > FIELD_LIMIT {
            return Err(fail("resource_limit", "documentação excede 64 KiB"));
        }
        Ok(())
    }
    let mut output = String::new();
    render(n, &mut output, acc, id)?;
    // Only documentation whitespace is formatted. Names/signatures/locations use raw().
    Ok(output
        .lines()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("\n"))
}
fn symbol(n: Node<'_, '_>, acc: &mut Acc) -> Result<Symbol> {
    let id = required(n, "id")?;
    for tag in [
        "compoundname",
        "name",
        "qualifiedname",
        "argsstring",
        "location",
        "briefdescription",
        "detaileddescription",
        "inbodydescription",
    ] {
        if n.children().filter(|c| c.has_tag_name(tag)).count() > 1 {
            return Err(fail(
                "invalid_document",
                format!("{id}: campo {tag} repetido"),
            ));
        }
    }
    let kind = required(n, "kind")?;
    if !matches!(
        kind.as_str(),
        "class"
            | "struct"
            | "union"
            | "interface"
            | "protocol"
            | "category"
            | "exception"
            | "file"
            | "namespace"
            | "group"
            | "page"
            | "example"
            | "dir"
            | "type"
            | "concept"
            | "module"
            | "function"
            | "variable"
            | "typedef"
            | "enum"
            | "define"
            | "property"
            | "event"
            | "signal"
            | "prototype"
            | "friend"
            | "dcop"
            | "slot"
    ) {
        acc.diag(
            "unsupported_kind",
            "Classificação não reconhecida neste perfil; texto original preservado.",
            Some(&id),
            true,
        )?;
    }
    let name = field(
        n,
        if n.has_tag_name("compounddef") {
            "compoundname"
        } else {
            "name"
        },
    )?
    .ok_or_else(|| fail("invalid_document", format!("{id}: nome ausente")))?;
    let location = if let Some(l) = child(n, "location") {
        match (
            l.attribute("file"),
            l.attribute("line")
                .and_then(|s| s.parse::<u32>().ok())
                .filter(|&v| v > 0),
        ) {
            (Some(file), Some(line)) if !file.is_empty() => Some(Location {
                file: file.into(),
                line,
            }),
            _ => {
                acc.diag(
                    "incomplete_location",
                    "Localização sem file/line válido; não inferida de body/decl.",
                    Some(&id),
                    true,
                )?;
                None
            }
        }
    } else {
        None
    };
    let brief = child(n, "briefdescription")
        .map(|c| text(c, acc, &id))
        .transpose()?
        .unwrap_or_default();
    let mut details = child(n, "detaileddescription")
        .map(|c| text(c, acc, &id))
        .transpose()?
        .unwrap_or_default();
    if let Some(body) = child(n, "inbodydescription") {
        let body = text(body, acc, &id)?;
        if !details.is_empty() && !body.is_empty() {
            details.push('\n');
        }
        details.push_str(&body);
    }
    if details.len() > FIELD_LIMIT {
        return Err(fail("resource_limit", "details excede 64 KiB"));
    }
    Ok(Symbol {
        id,
        kind,
        name,
        qualified_name: field(n, "qualifiedname")?,
        signature: field(n, "argsstring")?,
        location,
        brief,
        details,
        documentation_url: None,
    })
}
fn relation_tag(tag: &str) -> bool {
    matches!(
        tag,
        "basecompoundref"
            | "derivedcompoundref"
            | "includes"
            | "includedby"
            | "innerclass"
            | "innernamespace"
            | "innerfile"
            | "innerdir"
            | "innerpage"
            | "innergroup"
            | "innerconcept"
            | "innermodule"
            | "references"
            | "referencedby"
            | "reimplements"
            | "reimplementedby"
            | "ref"
    )
}
fn scan_symbol(n: Node<'_, '_>, acc: &mut Acc) -> Result<()> {
    let s = symbol(n, acc)?;
    let id = s.id.clone();
    acc.insert(s)?;
    for e in n
        .descendants()
        .filter(|c| c.is_element() && c.id() != n.id())
    {
        // Definitions nested in compound/section are handled separately, not attributed twice.
        if e.ancestors()
            .skip(1)
            .find(|a| a.has_tag_name("compounddef") || a.has_tag_name("memberdef"))
            .map(|a| a.id())
            != Some(n.id())
        {
            continue;
        }
        let tag = e.tag_name().name();
        if e.ancestors()
            .skip(1)
            .take_while(|a| a.id() != n.id())
            .any(|a| {
                matches!(
                    a.tag_name().name(),
                    "enumvalue"
                        | "htmlonly"
                        | "script"
                        | "style"
                        | "iframe"
                        | "object"
                        | "svg"
                        | "dot"
                        | "dotfile"
                        | "msc"
                        | "mscfile"
                        | "plantuml"
                        | "inheritancegraph"
                        | "collaborationgraph"
                        | "incdepgraph"
                        | "invincdepgraph"
                        | "programlisting"
                )
            })
        {
            continue;
        }
        if relation_tag(tag) {
            if e.attribute("external").is_some_and(|s| !s.is_empty()) {
                acc.diag(
                    "external_reference",
                    "Referência externa não convertida em relação local.",
                    Some(&id),
                    true,
                )?;
                continue;
            }
            if let Some(target) = e.attribute("refid").filter(|s| !s.is_empty()) {
                acc.relations.insert(Relation {
                    from: id.clone(),
                    to: target.into(),
                    kind: tag.into(),
                });
                if acc.relations.len() > 100_000 {
                    return Err(fail("resource_limit", "mais de 100000 relações"));
                }
            } else {
                acc.diag(
                    "reference_without_id",
                    format!("{tag} sem refid; destino não inventado."),
                    Some(&id),
                    true,
                )?;
            }
        }
        if tag == "member"
            && let Some(target) = e.attribute("refid")
        {
            acc.refs.insert((target.into(), Some(id.clone())));
        }
        let direct = e
            .parent()
            .is_some_and(|p| p.id() == n.id() || p.has_tag_name("sectiondef"));
        if direct
            && !relation_tag(tag)
            && !matches!(
                tag,
                "compoundname"
                    | "name"
                    | "qualifiedname"
                    | "argsstring"
                    | "definition"
                    | "type"
                    | "location"
                    | "briefdescription"
                    | "detaileddescription"
                    | "inbodydescription"
                    | "sectiondef"
                    | "memberdef"
                    | "member"
                    | "listofallmembers"
                    | "param"
                    | "qualifier"
                    | "templateparamlist"
                    | "title"
                    | "header"
                    | "read"
                    | "write"
                    | "bitfield"
                    | "initializer"
                    | "exceptions"
                    | "requiresclause"
            )
        {
            acc.diag(
                "unsupported_element",
                format!("Elemento {tag} fora do perfil de símbolos."),
                Some(&id),
                true,
            )?;
        }
        if matches!(tag, "param" | "templateparamlist")
            && e.descendants().any(|c| c.has_tag_name("briefdescription"))
        {
            acc.diag(
                "unsupported_element",
                "Documentação de parâmetro fora de brief/details não exportada.",
                Some(&id),
                true,
            )?;
        }
    }
    Ok(())
}
fn import(xml_dir: &Path) -> Result<Inventory> {
    let mut root = Root::open(xml_dir)?;
    let index = root
        .read("index.xml")?
        .ok_or_else(|| fail("index_missing", "index.xml ausente"))?;
    let doc = parse(&index, "doxygenindex", "index.xml")?;
    let version = doc.root_element().attribute("version").map(str::to_owned);
    let mut acc = Acc::default();
    acc.diag(
        "scope_unknown",
        "XML não declara escopo suficiente para afirmar cobertura completa.",
        None,
        false,
    )?;
    if version.is_none() {
        acc.diag(
            "producer_version_unknown",
            "Índice sem versão; versão do produtor permanece desconhecida.",
            None,
            false,
        )?;
    }
    let mut files = BTreeSet::new();
    let mut expected = BTreeMap::<String, BTreeSet<(String, String)>>::new();
    for compound in doc.root_element().children().filter(|n| n.is_element()) {
        if !compound.has_tag_name("compound") {
            acc.diag(
                "unsupported_element",
                format!(
                    "Elemento {} no índice ignorado.",
                    compound.tag_name().name()
                ),
                None,
                true,
            )?;
            continue;
        }
        let id = required(compound, "refid")?;
        if !files.insert(id.clone()) {
            acc.diag(
                "duplicate_index_compound",
                "Compound repetido no índice; arquivo lido uma vez.",
                Some(&id),
                true,
            )?;
        }
        if files.len() > 1024 {
            return Err(fail("resource_limit", "mais de 1024 compounds"));
        }
        for entry in std::iter::once(compound)
            .chain(compound.children().filter(|n| n.has_tag_name("member")))
        {
            let eid = required(entry, "refid")?;
            let kind = required(entry, "kind")?;
            let name = field(entry, "name")?
                .ok_or_else(|| fail("invalid_document", format!("índice {eid}: name ausente")))?;
            expected
                .entry(eid.clone())
                .or_default()
                .insert((kind, name));
            acc.refs.insert((eid, None));
        }
        for entry in compound
            .children()
            .filter(|n| n.is_element() && !matches!(n.tag_name().name(), "name" | "member"))
        {
            acc.diag(
                "unsupported_element",
                format!(
                    "Elemento {} em compound do índice ignorado.",
                    entry.tag_name().name()
                ),
                Some(&id),
                true,
            )?;
        }
    }
    for id in files {
        // refid is used only as a simple filename. Never follow links from documentation.
        if id.contains(['/', '\\', '\0']) || id == "." || id == ".." {
            return Err(fail(
                "path_forbidden",
                format!("refid de compound não permitido: {id}"),
            ));
        }
        let filename = format!("{id}.xml");
        let Some(text) = root.read(&filename)? else {
            acc.diag(
                "referenced_file_missing",
                format!("Arquivo referenciado ausente: {filename}"),
                Some(&id),
                true,
            )?;
            continue;
        };
        let compound_doc = parse(&text, "doxygen", &filename)?;
        if compound_doc.root_element().attribute("version").is_none() {
            acc.diag(
                "producer_version_unknown",
                format!("{filename}: versão XML não declarada."),
                Some(&id),
                false,
            )?;
        }
        let mut found = false;
        for n in compound_doc
            .root_element()
            .children()
            .filter(|n| n.is_element())
        {
            if !n.has_tag_name("compounddef") {
                acc.diag(
                    "unsupported_element",
                    format!(
                        "Elemento {} na raiz de {filename} ignorado.",
                        n.tag_name().name()
                    ),
                    Some(&id),
                    true,
                )?;
                continue;
            }
            if n.attribute("id") == Some(id.as_str()) {
                found = true;
            }
            scan_symbol(n, &mut acc)?;
            for member in n
                .children()
                .filter(|n| n.has_tag_name("sectiondef"))
                .flat_map(|n| n.children())
                .filter(|n| n.has_tag_name("memberdef"))
            {
                scan_symbol(member, &mut acc)?;
            }
        }
        if !found {
            acc.diag(
                "compound_id_mismatch",
                format!("{filename} não contém compounddef com o ID do índice."),
                Some(&id),
                true,
            )?;
        }
    }
    for (id, source) in acc.refs.clone() {
        if !acc.symbols.contains_key(&id) {
            acc.diag(
                "unresolved_index_reference",
                format!("Definição não encontrada para {id}."),
                source.as_deref().or(Some(&id)),
                true,
            )?;
        }
    }
    for (id, entries) in expected {
        if let Some(symbol) = acc.symbols.get(&id)
            && entries
                .iter()
                .any(|(kind, name)| kind != &symbol.kind || name != &symbol.name)
        {
            acc.diag(
                "index_definition_mismatch",
                "kind/name do índice diverge da definição; definição preservada.",
                Some(&id),
                true,
            )?;
        }
    }
    let mut pending_relations = Vec::new();
    for r in acc.relations.clone() {
        if !acc.symbols.contains_key(&r.from) || !acc.symbols.contains_key(&r.to) {
            let reason = match (
                acc.symbols.contains_key(&r.from),
                acc.symbols.contains_key(&r.to),
            ) {
                (false, false) => "missing_both",
                (false, true) => "missing_from",
                _ => "missing_to",
            };
            pending_relations.push(PendingRelation {
                from: r.from.clone(),
                to: r.to.clone(),
                kind: r.kind.clone(),
                reason: reason.into(),
            });
            acc.relations.remove(&r);
            acc.diag(
                "unresolved_reference",
                format!(
                    "{}: {} -> {} sem definição local inequívoca.",
                    r.kind, r.from, r.to
                ),
                Some(&r.from),
                true,
            )?;
        }
    }
    Ok(Inventory {
        schema: "lente.doxygen.inventory".into(),
        version: 2,
        producer: Producer {
            name: "doxygen".into(),
            version,
            configuration_digest: None,
        },
        coverage: if acc.partial { "partial" } else { "unknown" }.into(),
        symbols: acc.symbols.into_values().collect(),
        relations: acc.relations.into_iter().collect(),
        pending_relations,
        diagnostics: acc.diagnostics.into_iter().collect(),
    })
}
struct Temp(PathBuf);
impl Drop for Temp {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.0);
    }
}
fn publish(output: &Path, inventory: &Inventory) -> Result<()> {
    let mut bytes =
        serde_json::to_vec_pretty(inventory).map_err(|e| fail("output_json", e.to_string()))?;
    bytes.push(b'\n');
    if bytes.len() > OUTPUT_LIMIT {
        return Err(fail("resource_limit", "JSON final excede 16 MiB"));
    }
    let check: Inventory =
        serde_json::from_slice(&bytes).map_err(|e| fail("output_json", e.to_string()))?;
    if check.schema != "lente.doxygen.inventory"
        || check.version != 2
        || !matches!(check.coverage.as_str(), "unknown" | "partial")
    {
        return Err(fail("output_schema", "inventário incompatível"));
    }
    let parent = output
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let mut slot = None;
    for _ in 0..100 {
        let path = parent.join(format!(
            ".doxygen-inventory-{}-{}.tmp",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        match OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&path)
        {
            Ok(file) => {
                slot = Some((Temp(path), file));
                break;
            }
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(e) => return Err(fail("output_write", e.to_string())),
        }
    }
    let (temp, mut file) =
        slot.ok_or_else(|| fail("output_write", "não foi possível reservar temp exclusivo"))?;
    file.write_all(&bytes)
        .and_then(|_| file.sync_all())
        .map_err(|e| fail("output_write", e.to_string()))?;
    fs::hard_link(&temp.0, output).map_err(|e| {
        fail(
            "output_publish",
            format!("publicação sem substituição: {e}"),
        )
    })?;
    Ok(())
}
fn run(args: &[String]) -> Result<u8> {
    if args == ["--help"] {
        println!(
            "lente-doxygen-importador --xml-dir DIRETORIO --output INVENTARIO.json\nNão executa Doxygen; saída existente nunca é substituída. Linux."
        );
        return Ok(0);
    }
    let mut dir = None;
    let mut output = None;
    let mut it = args.iter();
    while let Some(flag) = it.next() {
        match flag.as_str() {
            "--xml-dir" if dir.is_none() => {
                dir = Some(
                    it.next()
                        .ok_or_else(|| fail("arguments", "--xml-dir exige valor"))?,
                )
            }
            "--output" if output.is_none() => {
                output = Some(
                    it.next()
                        .ok_or_else(|| fail("arguments", "--output exige valor"))?,
                )
            }
            _ => {
                return Err(fail(
                    "arguments",
                    "opção desconhecida ou repetida; consulte --help",
                ));
            }
        }
    }
    let dir = dir.ok_or_else(|| fail("arguments", "--xml-dir obrigatório"))?;
    let output = output.ok_or_else(|| fail("arguments", "--output obrigatório"))?;
    let inventory = import(Path::new(dir))?;
    publish(Path::new(output), &inventory)?;
    Ok(if inventory.coverage == "partial" {
        2
    } else {
        0
    })
}
fn main() {
    match run(&std::env::args().skip(1).collect::<Vec<_>>()) {
        Ok(code) => std::process::exit(code.into()),
        Err(e) => {
            eprintln!("{}", serde_json::json!({"error":e}));
            std::process::exit(1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    struct Fixture(PathBuf);
    impl Fixture {
        fn new() -> Self {
            let base = Path::new(env!("CARGO_MANIFEST_DIR")).join("target/test-tmp");
            fs::create_dir_all(&base).unwrap();
            let path = base.join(format!(
                "{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir(&path).unwrap();
            let f = Self(path);
            f.write("index.xml", include_str!("../fixtures/basico/index.xml"));
            f.write(
                "namespacedemo.xml",
                include_str!("../fixtures/basico/namespacedemo.xml"),
            );
            f.write(
                "classWidget.xml",
                include_str!("../fixtures/basico/classWidget.xml"),
            );
            f
        }
        fn write(&self, name: &str, text: &str) {
            fs::write(self.0.join(name), text).unwrap();
        }
        fn change(&self, name: &str, from: &str, to: &str) {
            let p = self.0.join(name);
            let text = fs::read_to_string(&p).unwrap();
            assert!(text.contains(from));
            fs::write(p, text.replace(from, to)).unwrap();
        }
        fn import(&self) -> Inventory {
            import(&self.0).unwrap()
        }
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }
    fn codes(i: &Inventory) -> BTreeSet<&str> {
        i.diagnostics.iter().map(|d| d.code.as_str()).collect()
    }
    fn get<'a>(i: &'a Inventory, id: &str) -> &'a Symbol {
        i.symbols.iter().find(|s| s.id == id).unwrap()
    }
    #[test]
    fn functions_classes_namespaces_overloads_and_unicode() {
        let f = Fixture::new();
        let i = f.import();
        assert_eq!(i.symbols.len(), 4);
        assert_eq!(i.schema, "lente.doxygen.inventory");
        assert_eq!(i.version, 2);
        assert_eq!(i.producer.version.as_deref(), Some("1.9.8"));
        assert!(i.producer.configuration_digest.is_none());
        assert_eq!(get(&i, "classWidget").kind, "class");
        assert_eq!(get(&i, "namespacedemo").kind, "namespace");
        let a = get(&i, "namespacedemo_1run_int");
        let b = get(&i, "namespacedemo_1run_double");
        assert_eq!(a.name, b.name);
        assert_ne!(a.id, b.id);
        assert_eq!(a.signature.as_deref(), Some("(int valor)"));
        assert_eq!(b.signature.as_deref(), Some("(double valor)"));
        assert_eq!(a.qualified_name.as_deref(), Some("demo::run"));
        assert_eq!(a.brief, "Executa a ação λ.");
        assert!(a.details.contains("<T> & preserva Unicode."));
        assert!(a.details.contains("- Primeiro passo."));
        assert_eq!(a.location.as_ref().unwrap().line, 10);
        assert_eq!(a.location.as_ref().unwrap().file, "src/demo.cpp");
        assert!(a.documentation_url.is_none());
        assert_eq!(i.coverage, "unknown");
        assert_eq!(codes(&i), BTreeSet::from(["scope_unknown"]));
    }
    #[test]
    fn missing_optional_data_never_becomes_invented_facts() {
        let f = Fixture::new();
        f.change(
            "classWidget.xml",
            "    <location file=\"include/widget.hpp\" line=\"3\"/>",
            "",
        );
        f.change("index.xml", " version=\"1.9.8\"", "");
        let i = f.import();
        let quiet = get(&i, "namespacedemo_1run_double");
        assert!(quiet.brief.is_empty());
        assert!(quiet.details.is_empty());
        let class = get(&i, "classWidget");
        assert!(class.qualified_name.is_none());
        assert!(class.signature.is_none());
        assert!(class.location.is_none());
        assert!(i.producer.version.is_none());
        assert_eq!(i.coverage, "unknown");
        assert!(codes(&i).contains("producer_version_unknown"));
    }
    #[test]
    fn relations_keep_original_kinds_and_unresolved_targets() {
        let f = Fixture::new();
        f.change(
            "classWidget.xml",
            "<location file=",
            "<basecompoundref refid=\"missing_base\">Base</basecompoundref><location file=",
        );
        let i = f.import();
        assert_eq!(i.coverage, "partial");
        assert!(i.relations.contains(&Relation {
            from: "namespacedemo".into(),
            to: "classWidget".into(),
            kind: "innerclass".into()
        }));
        assert!(i.relations.iter().any(|r| r.kind == "references"));
        assert!(i.relations.iter().any(|r| r.kind == "ref"));
        assert!(
            i.pending_relations
                .iter()
                .any(|r| r.to == "missing_base" && r.kind == "basecompoundref")
        );
        assert!(!i.symbols.iter().any(|s| s.id == "missing_base"));
        assert!(codes(&i).contains("unresolved_reference"));
        assert!(
            !i.relations
                .iter()
                .any(|r| r.kind == "uses" || r.kind == "calls")
        );
    }
    #[test]
    fn missing_compound_is_partial_while_missing_index_is_fatal() {
        let f = Fixture::new();
        fs::remove_file(f.0.join("classWidget.xml")).unwrap();
        let i = f.import();
        assert_eq!(i.coverage, "partial");
        assert_eq!(i.symbols.len(), 3);
        assert!(codes(&i).contains("referenced_file_missing"));
        assert!(codes(&i).contains("unresolved_index_reference"));
        fs::remove_file(f.0.join("index.xml")).unwrap();
        assert_eq!(import(&f.0).unwrap_err().code, "index_missing");
    }
    #[test]
    fn malformed_xml_is_fatal_and_preserves_existing_output() {
        let f = Fixture::new();
        let out = f.0.join("inventory.json");
        let i = f.import();
        publish(&out, &i).unwrap();
        let original = fs::read(&out).unwrap();
        f.write(
            "classWidget.xml",
            "<doxygen version=\"1.9.8\"><broken></doxygen>",
        );
        let args = vec![
            "--xml-dir".into(),
            f.0.display().to_string(),
            "--output".into(),
            out.display().to_string(),
        ];
        assert_eq!(run(&args).unwrap_err().code, "invalid_xml");
        assert_eq!(fs::read(&out).unwrap(), original);
        let absent = f.0.join("not-created.json");
        let mut args = args;
        args[3] = absent.display().to_string();
        assert!(run(&args).is_err());
        assert!(!absent.exists());
    }
    #[test]
    fn incompatible_root_version_and_namespace_are_clear_errors() {
        let f = Fixture::new();
        f.change("index.xml", "version=\"1.9.8\"", "version=\"9.0\"");
        assert_eq!(import(&f.0).unwrap_err().code, "incompatible_xml_version");
        f.write("index.xml", "<wrong/>");
        assert_eq!(import(&f.0).unwrap_err().code, "incompatible_xml_schema");
        f.write("index.xml", "<doxygenindex xmlns=\"urn:future\"/>");
        assert_eq!(import(&f.0).unwrap_err().code, "incompatible_xml_schema");
    }
    #[test]
    fn unsupported_markup_is_plain_text_or_omitted_with_diagnostics() {
        let f = Fixture::new();
        f.change("classWidget.xml","Objeto documentado.","Objeto <emphasis>legível</emphasis><linebreak/><custom>extra</custom><htmlonly>&lt;script&gt;bad()&lt;/script&gt;</htmlonly><script>also_bad()</script>");
        let i = f.import();
        let brief = &get(&i, "classWidget").brief;
        assert!(brief.contains("Objeto legível"));
        assert!(brief.contains("extra"));
        assert!(!brief.contains("script"));
        assert!(!brief.contains("bad()"));
        assert_eq!(i.coverage, "partial");
        assert!(codes(&i).contains("unsupported_element"));
    }
    #[test]
    fn duplicate_definitions_are_not_chosen_by_order() {
        let f = Fixture::new();
        let a = "<memberdef id=\"duplicate\" kind=\"function\"><name>a</name></memberdef>";
        let b = "<memberdef id=\"duplicate\" kind=\"function\"><name>b</name></memberdef>";
        let prefix = "<doxygen version=\"1.9.8\"><compounddef id=\"classWidget\" kind=\"class\"><compoundname>Widget</compoundname><sectiondef kind=\"public-func\">";
        let suffix = "</sectiondef></compounddef></doxygen>";
        f.write("classWidget.xml", &format!("{prefix}{a}{a}{b}{suffix}"));
        assert_eq!(import(&f.0).unwrap_err().code, "duplicate_definition");
        f.write("classWidget.xml", &format!("{prefix}{b}{a}{a}{suffix}"));
        assert_eq!(import(&f.0).unwrap_err().code, "duplicate_definition");
    }
    #[test]
    fn ordering_of_index_and_members_does_not_change_output_bytes() {
        let f = Fixture::new();
        let original = f.import();
        f.write("index.xml","<doxygenindex version=\"1.9.8\"><compound refid=\"classWidget\" kind=\"class\"><name>Widget</name></compound><compound refid=\"namespacedemo\" kind=\"namespace\"><name>demo</name><member refid=\"namespacedemo_1run_double\" kind=\"function\"><name>run</name></member><member refid=\"namespacedemo_1run_int\" kind=\"function\"><name>run</name></member></compound></doxygenindex>");
        assert_eq!(
            serde_json::to_vec(&f.import()).unwrap(),
            serde_json::to_vec(&original).unwrap()
        );
        let text = fs::read_to_string(f.0.join("namespacedemo.xml")).unwrap();
        let doc = parse(&text, "doxygen", "fixture").unwrap();
        let ms: Vec<_> = doc
            .descendants()
            .filter(|n| n.has_tag_name("memberdef"))
            .map(|n| n.range())
            .collect();
        let swapped = format!(
            "{}{}{}{}{}",
            &text[..ms[0].start],
            &text[ms[1].clone()],
            &text[ms[0].end..ms[1].start],
            &text[ms[0].clone()],
            &text[ms[1].end..]
        );
        f.write("namespacedemo.xml", &swapped);
        assert_eq!(f.import(), original);
    }
    #[test]
    fn dtd_and_entities_are_disabled_without_resolution() {
        assert_eq!(
            parse(
                "<!DOCTYPE doxygenindex><doxygenindex/>",
                "doxygenindex",
                "fixture"
            )
            .unwrap_err()
            .code,
            "dtd_forbidden"
        );
        assert_eq!(
            parse(
                "<!DOCTYPE doxygenindex [<!ENTITY local 'texto'>]><doxygenindex/>",
                "doxygenindex",
                "fixture"
            )
            .unwrap_err()
            .code,
            "dtd_forbidden"
        );
        assert_eq!(
            parse(
                "<doxygenindex>&undefined;</doxygenindex>",
                "doxygenindex",
                "fixture"
            )
            .unwrap_err()
            .code,
            "invalid_xml"
        );
    }
    #[test]
    fn symlinks_and_path_traversal_are_rejected() {
        let f = Fixture::new();
        fs::remove_file(f.0.join("classWidget.xml")).unwrap();
        std::os::unix::fs::symlink("namespacedemo.xml", f.0.join("classWidget.xml")).unwrap();
        assert_eq!(import(&f.0).unwrap_err().code, "input_read_forbidden");
        f.change("index.xml", "refid=\"classWidget\"", "refid=\"../other\"");
        assert_eq!(import(&f.0).unwrap_err().code, "path_forbidden");
        let mut root = Root::open(&f.0).unwrap();
        assert_eq!(root.read("/etc/passwd").unwrap_err().code, "path_forbidden");
        assert_eq!(root.read("a/b.xml").unwrap_err().code, "path_forbidden");
    }
    #[test]
    fn resource_limits_are_errors_not_truncations() {
        let f = Fixture::new();
        f.write("index.xml", &" ".repeat(FILE_LIMIT + 1));
        assert_eq!(import(&f.0).unwrap_err().code, "resource_limit");
        let deep = format!(
            "<doxygenindex>{}x{}</doxygenindex>",
            "<a>".repeat(130),
            "</a>".repeat(130)
        );
        assert_eq!(
            parse(&deep, "doxygenindex", "fixture").unwrap_err().code,
            "resource_limit"
        );
        let nodes = format!(
            "<doxygenindex>{}</doxygenindex>",
            "<a/>".repeat(NODE_LIMIT as usize)
        );
        assert!(parse(&nodes, "doxygenindex", "fixture").is_err());
        let long = format!("<doxygenindex refid=\"{}\"/>", "x".repeat(513));
        assert_eq!(
            parse(&long, "doxygenindex", "fixture").unwrap_err().code,
            "resource_limit"
        );
        let mut root = Root::open(&f.0).unwrap();
        root.total = TOTAL_LIMIT;
        f.write("tiny.xml", "x");
        assert_eq!(root.read("tiny.xml").unwrap_err().code, "resource_limit");
    }
    #[test]
    fn publication_is_exclusive_complete_and_roundtrips() {
        let f = Fixture::new();
        let i = f.import();
        let out = f.0.join("result.json");
        publish(&out, &i).unwrap();
        let bytes = fs::read(&out).unwrap();
        assert_eq!(serde_json::from_slice::<Inventory>(&bytes).unwrap(), i);
        let mut partial = i.clone();
        partial.coverage = "partial".into();
        assert_eq!(publish(&out, &partial).unwrap_err().code, "output_publish");
        assert_eq!(fs::read(&out).unwrap(), bytes);
        assert!(
            !fs::read_dir(&f.0).unwrap().any(|e| e
                .unwrap()
                .file_name()
                .to_string_lossy()
                .ends_with(".tmp"))
        );
    }
    #[test]
    fn concurrent_publications_have_exactly_one_winner() {
        let f = Fixture::new();
        let i = f.import();
        let out = f.0.join("result.json");
        let barrier = std::sync::Arc::new(std::sync::Barrier::new(2));
        let handles: Vec<_> = (0..2)
            .map(|_| {
                let i = i.clone();
                let out = out.clone();
                let b = barrier.clone();
                std::thread::spawn(move || {
                    b.wait();
                    publish(&out, &i).is_ok()
                })
            })
            .collect();
        assert_eq!(
            handles
                .into_iter()
                .map(|h| usize::from(h.join().unwrap()))
                .sum::<usize>(),
            1
        );
        assert_eq!(
            serde_json::from_slice::<Inventory>(&fs::read(out).unwrap()).unwrap(),
            i
        );
    }
    #[test]
    fn cli_partial_exit_preserves_coverage_and_diagnostics() {
        let f = Fixture::new();
        fs::remove_file(f.0.join("classWidget.xml")).unwrap();
        let out = f.0.join("partial.json");
        let args = vec![
            "--xml-dir".into(),
            f.0.display().to_string(),
            "--output".into(),
            out.display().to_string(),
        ];
        assert_eq!(run(&args).unwrap(), 2);
        let i: Inventory = serde_json::from_slice(&fs::read(out).unwrap()).unwrap();
        assert_eq!(i.coverage, "partial");
        assert!(codes(&i).contains("referenced_file_missing"));
    }
    #[test]
    fn location_and_signature_are_not_silently_normalized() {
        let f = Fixture::new();
        f.change("namespacedemo.xml", "(int valor)", "( int  valor ) const");
        f.change("namespacedemo.xml", "src/demo.cpp", "./src/../src/demo.cpp");
        let i = f.import();
        let s = get(&i, "namespacedemo_1run_int");
        assert_eq!(s.signature.as_deref(), Some("( int  valor ) const"));
        assert_eq!(s.location.as_ref().unwrap().file, "./src/../src/demo.cpp");
        f.change(
            "classWidget.xml",
            "line=\"3\"",
            "bodyfile=\"unread-source.cpp\" bodystart=\"3\"",
        );
        let i = f.import();
        assert!(get(&i, "classWidget").location.is_none());
        assert!(codes(&i).contains("incomplete_location"));
    }
    #[test]
    fn directories_are_not_read_as_xml_and_index_symlinks_are_rejected() {
        let f = Fixture::new();
        fs::remove_file(f.0.join("classWidget.xml")).unwrap();
        fs::create_dir(f.0.join("classWidget.xml")).unwrap();
        assert_eq!(import(&f.0).unwrap_err().code, "input_not_regular");
        fs::remove_file(f.0.join("index.xml")).unwrap();
        std::os::unix::fs::symlink("namespacedemo.xml", f.0.join("index.xml")).unwrap();
        assert_eq!(import(&f.0).unwrap_err().code, "input_read_forbidden");
    }
    #[test]
    fn index_mismatch_and_unresolved_member_have_distinct_diagnostics() {
        let f = Fixture::new();
        f.change("index.xml", "<name>Widget</name>", "<name>Another</name><member refid=\"missing\" kind=\"function\"><name>missing</name></member>");
        let i = f.import();
        assert!(codes(&i).contains("index_definition_mismatch"));
        assert!(codes(&i).contains("unresolved_index_reference"));
        assert_eq!(get(&i, "classWidget").name, "Widget");
        assert!(!i.symbols.iter().any(|s| s.id == "missing"));
    }
    #[test]
    fn repeated_singleton_and_unknown_kind_are_not_silently_accepted() {
        let f = Fixture::new();
        f.change("classWidget.xml", "kind=\"class\"", "kind=\"future-kind\"");
        let i = f.import();
        assert_eq!(get(&i, "classWidget").kind, "future-kind");
        assert!(codes(&i).contains("unsupported_kind"));
        f.change(
            "index.xml",
            "<name>Widget</name>",
            "<name>Widget</name><name>Another</name>",
        );
        assert_eq!(import(&f.0).unwrap_err().code, "invalid_document");
    }
    #[test]
    fn empty_index_is_unknown_not_complete() {
        let f = Fixture::new();
        f.write("index.xml", "<doxygenindex version=\"1.9.8\"/>");
        let i = f.import();
        assert!(i.symbols.is_empty());
        assert!(i.relations.is_empty());
        assert_eq!(i.coverage, "unknown");
    }
    #[test]
    fn unsupported_elements_and_external_refs_do_not_fabricate_symbols() {
        let f = Fixture::new();
        f.change(
            "classWidget.xml",
            "<location file=",
            "<mystery>data</mystery><includes>external.hpp</includes><location file=",
        );
        f.change(
            "classWidget.xml",
            "kindref=\"member\"",
            "kindref=\"member\" external=\"other-project\"",
        );
        let i = f.import();
        assert!(codes(&i).contains("unsupported_element"));
        assert!(codes(&i).contains("reference_without_id"));
        assert!(codes(&i).contains("external_reference"));
        assert_eq!(i.symbols.len(), 4);
        assert!(!i.relations.iter().any(|r| r.from == "classWidget"));
    }
}
