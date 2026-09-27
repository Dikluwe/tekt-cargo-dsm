#!/usr/bin/env python3
# Crystalline Lineage
# @prompt lab/adaptador-cpp/README.md
# @layer Lab
"""Dependências semânticas C++ por arquivo via libclang e compilation database."""
import argparse
import hashlib
import json
from pathlib import Path
import shlex
from clang_api import Clang


def encode(value):
    return json.dumps(value, ensure_ascii=False, sort_keys=True, indent=2) + "\n"


def digest(value):
    return hashlib.sha256(encode(value).encode()).hexdigest()


def load_commands(database):
    """Nunca executa argv: apenas prepara argumentos aceitos por libclang."""
    result = []
    for entry in json.loads(Path(database).read_text(encoding="utf-8")):
        directory = Path(entry["directory"])
        if not directory.is_absolute() or not directory.is_dir():
            raise ValueError("directory deve ser absoluto e existir")
        directory = directory.resolve()
        source = (directory / entry["file"]).resolve()
        argv = entry.get("arguments")
        if argv is None:
            argv = shlex.split(entry["command"], posix=True)
        if not isinstance(argv, list) or not argv or not all(isinstance(a, str) for a in argv):
            raise ValueError("arguments deve ser argv não vazio")
        if Path(argv[0]).name not in {"clang++", "clang", "g++", "c++"} and not Path(argv[0]).name.startswith(("clang++-", "clang-", "g++-")):
            raise ValueError("compiler/wrapper não suportado: " + argv[0])
        args, removed = [], []
        i = 1
        paired = {"-I", "-isystem", "-iquote", "-idirafter", "-include", "-D", "-U", "-isysroot", "--sysroot", "-target", "--target", "-x", "-resource-dir"}
        standalone = {"-pthread", "-nostdinc", "-nostdinc++", "-fno-exceptions", "-fexceptions", "-fno-rtti", "-frtti", "-fPIC", "-fpic", "-m32", "-m64"}
        while i < len(argv):
            arg = argv[i]
            if not arg.startswith("-") and (directory / arg).resolve() == source:
                i += 1
                continue
            if arg in {"-o", "-MF", "-MT", "-MQ", "-MJ"}:
                if i + 1 == len(argv):
                    raise ValueError("argumento incompleto: " + arg)
                removed.extend(argv[i:i + 2]); i += 2; continue
            if arg in {"-c", "-MD", "-MMD", "-MP"} or arg.startswith("-o"):
                removed.append(arg); i += 1; continue
            if arg in paired:
                if i + 1 == len(argv):
                    raise ValueError("argumento incompleto: " + arg)
                args.extend(argv[i:i + 2]); i += 2; continue
            if arg in standalone or arg.startswith(("-std=", "-I", "-D", "-U", "-O", "-g", "-W", "--sysroot=", "--target=")):
                args.append(arg); i += 1; continue
            # Inclui plugins, @response files, PCH, modules e ações de driver.
            raise ValueError("flag não suportada (não foi descartada): " + arg)
        config = dict(directory=str(directory), compiler=argv[0], arguments=args)
        result.append(dict(source=str(source), config=config, configuration=digest(config), removed=removed))
    if not result:
        raise ValueError("compilation database vazia")
    return sorted(result, key=encode)


def analyze(database, root, library, configuration=None):
    root = Path(root).resolve(strict=True)
    commands = load_commands(database)
    configs = {c["configuration"]: c["config"] for c in commands}
    if configuration is None:
        if len(configs) != 1:
            raise ValueError("configurações distintas; selecione --configuration: " + ", ".join(sorted(configs)))
        configuration = next(iter(configs))
    if configuration not in configs:
        raise ValueError("configuration desconhecida")
    selected = {c["source"]: c for c in commands if c["configuration"] == configuration}
    clang = Clang(library)
    diagnostics, evidence, includes, symbols = [], {}, {}, {}
    files = set()
    parsed = 0

    def in_scope(file):
        return Path(file).is_relative_to(root)

    def record(table, item, tu):
        key = encode(item)
        if key not in table:
            table[key] = dict(item, observed_in=[])
        if tu not in table[key]["observed_in"]:
            table[key]["observed_in"].append(tu)

    def diagnostic(code, tu, **details):
        diagnostics.append(dict(code=code, translation_unit=tu, **details))

    for tu, command in sorted(selected.items()):
        directory = Path(command["config"]["directory"])
        if not in_scope(tu):
            diagnostic("translation_unit_outside_scope", tu)
            continue
        if Path(tu).suffix in {".cppm", ".ixx", ".mpp"}:
            diagnostic("cpp20_modules_unsupported", tu)
            continue
        files.add(tu)
        # argv[0] preserva o modo C++ do driver; nunca é executado.
        argv = [command["config"]["compiler"], *command["config"]["arguments"], "-working-directory=" + str(directory)]
        index, unit, code = clang.parse(tu, argv)
        try:
            if code or not unit:
                diagnostic("parse_failed", tu, error_code=code)
                continue
            errors = False
            for i in range(clang.getNumDiagnostics(unit)):
                value = clang.getDiagnostic(unit, i)
                try:
                    severity = clang.getDiagnosticSeverity(value)
                    diagnostic("clang_diagnostic", tu, severity=severity, message=clang.text(clang.formatDiagnostic(value, 0)))
                    errors |= severity >= 3
                finally:
                    clang.disposeDiagnostic(value)
            if errors:
                # AST com erro pode conter resolução recuperada; não publicá-la como evidência.
                diagnostic("translation_unit_discarded", tu)
                continue
            parsed += 1

            def visit(cursor, context="ordinary"):
                kind = clang.kind(cursor)
                loc = clang.location(cursor, directory)
                if loc and not in_scope(loc["file"]):
                    return
                if loc:
                    files.add(loc["file"])
                specialized = clang.getSpecializedCursorTemplate(cursor)
                if kind in {"FunctionTemplate", "ClassTemplate", "ClassTemplatePartialSpecialization"}:
                    context = "pattern"
                elif not clang.Cursor_isNull(specialized):
                    context = "instantiation_or_specialization"
                if cursor.kind == 503 and loc:  # CXCursor_InclusionDirective
                    included = clang.getIncludedFile(cursor)
                    if included:
                        target = str((directory / clang.text(clang.getFileName(included))).resolve())
                        if in_scope(target):
                            files.add(target)
                        record(includes, dict(source=loc, target_file=target, target_in_scope=in_scope(target)), tu)
                elif cursor.kind == 502 and loc:  # CXCursor_MacroExpansion
                    diagnostic("macro_expansion_partial", tu, source=loc, spelling=clang.text(clang.getCursorSpelling(cursor)))
                elif cursor.kind == 600:  # CXCursor_ModuleImportDecl
                    diagnostic("cpp20_modules_unsupported", tu, source=loc)
                if loc and clang.isDeclaration(cursor.kind):
                    usr = clang.text(clang.getCursorUSR(cursor))
                    if usr:
                        canonical = clang.getCanonicalCursor(cursor)
                        declaration = clang.location(canonical, directory)
                        record(symbols, dict(usr=usr, name=clang.text(clang.getCursorDisplayName(cursor)), location=loc, canonical_declaration=declaration, is_definition=bool(clang.isCursorDefinition(cursor)), template_origin=context), tu)
                        if declaration and declaration["file"] != loc["file"]:
                            record(evidence, dict(source=loc, target=declaration, usr=usr, declaration=clang.text(clang.getCursorDisplayName(canonical)), nature="declaration_definition" if clang.isCursorDefinition(cursor) else "redeclaration", template_origin=context, target_template="not_applicable", runtime_target=False), tu)
                refs = {"TypeRef", "TemplateRef", "DeclRefExpr", "MemberRefExpr", "C++ base class specifier", "CallExpr", "OverloadedDeclRef"}
                if loc and kind in refs:
                    ref = clang.getCursorReferenced(cursor)
                    if clang.Cursor_isNull(ref) or kind == "OverloadedDeclRef":
                        diagnostic("unresolved_reference", tu, source=loc, cursor_kind=kind, template_origin=context)
                    else:
                        ref_kind = clang.kind(ref)
                        if kind == "CallExpr" and ref_kind not in {"FunctionDecl", "CXXMethod", "Constructor", "ConversionFunction", "FunctionTemplate"}:
                            diagnostic("indirect_call_no_runtime_target", tu, source=loc)
                        else:
                            canonical = clang.getCanonicalCursor(ref)
                            target = clang.location(canonical, directory)
                            virtual = bool(clang.CXXMethod_isVirtual(ref)) if ref_kind == "CXXMethod" else False
                            nature = "virtual_dispatch_static_declaration" if kind == "CallExpr" and virtual else ("static_call" if kind == "CallExpr" else kind)
                            if kind == "CallExpr" and virtual:
                                diagnostic("virtual_call_no_runtime_target", tu, source=loc)
                            if target and target["file"] != loc["file"]:
                                inst = not clang.Cursor_isNull(clang.getSpecializedCursorTemplate(ref))
                                record(evidence, dict(source=loc, target=target, usr=clang.text(clang.getCursorUSR(canonical)), declaration=clang.text(clang.getCursorDisplayName(canonical)), nature=nature, template_origin=context, target_template="instantiation_or_specialization" if inst else "pattern" if "Template" in ref_kind else "ordinary", runtime_target=False), tu)
                            elif target is None:
                                diagnostic("reference_without_file", tu, source=loc, cursor_kind=kind)
                for child in clang.children(cursor):
                    visit(child, context)
            visit(clang.getTranslationUnitCursor(unit))
        finally:
            if unit:
                clang.disposeTranslationUnit(unit)
            clang.disposeIndex(index)

    def values(table):
        for item in table.values():
            item["observed_in"].sort()
        return sorted(table.values(), key=encode)
    semantic = values(evidence)
    for item in semantic:
        if in_scope(item["target"]["file"]):
            files.add(item["target"]["file"])
    names = sorted(files)
    ids = {name: i for i, name in enumerate(names)}
    def path(file):
        return "cpp::" + Path(file).relative_to(root).as_posix()
    pairs = sorted({(e["source"]["file"], e["target"]["file"]) for e in semantic if e["source"]["file"] in ids and e["target"]["file"] in ids})
    graph = dict(crate="cpp", nodes=[dict(id=ids[f], path=path(f), name=Path(f).relative_to(root).as_posix(), kind="mod", visibility="pub") for f in names], edges=[dict(id_from=ids[a], id_to=ids[b], **{"from": path(a), "to": path(b)}, relation="uses", uses_kind=None) for a, b in pairs])
    metadata = dict(metadata_version=1, provider="experimental-libclang-cpp", clang_version=clang.version, root=str(root), language="cpp", granularity="file", configuration=configuration, selected_configuration=configs[configuration], other_configurations=sorted(set(configs) - {configuration}), commands=list(selected.values()), semantic_evidence=semantic, includes=values(includes), symbols=values(symbols), diagnostics=sorted(diagnostics, key=encode), execution=dict(status="complete" if parsed == len(selected) else "partial", translation_units_requested=len(selected), translation_units_parsed=parsed), semantic_coverage="partial", capabilities=dict(semantic_references="observed-libclang-cursors", includes="separate-preprocessing-record", templates="patterns-and-observed-specialization-references-only", virtual_runtime_targets=False, indirect_runtime_targets=False, cpp20_modules=False, macros="expansion-locations-only-partial"), identity_scope="single-extraction", graph_sha256=hashlib.sha256(encode(graph).encode()).hexdigest(), cache=False)
    return graph, metadata


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--database", required=True)
    parser.add_argument("--root", required=True)
    parser.add_argument("--libclang", required=True, help="biblioteca compartilhada instalada pelo usuário")
    parser.add_argument("--configuration")
    parser.add_argument("--graph", type=Path, required=True)
    parser.add_argument("--metadata", type=Path, required=True)
    args = parser.parse_args()
    try:
        if args.graph.resolve() == args.metadata.resolve():
            raise ValueError("saídas devem ser distintas")
        graph, metadata = analyze(args.database, args.root, args.libclang, args.configuration)
        args.graph.write_text(encode(graph), encoding="utf-8")
        args.metadata.write_text(encode(metadata), encoding="utf-8")
    except (OSError, ValueError, KeyError, AttributeError) as error:
        parser.exit(2, str(error) + "\n")
    return 0 if metadata["execution"]["status"] == "complete" else 1


if __name__ == "__main__":
    raise SystemExit(main())
