# Extração sintática de ctypes (L3)
Hash do Código: 4a5e65b8

Leia documentos Python UTF-8 pelo protocolo do adaptador e analise ast.parse.
Conserve os identificadores de chamadas e funções envolventes.
As pontes candidatas exigem um import de CDLL de ctypes, uma carga por
caminho literal e chamada por atributo direto. Evite atribuir a ligação
quando nomes de importação ou variáveis foram redefinidos ou sombreados.
