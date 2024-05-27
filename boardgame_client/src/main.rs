mod catan;
mod common;
mod greedy_snake;

fn main() {
    #[cfg(target_family = "wasm")]
    {
        console_error_panic_hook::set_once();
        // tracing_wasm::set_as_global_default();
    }
    catan::catan_run();
    // greedy_snake::greedy_snake_run();
}
