// Prevents an extra console window on Windows in release builds.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if let Some(i) = args.iter().position(|a| a == "--launch") {
        match args.get(i + 1) {
            Some(id) => std::process::exit(facet_lib::run_launch_cli(id)),
            None => {
                eprintln!("facet: --launch needs an account id");
                std::process::exit(2);
            }
        }
    }
    // `facet --open-url <url>`: the link opener (D-022). Exits without a window.
    if let Some(i) = args.iter().position(|a| a == "--open-url") {
        std::process::exit(facet_lib::run_open_url_cli(
            args.get(i + 1).map(String::as_str),
        ));
    }
    facet_lib::run()
}
