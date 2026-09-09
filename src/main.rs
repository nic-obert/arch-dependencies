#![feature(gen_blocks)]

use std::{collections::{HashMap, hash_map::Entry}, fs, io::{self, Write}, path::Path};

use alpm::{Alpm, AlpmList, Package, PackageReason};


const FS_ROOT: &str = "/";
const DB_PATH: &str = "/var/lib/pacman";


/// Wraps a package pointer
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
struct PkgPtr(pub *const Package);

impl From<&Package> for PkgPtr {
    fn from(pkg: &Package) -> Self {
        PkgPtr(pkg as *const _)
    }
}

impl Into<&Package> for PkgPtr {
    fn into(self) -> &'static Package {
        unsafe { &*(self.0 as *const Package) }
    }
}


/// Hash of the name of a dependency, used for mapping dependencies to their providers
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
struct DepHash(pub u64);


/// Index of a package in the dependency matrix
#[derive(Clone, Copy)]
struct PackageIndex(pub usize);


/// A node in a linked list of virtual package providers.
/// Most virtual packages only have one provider (on my system, there are 834 virtual packages and only about 90 have more than one provider)
/// Still, we cannot assume a strict maximum number of providers and we should not over-allocate for a worst-case scenario.
struct ProviderNode {
    pub pkg: PackageIndex,
    pub next: Option<Box<ProviderNode>>
}


/// A singly-linked list of provider nodes.
/// There is always at least one provider.
struct ProviderList {
    head: ProviderNode,
}

impl ProviderList {

    pub fn new(pkg: PackageIndex) -> Self {
        ProviderList {
            head: ProviderNode {
                pkg,
                next: None,
            }
        }
    }

    pub fn add_provider(&mut self, pkg: PackageIndex) {
        // Traverse the list instead of storing a tail pointer, since the number of providers is usually just one or two and we want to avoid over-allocating memory for a tail pointer.
        let mut current = &mut self.head;
        while let Some(ref mut next_node) = current.next {
            current = next_node;
        }
        current.next = Some(Box::new(ProviderNode {
            pkg,
            next: None,
        }));
    }

    pub fn iter(&self) -> impl Iterator<Item = &PackageIndex> {
        gen {
            let mut current = Some(&self.head);
            while let Some(node) = current {
                yield &node.pkg;
                current = node.next.as_deref();
            }
        }
    }

}



/// A dense square matrix implemented as a flat array.
/// Each row corresponds to a package, and each column corresponds to a package that it depends on.
/// Optional dependencies are also included in the matrix, but only if they are present in the local database.
struct DepMatrix<'a> {
    matrix: Vec<bool>,
    side: usize,
    index_to_pkg: Vec<&'a Package>,
    pkg_to_index: HashMap<PkgPtr, PackageIndex>,
    dep_to_providers: HashMap<DepHash, ProviderList>
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

            let hard_deps = pkg.depends();
            for hard_dep in hard_deps {
                // TODO: we may want to introduce a cache that maps a dependency name hash to a PackageIndex or &Package
                // It's not ideal to hash a string every time when we could hash an integer instead. Benchmark this
                // Also, calling the .pkg() function repeatedly may introduce significant overhead from allocations and checks. See function definition
                if let Ok(dep_pkg) = db_handle.pkg(hard_dep.name()) {
                    let column = pkg_to_index.get(&PkgPtr::from(dep_pkg)).unwrap().0;
                    matrix[row_base_index+column] = true;
                } else {
                    let providers = dep_to_providers.get(&DepHash(hard_dep.name_hash())).unwrap();
                    for provider in providers.iter() {
                        // TODO: filter providers by version constraints and architecture
                        let column = provider.0;
                        matrix[row_base_index+column] = true;
                    }
                }
            }

            let opt_deps = pkg.optdepends();
            for opt_dep in opt_deps {
                if let Ok(dep_pkg) = db_handle.pkg(opt_dep.name()) {
                    let column = pkg_to_index.get(&PkgPtr::from(dep_pkg)).unwrap().0;
                    matrix[row_base_index+column] = true;
                } else if let Some(providers) = dep_to_providers.get(&DepHash(opt_dep.name_hash())) {
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
            dep_to_providers
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


    pub fn print_unneeded_naive_by_columns(&self) {
        // Find unneeded packages, naive approach (should output the same as `pacman -Qt`)
        // A package is unneeded if its corresponding column in the matrix is all 0s
        // Iterate by columns and check one package at a time (bad for cache locality). Stop early if a 1 is reached
        for col in 0..self.side {
            let mut needed = false;
            for row in 0..self.side {
                if self.matrix[row*self.side+col] {
                    needed = true;
                    break;
                }
            }
            if !needed {
                println!("{}", self.index_to_pkg[col].name());
            }
        }
    }


    pub fn print_unneeded_naive_by_rows(&self) {
        // Find unneeded packages, optimized approach (should output the same as `pacman -Qt`)
        // A package is unneeded if its corresponding column in the matrix is all 0s
        // Iterate by rows and check all packages at once (better for cache locality)
        let mut needed = vec![false; self.side];
        for row in 0..self.side {
            let row_base_index = row*self.side;
            for col in 0..self.side {
                if self.matrix[row_base_index+col] {
                    needed[col] = true;
                }
            }
        }
        for col in 0..self.side {
            if !needed[col] {
                println!("{}", self.index_to_pkg[col].name());
            }
        }
    }


    pub fn print_all_dependencies_dfs(&self, package: &Package) {
        // Print all depencencies of the given package.
        let mut visited = vec![false; self.side];
        let mut stack = vec![self.pkg_to_index[&PkgPtr::from(package)]];

        while let Some(pkg_index) = stack.pop() {
            if visited[pkg_index.0] {
                continue;
            }
            visited[pkg_index.0] = true;

            let pkg = self.index_to_pkg[pkg_index.0];
            println!("{}", pkg.name());

            // Iterate over the row corresponding to the current package and push all packages that it depends on onto the stack
            let row_base_index = pkg_index.0*self.side;
            for col in 0..self.side {
                if self.matrix[row_base_index+col] {
                    stack.push(PackageIndex(col));
                }
            }
        }
    }


    pub fn print_all_dependents_dfs(&self, package: &Package) {
        // Print all packages that depend on the given package.
        let mut visited = vec![false; self.side];
        let mut stack = vec![self.pkg_to_index[&PkgPtr::from(package)]];

        while let Some(pkg_index) = stack.pop() {
            if visited[pkg_index.0] {
                continue;
            }
            visited[pkg_index.0] = true;

            let pkg = self.index_to_pkg[pkg_index.0];
            println!("{}", pkg.name());

            // Iterate over the column corresponding to the current package and push all packages that depend on it onto the stack
            for row in 0..self.side {
                if self.matrix[row*self.side+pkg_index.0] {
                    stack.push(PackageIndex(row));
                }
            }
        }
    }


    pub fn is_needed_by_explicit_dfs(&self, package: &Package) -> bool {
        // Check if the given package is either explicitly installed or needed by an explicitly installed package.

        // Short circuit for packages that are explicitly installed
        if matches!(package.reason(), PackageReason::Explicit) {
            return true;
        }

        let mut visited = vec![false; self.side];
        let mut stack = vec![self.pkg_to_index[&PkgPtr::from(package)]];

        while let Some(pkg_index) = stack.pop() {
            if visited[pkg_index.0] {
                continue;
            }
            visited[pkg_index.0] = true;

            // let pkg = self.index_to_pkg[pkg_index.0];
            // println!("Checking non-explicit {}", pkg.name());

            // Iterate over the column corresponding to the current package and push all packages that depend on it onto the stack
            for dependent_index in 0..self.side {
                // Only check dependents that have not been visited yet
                if self.matrix[dependent_index*self.side+pkg_index.0] && !visited[dependent_index] {
                    let dependent = self.index_to_pkg[dependent_index];
                    // println!("\tFound dependent {}", dependent.name());
                    match dependent.reason() {
                        // Short circuit if the dependent is explicitly installed
                        PackageReason::Explicit => {
                            // println!("\tFound explicit dependent {}", dependent.name());
                            return true;
                        },
                        PackageReason::Depend => {
                            if !visited[dependent_index] {
                                // println!("\tPushing dependent {}", dependent.name());
                                stack.push(PackageIndex(dependent_index));
                            }
                        },
                    }
                }
            }
        }

        false
    }


    pub fn compute_unneeded_packages_dfs(&self) {
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

        // Print all unneeded packages
        for (index, package) in self.index_to_pkg.iter().enumerate() {
            if !visited[index] {
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

}


fn main() {

    let alpm_handle = Alpm::new(FS_ROOT, DB_PATH).unwrap_or_else(
        |e| panic!("Failed to initialize ALPM: {}", e)
    );

    let db_handle = alpm_handle.localdb();

    if let Err(e) = db_handle.is_valid() {
        panic!("Error checking database validity: {}", e);
    }

    let packages = db_handle.pkgs();

    let dep_matrix = DepMatrix::from_alpm_packages(packages, db_handle);

    let edges = dep_matrix.count_edges();
    println!("Edges: {} / Total: {} / Density: {:.2}%", edges, dep_matrix.total_size(), edges as f64 / dep_matrix.total_size() as f64 * 100_f64);

    // dep_matrix.compute_unneeded_packages_dfs();
}
