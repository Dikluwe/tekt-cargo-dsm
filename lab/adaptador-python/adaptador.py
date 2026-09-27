#!/usr/bin/env python3
# Crystalline Lineage
# @prompt lab/adaptador-python/README.md
# @layer Lab
"""Extração experimental de imports Python. Apenas AST; nunca importa o alvo."""
import argparse
import ast
import hashlib
import json
import os
from pathlib import Path
import tokenize


def analisar(raizes):
    """Raízes são diretórios de busca explícitos, sem precedência entre si."""
    roots = sorted({Path(r).resolve(strict=True) for r in raizes})
    if not roots or any(not r.is_dir() for r in roots):
        raise ValueError("informe ao menos uma raiz de código que seja diretório")
    diagnostics = []

    def diag(code, file, line=0, **details):
        diagnostics.append(dict(code=code, file=str(file), line=line, **details))

    candidates = {}
    excluded_tops = set()
    discovered = set()
    for root in roots:
        def walk_error(error):
            diag("io_error", error.filename or root, reason=str(error))
        for folder, dirs, files in os.walk(root, followlinks=False, onerror=walk_error):
            dirs.sort()
            for directory in dirs[:]:
                path = Path(folder) / directory
                if path.is_symlink():
                    diag("symlink_ignored", path)
                    dirs.remove(directory)
                elif directory in {".git", "__pycache__"}:
                    dirs.remove(directory)
            for filename in sorted(files):
                if not filename.endswith(".py"):
                    continue
                file = Path(folder) / filename
                discovered.add(str(file))
                rel = file.relative_to(root)
                if file.is_symlink():
                    diag("symlink_ignored", file)
                    excluded_tops.add(rel.parts[0].removesuffix(".py"))
                    continue
                package = filename == "__init__.py"
                parts = list(rel.parts[:-1]) + ([] if package else [file.stem])
                if not parts:
                    diag("root_init_ignored", file, reason="declare o diretório pai do pacote como raiz")
                    continue
                if not all(p.isidentifier() for p in parts):
                    diag("invalid_module_name", file)
                    continue
                parents = [root.joinpath(*rel.parts[:i]) for i in range(1, len(rel.parts))]
                if any(not (p / "__init__.py").is_file() or (p / "__init__.py").is_symlink() for p in parents):
                    diag("namespace_package_unsupported", file)
                    excluded_tops.add(parts[0])
                    continue
                name = ".".join(parts)
                candidates.setdefault(name, {})[str(file)] = (file, package)

    # Não adota a ordem de raízes como se fosse sys.path.
    ambiguous = {name for name, files in candidates.items() if len(files) > 1}
    for name in sorted(ambiguous):
        diag("ambiguous_module", "", module=name, candidates=sorted(candidates[name]))
    modules = {}
    for name, files in sorted(candidates.items()):
        prefixes = [".".join(name.split(".")[:i]) for i in range(1, len(name.split(".")) + 1)]
        if any(p in ambiguous for p in prefixes):
            continue
        file, package = next(iter(files.values()))
        source = None
        try:
            with tokenize.open(file) as stream:
                source = stream.read()
            tree = ast.parse(source, filename=str(file))
        except (SyntaxError, UnicodeError, OSError, LookupError) as error:
            code = "syntax_error" if isinstance(error, SyntaxError) else "read_error"
            diag(code, file, getattr(error, "lineno", 0) or 0, reason=str(error))
            modules[name] = dict(file=file, package=package, tree=None, lines=max(1, len(source.splitlines())) if source is not None else None, bindings=set(), wildcard=True)
            continue
        # Conservador: bindings mesmo condicionais podem esconder um submódulo.
        bindings = set()
        wildcard = False
        def collect(node):
            nonlocal wildcard
            if isinstance(node, (ast.FunctionDef, ast.AsyncFunctionDef, ast.ClassDef)):
                bindings.add(node.name)
                return
            if isinstance(node, ast.Name) and isinstance(node.ctx, ast.Store):
                bindings.add(node.id)
            if isinstance(node, (ast.Import, ast.ImportFrom)):
                for alias in node.names:
                    if alias.name == "*":
                        wildcard = True
                    else:
                        bindings.add(alias.asname or (alias.name.split(".")[0] if isinstance(node, ast.Import) else alias.name))
            for child in ast.iter_child_nodes(node):
                collect(child)
        collect(tree)
        modules[name] = dict(file=file, package=package, tree=tree, lines=max(1, len(source.splitlines())), bindings=bindings, wildcard=wildcard)

    dependencies = set()
    local_tops = {n.split(".")[0] for n in candidates} | excluded_tops

    def resolve(name, owner, line, relative=False):
        parts = name.split(".")
        prefixes = [".".join(parts[:i]) for i in range(1, len(parts) + 1)]
        if any(p in ambiguous for p in prefixes):
            diag("ambiguous_import", modules[owner]["file"], line, module=name)
            return False
        if name not in modules:
            code = "unresolved_local_import" if relative or parts[0] in local_tops else "external_dependency"
            diag(code, modules[owner]["file"], line, module=name)
            return False
        # import a.b depende também dos inicializadores de a.
        for prefix in prefixes:
            if prefix in modules:
                dependencies.add((owner, prefix))
        return True

    for name, module in modules.items():
        if module["tree"] is None:
            continue
        for node in ast.walk(module["tree"]):
            if isinstance(node, ast.Import):
                for alias in node.names:
                    resolve(alias.name, name, node.lineno)
            elif isinstance(node, ast.ImportFrom):
                if node.level:
                    package_parts = name.split(".") if module["package"] else name.split(".")[:-1]
                    if node.level > len(package_parts):
                        diag("relative_out_of_scope", module["file"], node.lineno)
                        continue
                    base_parts = package_parts[:len(package_parts) - node.level + 1]
                    base = ".".join(base_parts + ([node.module] if node.module else []))
                else:
                    base = node.module or ""
                if not resolve(base, name, node.lineno, relative=bool(node.level)):
                    continue
                target = modules[base]
                for alias in node.names:
                    if alias.name == "*":
                        diag("wildcard_import", module["file"], node.lineno, module=base)
                        continue
                    child = base + "." + alias.name
                    if child in candidates:
                        if alias.name in target["bindings"] or "__getattr__" in target["bindings"] or target["wildcard"]:
                            diag("ambiguous_symbol_or_module", module["file"], node.lineno, module=child)
                        else:
                            resolve(child, name, node.lineno, relative=True)
                    # Um símbolo não vira nó nem aresta para um submódulo fictício.
            elif isinstance(node, ast.Call):
                func = node.func
                if (isinstance(func, ast.Name) and func.id == "__import__") or (isinstance(func, ast.Attribute) and func.attr == "import_module"):
                    diag("dynamic_import_unsupported", module["file"], node.lineno)

    names = sorted(modules)
    ids = {name: index for index, name in enumerate(names)}
    def graph_path(name):
        return "python::" + name.replace(".", "::")
    graph = dict(crate="python", nodes=[dict(
        id=ids[name], path=graph_path(name), name=name, kind="mod", visibility="pub",
        position=(dict(file=str(modules[name]["file"]), start_line=1, end_line=modules[name]["lines"]) if modules[name]["lines"] is not None else None)
    ) for name in names], edges=[dict(
        id_from=ids[a], id_to=ids[b], **{"from": graph_path(a), "to": graph_path(b)},
        relation="uses", uses_kind="import"
    ) for a, b in sorted(dependencies)])
    diagnostics.sort(key=lambda d: json.dumps(d, sort_keys=True, ensure_ascii=False))
    errors = {"io_error", "read_error", "syntax_error", "ambiguous_module", "namespace_package_unsupported", "root_init_ignored", "invalid_module_name", "symlink_ignored"}
    metadata = dict(
        metadata_version=1, provider="experimental-python-ast", language="python",
        granularity="module", roots=[str(r) for r in roots],
        identity=dict(scope="single-extraction", deterministic_for="same roots, files and adapter version", persistent=False),
        capabilities=dict(module_imports="partial-static", type_references=False, calls=False, positions="whole-file"),
        execution=dict(status="partial" if any(d["code"] in errors for d in diagnostics) else "complete", files_discovered=len(discovered), modules_emitted=len(names), modules_parsed=sum(m["tree"] is not None for m in modules.values())),
        semantic_coverage="partial", external_policy="diagnostic-only-no-nodes-or-edges",
        diagnostics=diagnostics, cache=False,
        graph_sha256=hashlib.sha256(encode(graph).encode("utf-8")).hexdigest(),
        limits=["dynamic imports (including aliases) are not resolved", "namespace packages are not supported", "runtime sys.path and __path__ mutations are not evaluated", "conditional/function imports are syntactic dependencies, not proof of execution", "runtime package attributes and reexports may change import resolution"])
    return graph, metadata


def encode(value):
    return json.dumps(value, ensure_ascii=False, sort_keys=True, indent=2) + "\n"


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", action="append", required=True, help="diretório de busca explícito; repetível")
    parser.add_argument("--graph", type=Path, required=True)
    parser.add_argument("--metadata", type=Path, required=True)
    args = parser.parse_args()
    if args.graph.resolve() == args.metadata.resolve():
        parser.error("grafo e metadados exigem arquivos distintos")
    try:
        graph, metadata = analisar(args.root)
        args.graph.write_text(encode(graph), encoding="utf-8")
        args.metadata.write_text(encode(metadata), encoding="utf-8")
    except (OSError, ValueError) as error:
        parser.exit(2, f"erro: {error}\n")
    return 0 if metadata["execution"]["status"] == "complete" else 1


if __name__ == "__main__":
    raise SystemExit(main())
