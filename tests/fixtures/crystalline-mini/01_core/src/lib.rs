// Violação intencional: L1 importa de L3
use mini_infra::do_something;

pub fn core_func() {
    do_something();
}
