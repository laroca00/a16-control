#![windows_subsystem = "windows"]

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if gienemy_lib::cli::is_cli(&args) {
        std::process::exit(gienemy_lib::cli::run(&args));
    }
    gienemy_lib::run()
}
