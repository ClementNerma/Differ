#![forbid(unsafe_code)]
#![forbid(unused_must_use)]

mod cli;
mod diffing;
mod drivers;

fn main() {
    cli::main();
}
