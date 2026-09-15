#![feature(gen_blocks, test)]

mod dep_matrix;
mod common;


use crate::common::{get_packages, init_alpm};


fn main() {

    let alpm = init_alpm();
    let (db_handle, packages) = get_packages(&alpm);

    let dep_matrix = dep_matrix::DepMatrix::from_alpm_packages(packages, db_handle);

    let edges = dep_matrix.count_edges();
    println!("Edges: {} / Total: {} / Density: {:.2}%", edges, dep_matrix.total_size(), edges as f64 / dep_matrix.total_size() as f64 * 100_f64);

    dep_matrix.print_unneeded();
}
