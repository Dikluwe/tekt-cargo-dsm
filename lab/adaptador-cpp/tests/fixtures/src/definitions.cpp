// Crystalline Lineage
// @prompt lab/adaptador-cpp/README.md
#include "types.hpp"
int declared(Item const& value) { return value.value; }
int overloaded(int value) { return value; }
double overloaded(double value) { return value; }
int Base::run() { return 1; }
