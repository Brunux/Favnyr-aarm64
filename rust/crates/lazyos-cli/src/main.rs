//! The `lazyos` binary: everything is in the library so the tests can drive the
//! dispatch, the host decision and the paths without a process.

fn main() {
    let argv: Vec<String> = std::env::args().collect();
    let argv0 = argv
        .first()
        .cloned()
        .unwrap_or_else(|| "lazyos".to_string());
    let env = lazyos_cli::host::Env::from_process(&argv0);
    std::process::exit(lazyos_cli::run(argv, &env));
}
