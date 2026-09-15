use std::{collections::{HashMap, hash_map::Entry}, fs, io::{self, Write}, path::Path};

use alpm::{AlpmList, Package, PackageReason};

use crate::common::{DepHash, PackageIndex, PkgPtr, ProviderList};


/// A dense square matrix implemented as a flat array.
/// Each row corresponds to a package, and each column corresponds to a package that it depends on.
/// Optional dependencies are also included in the matrix, but only if they are present in the local database.
pub struct DepMatrix<'a> {
    matrix: Vec<bool>,
    side: usize,
    index_to_pkg: Vec<&'a Package>,
    pkg_to_index: HashMap<PkgPtr, PackageIndex>,
}

impl<'a> DepMatrix<'a> {

    pub fn from_alpm_packages(alpm_packages: AlpmList<&'a Package>, db_handle: &alpm::Db) -> Self {

        let pkg_count = alpm_packages.len();

        let mut pkg_to_index = HashMap::with_capacity(pkg_count);
        // Use this structure for fast iteration
        let mut index_to_pkg = Vec::with_capacity(pkg_count);
        // We do not know how many packages are provided by the local packages
        // TODO: estimate the size empirically based on the number of installed packages?
        let mut dep_to_providers: HashMap<DepHash, ProviderList>  = HashMap::new();

        for pkg in alpm_packages {

            let index = PackageIndex(index_to_pkg.len());
            pkg_to_index.insert(PkgPtr::from(pkg), index);
            index_to_pkg.push(pkg);

            for provided in pkg.provides() {
                match dep_to_providers.entry(DepHash(provided.name_hash())) {
                    Entry::Occupied(mut occupied_entry) => {
                        occupied_entry.get_mut().add_provider(index);
                        // TODO: suppress this warning in case a virtual package is provided both by a package and its lib32 version (not that simple since some lib32 packages may have different naming)
                        // eprintln!("Package {} provides {}, which is already provided by package {}. Adding to the list of providers.", pkg.name(), provided.name(), index_to_pkg[occupied_entry.get().head.pkg.0].name());
                    },
                    Entry::Vacant(vacant_entry) => {
                        vacant_entry.insert(ProviderList::new(index));
                    },
                }
            }
        }

        let mut matrix = vec![false; pkg_count*pkg_count];

        for (row, pkg) in index_to_pkg.iter().enumerate() {

            let row_base_index = row*pkg_count;

            for hard_dep in pkg.depends() {
                // First check if there is a real package that satisfies this requirement. If not, expect the dependency to be provided as a virtual package
                // TODO: we may want to introduce a cache that maps a dependency name hash to a PackageIndex or &Package
                // It's not ideal to hash a string every time when we could hash an integer instead. Benchmark this
                // Also, calling the .pkg() function repeatedly may introduce significant overhead from allocations and checks. See function definition
                if let Ok(dep_pkg) = db_handle.pkg(hard_dep.name()) {
                    let column = pkg_to_index.get(&PkgPtr::from(dep_pkg)).unwrap().0;
                    matrix[row_base_index+column] = true;
                } else {
                    let providers = dep_to_providers.get(&DepHash(hard_dep.name_hash())).unwrap();

                    // Debug:
                    // if providers.iter().count() > 1 {
                    //     println!("Hard dependency `{}` has multiple providers: {:?}", hard_dep.name(), providers.iter().map(|p| index_to_pkg[p.0].name()).collect::<Vec<_>>());
                    // }

                    for provider in providers.iter() {
                        // TODO: filter providers by version constraints and architecture
                        let column = provider.0;
                        matrix[row_base_index+column] = true;
                    }
                }
            }

            for opt_dep in pkg.optdepends() {
                // First check if there is a real package that satisfies this requirement. If not, check if the dependency is provided as a virtual package
                if let Ok(dep_pkg) = db_handle.pkg(opt_dep.name()) {
                    let column = pkg_to_index.get(&PkgPtr::from(dep_pkg)).unwrap().0;
                    matrix[row_base_index+column] = true;
                } else if let Some(providers) = dep_to_providers.get(&DepHash(opt_dep.name_hash())) {

                    // Debug:
                    // if providers.iter().count() > 1 {
                    //     println!("Optional dependency `{}` has multiple providers: {:?}", opt_dep.name(), providers.iter().map(|p| index_to_pkg[p.0].name()).collect::<Vec<_>>());
                    // }

                    for provider in providers.iter() {
                        // TODO: filter providers by version constraints and architecture
                        let column = provider.0;
                        matrix[row_base_index+column] = true;
                    }
                }
            }
        }

        DepMatrix {
            matrix,
            side: pkg_count,
            index_to_pkg,
            pkg_to_index,
        }
    }


    pub fn export_to_file(&self, file_path: &Path) -> io::Result<()> {
        let mut file = fs::File::create(file_path)?;
        for row in 0..self.side {
            let row_base_index = row*self.side;
            for column in 0..self.side {
                let needed = self.matrix[row_base_index+column];
                write!(file, "{} ", needed as u8)?
            }
            writeln!(file, "")?;
        }
        Ok(())
    }


    pub fn compute_needed(&self) -> Vec<bool> {
        // Should include all output from `pacman -Qdt`, plus non-explicit circular dependencies
        // Iterate over all explicitly installed packages.
        // Mark all their dependencies as needed by visiting them using DFS.
        // Unvisited nodes are therefore not needed by any explicitly installed pakcage.
        let mut visited: Vec<bool> = vec![false; self.side];
        let mut stack: Vec<PackageIndex> = Vec::new();

        for &package in &self.index_to_pkg {
            let pkg_index = self.pkg_to_index[&PkgPtr::from(package)];

            // Skip already visited packages (we already know they are needed) and non-explicit packages
            if visited[pkg_index.0] || matches!(package.reason(), PackageReason::Depend) {
                continue;
            }
            // In this branch, the package is explicitly installed and not visited yet
            visited[pkg_index.0] = true;

            // Push its dependencies onto the stack for DFS
            let row_base_index = pkg_index.0*self.side;
            for col in 0..self.side {
                // Only push dependencies that have not been visited yet
                if self.matrix[row_base_index+col] && !visited[col] {
                    stack.push(PackageIndex(col));
                }
            }
            while let Some(pkg_index) = stack.pop() {
                if visited[pkg_index.0] {
                    continue;
                }
                visited[pkg_index.0] = true;

                // Iterate over the row corresponding to the current package and push all packages that it depends on onto the stack
                let row_base_index = pkg_index.0*self.side;
                for col in 0..self.side {
                    if self.matrix[row_base_index+col] && !visited[col] {
                        stack.push(PackageIndex(col));
                    }
                }
            }
        }

        visited
    }


    pub fn print_unneeded(&self) {

        let needed = self.compute_needed();

        // Print all unneeded packages
        for (index, package) in self.index_to_pkg.iter().enumerate() {
            if !needed[index] {
                println!("{}", package.name());
            }
        }
    }


    pub fn count_edges(&self) -> usize {
        self.matrix.iter().filter(|&&x| x).count()
    }


    pub fn total_size(&self) -> usize {
        self.side*self.side
    }


    pub fn compute_density(&self) -> f64 {
        self.count_edges() as f64 / self.total_size() as f64
    }

}


#[cfg(test)]
mod tests {
    extern crate test;
    use crate::common::{get_packages, init_alpm};

    use super::*;
    use test::Bencher;


    #[bench]
    fn build_matrix(b: &mut Bencher) {
        let alpm = init_alpm();
        let (db, packages) = get_packages(&alpm);
        b.iter(|| {
            DepMatrix::from_alpm_packages(packages, db)
        })
    }

    
    #[bench]
    fn compute_unneeded(b: &mut Bencher) {
        let alpm = init_alpm();
        let (db, packages) = get_packages(&alpm);
        let matrix = DepMatrix::from_alpm_packages(packages, db);
        b.iter(|| {
            matrix.compute_needed()
        })
    }

}
