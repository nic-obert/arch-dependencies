#![feature(gen_blocks)]

mod dep_matrix;
mod common;


use alpm::Alpm;


const FS_ROOT: &str = "/";
const DB_PATH: &str = "/var/lib/pacman";


fn main() {

    let alpm_handle = Alpm::new(FS_ROOT, DB_PATH).unwrap_or_else(
        |e| panic!("Failed to initialize ALPM: {}", e)
    );

    let db_handle = alpm_handle.localdb();

    if let Err(e) = db_handle.is_valid() {
        panic!("Error checking database validity: {}", e);
    }

    let packages = db_handle.pkgs();

    let dep_matrix = dep_matrix::DepMatrix::from_alpm_packages(packages, db_handle);

    let edges = dep_matrix.count_edges();
    println!("Edges: {} / Total: {} / Density: {:.2}%", edges, dep_matrix.total_size(), edges as f64 / dep_matrix.total_size() as f64 * 100_f64);

    dep_matrix.compute_unneeded_packages_dfs();
}
