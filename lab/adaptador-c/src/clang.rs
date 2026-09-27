//! Crystalline Lineage
//! @prompt lab/adaptador-c/CONTRATO.md
//! @layer Lab
//! @updated 2026-09-26
//! FFI mínima, ABI declarada em clang-c/{Index,CXCompilationDatabase}.h (LLVM 18).

use serde_json::{Value, json};
use std::ffi::{CStr, CString, c_char, c_int, c_uint, c_void};
use std::path::{Path, PathBuf};

type Ptr = *mut c_void;
#[repr(C)]
#[derive(Clone, Copy)]
struct Text {
    data: *const c_void,
    flags: c_uint,
}
#[repr(C)]
#[derive(Clone, Copy)]
struct Cursor {
    kind: c_uint,
    xdata: c_int,
    data: [*const c_void; 3],
}
#[repr(C)]
#[derive(Clone, Copy)]
struct Location {
    data: [*const c_void; 2],
    value: c_uint,
}

#[link(name = "clang-18")]
unsafe extern "C" {
    fn clang_getCString(s: Text) -> *const c_char;
    fn clang_disposeString(s: Text);
    fn clang_getClangVersion() -> Text;
    fn clang_CompilationDatabase_fromDirectory(dir: *const c_char, error: *mut c_uint) -> Ptr;
    fn clang_CompilationDatabase_dispose(db: Ptr);
    fn clang_CompilationDatabase_getAllCompileCommands(db: Ptr) -> Ptr;
    fn clang_CompileCommands_dispose(commands: Ptr);
    fn clang_CompileCommands_getSize(commands: Ptr) -> c_uint;
    fn clang_CompileCommands_getCommand(commands: Ptr, i: c_uint) -> Ptr;
    fn clang_CompileCommand_getDirectory(command: Ptr) -> Text;
    fn clang_CompileCommand_getFilename(command: Ptr) -> Text;
    fn clang_CompileCommand_getNumArgs(command: Ptr) -> c_uint;
    fn clang_CompileCommand_getArg(command: Ptr, i: c_uint) -> Text;
    fn clang_createIndex(exclude: c_int, display: c_int) -> Ptr;
    fn clang_disposeIndex(index: Ptr);
    fn clang_parseTranslationUnit2FullArgv(
        index: Ptr,
        source: *const c_char,
        args: *const *const c_char,
        nargs: c_int,
        unsaved: Ptr,
        count: c_uint,
        options: c_uint,
        out: *mut Ptr,
    ) -> c_int;
    fn clang_disposeTranslationUnit(tu: Ptr);
    fn clang_getTranslationUnitCursor(tu: Ptr) -> Cursor;
    fn clang_visitChildren(
        cursor: Cursor,
        visitor: extern "C" fn(Cursor, Cursor, Ptr) -> c_uint,
        data: Ptr,
    ) -> c_uint;
    fn clang_getIncludedFile(cursor: Cursor) -> Ptr;
    fn clang_getCursorLocation(cursor: Cursor) -> Location;
    fn clang_getFileName(file: Ptr) -> Text;
    fn clang_getFile(tu: Ptr, name: *const c_char) -> Ptr;
    fn clang_getLocationForOffset(tu: Ptr, file: Ptr, offset: c_uint) -> Location;
    fn clang_Location_isInSystemHeader(loc: Location) -> c_int;
    fn clang_getSpellingLocation(
        loc: Location,
        file: *mut Ptr,
        line: *mut c_uint,
        column: *mut c_uint,
        offset: *mut c_uint,
    );
    fn clang_getNumDiagnostics(tu: Ptr) -> c_uint;
    fn clang_getDiagnostic(tu: Ptr, i: c_uint) -> Ptr;
    fn clang_getDiagnosticSeverity(d: Ptr) -> c_uint;
    fn clang_formatDiagnostic(d: Ptr, options: c_uint) -> Text;
    fn clang_defaultDiagnosticDisplayOptions() -> c_uint;
    fn clang_disposeDiagnostic(d: Ptr);
}

// Cada CXString é consumida uma vez; nenhum ponteiro escapa de sua TU.
fn text(s: Text) -> String {
    unsafe {
        let p = clang_getCString(s);
        let result = if p.is_null() {
            String::new()
        } else {
            CStr::from_ptr(p).to_string_lossy().into_owned()
        };
        clang_disposeString(s);
        result
    }
}
fn cstring(s: &str) -> Result<CString, String> {
    CString::new(s).map_err(|_| "NUL em argumento/caminho".into())
}
pub fn version() -> String {
    unsafe { text(clang_getClangVersion()) }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Config {
    pub directory: String,
    pub file: String,
    pub arguments: Vec<String>,
}
impl Config {
    pub fn json(&self) -> Value {
        json!({"directory":self.directory,"file":self.file,"arguments":self.arguments})
    }
}

pub fn database(dir: &Path) -> Result<Vec<Config>, String> {
    let name = cstring(&dir.to_string_lossy())?;
    unsafe {
        let mut error = 0;
        let db = clang_CompilationDatabase_fromDirectory(name.as_ptr(), &mut error);
        if db.is_null() || error != 0 {
            return Err(format!("compilation database inválida: {error}"));
        }
        let commands = clang_CompilationDatabase_getAllCompileCommands(db);
        let mut configs = Vec::new();
        for i in 0..clang_CompileCommands_getSize(commands) {
            let command = clang_CompileCommands_getCommand(commands, i);
            configs.push(Config {
                directory: text(clang_CompileCommand_getDirectory(command)),
                file: text(clang_CompileCommand_getFilename(command)),
                arguments: (0..clang_CompileCommand_getNumArgs(command))
                    .map(|j| text(clang_CompileCommand_getArg(command, j)))
                    .collect(),
            });
        }
        clang_CompileCommands_dispose(commands);
        clang_CompilationDatabase_dispose(db);
        configs.sort();
        configs.dedup();
        Ok(configs)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Include {
    pub from: String,
    pub to: String,
    pub line: u32,
    pub column: u32,
    pub offset: u32,
}
pub struct Parsed {
    pub includes: Vec<Include>,
    pub diagnostics: Vec<Value>,
    pub failed: bool,
    pub system: Vec<String>,
}
struct Visitor {
    includes: Vec<Include>,
    directory: PathBuf,
}
fn physical(dir: &Path, file: &str) -> String {
    let p = dir.join(file);
    p.canonicalize().unwrap_or(p).to_string_lossy().into_owned()
}
extern "C" fn visit(cursor: Cursor, _: Cursor, data: Ptr) -> c_uint {
    // SAFETY: data is a live unique Visitor for this synchronous traversal.
    // Only top-level preprocessing cursors are needed, no AST recursion.
    if cursor.kind != 503 {
        return 1;
    }
    unsafe {
        let state = &mut *(data as *mut Visitor);
        let target = clang_getIncludedFile(cursor);
        if target.is_null() {
            return 1;
        }
        let mut source = std::ptr::null_mut();
        let (mut line, mut column, mut offset) = (0, 0, 0);
        clang_getSpellingLocation(
            clang_getCursorLocation(cursor),
            &mut source,
            &mut line,
            &mut column,
            &mut offset,
        );
        if !source.is_null() {
            state.includes.push(Include {
                from: physical(&state.directory, &text(clang_getFileName(source))),
                to: physical(&state.directory, &text(clang_getFileName(target))),
                line,
                column,
                offset,
            });
        }
    }
    1
}

pub fn parse(config: &Config, argv: &[String]) -> Result<Parsed, String> {
    let strings: Vec<CString> = argv.iter().map(|s| cstring(s)).collect::<Result<_, _>>()?;
    let pointers: Vec<_> = strings.iter().map(|s| s.as_ptr()).collect();
    unsafe {
        let index = clang_createIndex(0, 0);
        if index.is_null() {
            return Err("clang_createIndex falhou".into());
        }
        let mut tu = std::ptr::null_mut();
        let error = clang_parseTranslationUnit2FullArgv(
            index,
            std::ptr::null(),
            pointers.as_ptr(),
            pointers.len() as c_int,
            std::ptr::null_mut(),
            0,
            1,
            &mut tu,
        );
        if tu.is_null() || error != 0 {
            if !tu.is_null() {
                clang_disposeTranslationUnit(tu);
            }
            clang_disposeIndex(index);
            return Err(format!("clang_parseTranslationUnit2FullArgv: {error}"));
        }
        let mut result = Parsed {
            includes: Vec::new(),
            diagnostics: Vec::new(),
            failed: false,
            system: Vec::new(),
        };
        for i in 0..clang_getNumDiagnostics(tu) {
            let d = clang_getDiagnostic(tu, i);
            let severity = clang_getDiagnosticSeverity(d);
            result.failed |= severity >= 3;
            result.diagnostics.push(json!({"severity":severity,"message":text(clang_formatDiagnostic(d,clang_defaultDiagnosticDisplayOptions()))}));
            clang_disposeDiagnostic(d);
        }
        let mut visitor = Visitor {
            includes: Vec::new(),
            directory: PathBuf::from(&config.directory),
        };
        clang_visitChildren(
            clang_getTranslationUnitCursor(tu),
            visit,
            &mut visitor as *mut _ as Ptr,
        );
        visitor.includes.sort();
        visitor.includes.dedup();
        for include in &visitor.includes {
            if let Ok(name) = cstring(&include.to) {
                let file = clang_getFile(tu, name.as_ptr());
                if !file.is_null()
                    && clang_Location_isInSystemHeader(clang_getLocationForOffset(tu, file, 0)) != 0
                {
                    result.system.push(include.to.clone());
                }
            }
        }
        result.system.sort();
        result.system.dedup();
        result.includes = visitor.includes;
        clang_disposeTranslationUnit(tu);
        clang_disposeIndex(index);
        Ok(result)
    }
}
