use std::path::PathBuf;

#[derive(clap::Parser)]
struct Args {
    path: PathBuf,
}

fn main() -> eyre::Result {
    color_eyre::install().unwrap();
    let args: Args = clap::Parser::parse();
    let file = fs_err::read(args.path)?;
    let parser = tp_parser::Parser::new(&file);
    let newlines = parser.parse_file();
    Ok(())
}
