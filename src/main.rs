#[allow(
    redundant_imports,
    unused_imports,
    clippy::single_component_path_imports
)]
use brooks_lib;

#[cfg(test)]
mod test;

use clap::{Parser, Subcommand};

#[derive(Parser)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Compile {
        #[arg(long)]
        path: String,
    },
}

fn main() {
    let cli = Cli::parse();
}
