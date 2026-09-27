// Crystalline Lineage
// @prompt lab/adaptador-cpp/README.md
#pragma once
struct Item { int value; };
struct Base { virtual int run(); };
int overloaded(int);
double overloaded(double);
int declared(Item const&);
template<class T> int generic(T const& value) { return declared(value); }
