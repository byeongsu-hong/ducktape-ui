fn main() {
    let root = match std::env::current_dir() {
        Ok(root) => root,
        Err(error) => {
            eprintln!("error: {error}");
            std::process::exit(1);
        }
    };

    match ducktape_ui::execute(std::env::args().skip(1), &root) {
        Ok(output) => print!("{output}"),
        Err(error) => {
            eprintln!("error: {error}");
            std::process::exit(1);
        }
    }
}
