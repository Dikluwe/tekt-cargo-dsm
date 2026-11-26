// Crystalline Lineage
// @prompt 00_nucleo/prompts/infra-leitor-chamadas-go.md
// @prompt-hash 50e2a806
// @layer L3
// @updated 2026-11-26

package main

import (
	"encoding/json"
	"go/ast"
	"go/parser"
	"go/token"
	"os"
	"sort"
)

type posicao [3]int

type funcao struct {
	Arquivo string  `json:"arquivo"`
	Nome    string  `json:"nome"`
	Local   posicao `json:"local"`
}

type chamada struct {
	Origem int     `json:"origem"`
	Local  posicao `json:"local"`
}

type dados struct {
	Funcoes  []funcao  `json:"funcoes"`
	Chamadas []chamada `json:"chamadas"`
}

func identificador(expr ast.Expr) *ast.Ident {
	switch x := ast.Unparen(expr).(type) {
	case *ast.Ident:
		return x
	case *ast.SelectorExpr:
		return x.Sel
	case *ast.IndexExpr:
		return identificador(x.X)
	case *ast.IndexListExpr:
		return identificador(x.X)
	default:
		return nil
	}
}

func nome(d *ast.FuncDecl) string {
	if d.Recv == nil || len(d.Recv.List) != 1 {
		return d.Name.Name
	}
	t := ast.Unparen(d.Recv.List[0].Type)
	if ponteiro, ok := t.(*ast.StarExpr); ok {
		t = ponteiro.X
	}
	if receptor := identificador(t); receptor != nil {
		return receptor.Name + "." + d.Name.Name
	}
	return d.Name.Name
}

func coletar(s *dados, arquivo, texto string) {
	fset := token.NewFileSet()
	f, err := parser.ParseFile(fset, arquivo, texto, parser.SkipObjectResolution)
	if err != nil {
		return
	}
	pos := func(n *ast.Ident) posicao {
		p := fset.PositionFor(n.Pos(), false)
		return posicao{p.Line, p.Column - 1, len(n.Name)}
	}
	for _, decl := range f.Decls {
		d, ok := decl.(*ast.FuncDecl)
		if !ok || d.Body == nil || d.Name.Name == "_" {
			continue
		}
		id := len(s.Funcoes)
		s.Funcoes = append(s.Funcoes, funcao{arquivo, nome(d), pos(d.Name)})
		ast.Inspect(d.Body, func(n ast.Node) bool {
			switch n := n.(type) {
			case *ast.FuncLit:
				return false
			case *ast.CallExpr:
				if alvo := identificador(n.Fun); alvo != nil {
					s.Chamadas = append(s.Chamadas, chamada{id, pos(alvo)})
				}
			}
			return true
		})
	}
}

func main() {
	var fontes map[string]string
	if json.NewDecoder(os.Stdin).Decode(&fontes) != nil {
		os.Exit(1)
	}
	arquivos := make([]string, 0, len(fontes))
	for arquivo := range fontes {
		arquivos = append(arquivos, arquivo)
	}
	sort.Strings(arquivos)
	saida := dados{Funcoes: []funcao{}, Chamadas: []chamada{}}
	for _, arquivo := range arquivos {
		coletar(&saida, arquivo, fontes[arquivo])
	}
	if json.NewEncoder(os.Stdout).Encode(saida) != nil {
		os.Exit(1)
	}
}
