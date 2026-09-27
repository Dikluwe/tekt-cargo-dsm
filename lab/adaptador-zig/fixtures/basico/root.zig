//! @prompt lab/adaptador-zig/CONTRATO.md — fixture, não executar
const local = @import("local.zig");
const repetido = @import("local.zig");
const pacote = @import("pacote");
const std = @import("std");
const builtin = @import("builtin");
const ausente = @import("ausente");
const gerado = @import("generated");
const condicional = if (builtin.os.tag == .linux) @import("linux.zig") else @import("outro.zig");
comptime { _ = @import("comptime.zig"); }
const c = @cImport({ @cInclude("cabecalho_que_nao_existe.h"); });
const falso = "@import(\"nao_importar.zig\")";
// @import("tambem_nao_importar.zig")
