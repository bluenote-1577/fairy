use clap::Parser;
use prita::cmdline::*;
use prita::sketch;
use prita::contain;

fn main() {
        

    let cli = Cli::parse();
    match cli.mode {
        Mode::Sketch(sketch_args) => sketch::sketch(sketch_args),
        Mode::Contain(contain_args) => contain::contain(contain_args),
    }
}
