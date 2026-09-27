//! Crystalline Lineage
//! @prompt lab/adaptador-zig/CONTRATO.md
//! @layer Lab
//! @updated 2026-09-26
const std = @import("std");
const builtin = @import("builtin");
comptime {
    if (!std.mem.eql(u8, builtin.zig_version_string, "0.15.2"))
        @compileError("Este helper exige Zig 0.15.2");
}
const Occurrence = struct {
    builtin: []const u8,
    literal: ?[]const u8,
    argument: ?[]const u8,
    line: usize,
    column: usize,
    contexts: []const []const u8,
};
const Diagnostic = struct { code: []const u8, line: usize, column: usize };
const Position = struct { line: usize, column: usize };
fn position(source: []const u8, offset: usize) Position {
    var line: usize = 1;
    var column: usize = 1;
    for (source[0..offset]) |c| {
        if (c == '\n') {
            line += 1;
            column = 1;
        } else {
            column += 1;
        }
    }
    return .{ .line = line, .column = column };
}
fn context(tag: std.zig.Ast.Node.Tag) bool {
    return switch (tag) {
        .if_simple, .@"if", .@"switch", .switch_comma, .while_simple, .while_cont, .@"while", .for_simple, .@"for", .@"comptime", .fn_decl, .test_decl, .bool_and, .bool_or, .@"orelse", .@"catch" => true,
        else => false,
    };
}
fn literalValue(gpa: std.mem.Allocator, tree: *const std.zig.Ast, node: std.zig.Ast.Node.Index) !?[]const u8 {
    if (tree.nodeTag(node) == .string_literal)
        return try std.zig.string_literal.parseAlloc(gpa, tree.tokenSlice(tree.nodeMainToken(node)));
    // Strings multiline têm tokens específicos e conteúdo sem escapes.
    if (tree.nodeTag(node) == .multiline_string_literal) {
        var bytes: std.ArrayList(u8) = .empty;
        var token = tree.firstToken(node);
        const last = tree.lastToken(node);
        while (token <= last) : (token += 1) {
            if (token != tree.firstToken(node)) try bytes.append(gpa, '\n');
            try bytes.appendSlice(gpa, tree.tokenSlice(token)[2..]);
        }
        return try bytes.toOwnedSlice(gpa);
    }
    return null;
}
pub fn main() !void {
    var arena = std.heap.ArenaAllocator.init(std.heap.page_allocator);
    defer arena.deinit();
    const gpa = arena.allocator();
    const args = try std.process.argsAlloc(gpa);
    if (args.len != 2) return error.ExpectedSourceFile;
    const bytes = try std.fs.cwd().readFileAlloc(gpa, args[1], 16 * 1024 * 1024);
    const source = try gpa.dupeZ(u8, bytes);
    var tree = try std.zig.Ast.parse(gpa, source, .zig);
    defer tree.deinit(gpa);
    var diagnostics: std.ArrayList(Diagnostic) = .empty;
    for (tree.errors) |e| {
        const p = position(source, tree.tokenStart(e.token));
        try diagnostics.append(gpa, .{ .code = @tagName(e.tag), .line = p.line, .column = p.column });
    }
    var occurrences: std.ArrayList(Occurrence) = .empty;
    if (tree.errors.len == 0) {
        for (1..tree.nodes.len) |i| {
            const node: std.zig.Ast.Node.Index = @enumFromInt(i);
            var buf: [2]std.zig.Ast.Node.Index = undefined;
            const params = tree.builtinCallParams(&buf, node) orelse continue;
            const token = tree.nodeMainToken(node);
            const name = tree.tokenSlice(token);
            if (!std.mem.eql(u8, name, "@import") and !std.mem.eql(u8, name, "@cImport") and !std.mem.eql(u8, name, "@embedFile")) continue;
            var contexts: std.ArrayList([]const u8) = .empty;
            for (1..tree.nodes.len) |j| {
                const parent: std.zig.Ast.Node.Index = @enumFromInt(j);
                if (parent == node or !context(tree.nodeTag(parent))) continue;
                if (tree.firstToken(parent) <= token and tree.lastToken(parent) >= token)
                    try contexts.append(gpa, @tagName(tree.nodeTag(parent)));
            }
            const p = position(source, tree.tokenStart(token));
            try occurrences.append(gpa, .{
                .builtin = name,
                .literal = if (params.len == 1) try literalValue(gpa, &tree, params[0]) else null,
                .argument = if (params.len == 1) tree.getNodeSource(params[0]) else null,
                .line = p.line,
                .column = p.column,
                .contexts = try contexts.toOwnedSlice(gpa),
            });
        }
    }
    const output = try std.json.Stringify.valueAlloc(gpa, .{
        .parser = "std.zig.Ast",
        .zig_version = builtin.zig_version_string,
        .line_count = position(source, source.len).line,
        .occurrences = occurrences.items,
        .diagnostics = diagnostics.items,
    }, .{});
    try std.fs.File.stdout().writeAll(output);
    try std.fs.File.stdout().writeAll("\n");
}
