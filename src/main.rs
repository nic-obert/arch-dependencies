#![feature(gen_blocks, test)]

mod dep_matrix;
mod common;


use crate::{common::{get_packages, init_alpm}, dep_matrix::print_unneeded_no_matrix};


fn main() {

    let argv = std::env::args_os();
    if argv.len() > 1 {
        println!("
Depencency graph analyzer - help page

Run this program without arguments

This program lists all locally installed packages that are not needed, and can be safely removed.
A package is unneeded if it was installed as a dependency and it's not (optionally) required, directly or indirectly, by any other needed package.

Note: some packages may depend on virtual packages, which may have multiple providers. If the virtual package name is found to be needed, this program will treat all its providers as needed.
");
    } else {
        let alpm = init_alpm();
        let (db, packages) = get_packages(&alpm);
    
        print_unneeded_no_matrix(db, packages);
    }
}
