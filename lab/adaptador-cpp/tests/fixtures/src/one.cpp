// Crystalline Lineage
// @prompt lab/adaptador-cpp/README.md
#include "shared.hpp"
#include "unused.hpp"
#include "templates.hpp"
#ifndef FIXTURE_CONFIG
#error compilation database flags not respected
#endif
int one(Item const& value, Base& base, int (*callback)()) {
    return declared(value) + overloaded(1) + int(overloaded(1.5))
        + generic(value) + base.run() + callback() + shared(value);
}
