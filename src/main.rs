#![feature(gen_blocks)]

use std::{collections::{HashMap, hash_map::Entry}, fs, io::{self, Write}, path::Path};

use alpm::{Alpm, AlpmList, Package};


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



/// A square matrix implemented as a flat array.
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


    pub fn print_unneeded_naive(&self) {
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

    dep_matrix.print_unneeded_naive();

}
