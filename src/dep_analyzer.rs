use std::{collections::hash_map::Entry, io::Write};

use alpm::{AlpmList, Db, Dep, Package, PackageReason};
use rustc_hash::FxHashMap;

use crate::common::{DepHash, PkgIndex, PkgPtr, ProviderList};


pub fn print_unneeded(db: &Db, alpm_packages: AlpmList<&Package>) {
    
    let pkg_count = alpm_packages.len();

    let mut pkg_to_index: FxHashMap<PkgPtr, PkgIndex> = FxHashMap::default();
    pkg_to_index.reserve(pkg_count);
    // Use this structure for fast iteration
    let mut index_to_pkg: Vec<&Package> = Vec::with_capacity(pkg_count);
    // We do not know how many packages are provided by the local packages
    // TODO: estimate the size empirically based on the number of installed packages?
    let mut dep_to_providers: FxHashMap<DepHash, ProviderList>  = FxHashMap::default();
    // Pre-allocate the HashMap with a capacity chosen through empirical tests
    let mut dep_hash_to_pkg: FxHashMap<DepHash, PkgIndex> = FxHashMap::default();
    dep_hash_to_pkg.reserve(pkg_count);

    for pkg in alpm_packages {

        let index = PkgIndex(index_to_pkg.len());
        pkg_to_index.insert(PkgPtr::from(pkg), index);
        index_to_pkg.push(pkg);

        for provided in pkg.provides() {
            match dep_to_providers.entry(DepHash(provided.name_hash())) {
                Entry::Occupied(mut occupied_entry) => {
                    occupied_entry.get_mut().add_provider(index);
                    eprintln!("Package {} provides {}, which is already provided by package {}. Is it intentional?.", pkg.name(), provided.name(), index_to_pkg[occupied_entry.get().first().0].name());
                },
                Entry::Vacant(vacant_entry) => {
                    vacant_entry.insert(ProviderList::new(index));
                },
            }
        }
    }

    let mut discovered: Vec<bool> = vec![false; pkg_count];
    let mut stack: Vec<PkgIndex> = Vec::new();

    for (pkg_index, &pkg) in index_to_pkg.iter().enumerate() {

        // Skip already visited packages (we already know they are needed) and non-explicit packages
        if discovered[pkg_index] || matches!(pkg.reason(), PackageReason::Depend) {
            continue;
        }
        // In this branch, the package is explicitly installed and not visited yet
        discovered[pkg_index] = true;

        // Push its dependencies onto the stack for DFS
        push_dependencies(db, &pkg_to_index, &dep_to_providers, &mut dep_hash_to_pkg, &mut discovered, &mut stack, pkg);
        
        while let Some(pkg_index) = stack.pop() {
            push_dependencies(db, &pkg_to_index, &dep_to_providers, &mut dep_hash_to_pkg, &mut discovered, &mut stack, index_to_pkg[pkg_index.0]);
        }
    }

    // Disable printing when performing benchmarks and tests
    if !cfg!(test) {
        let mut stdout = std::io::stdout().lock();
        for (pkg_index, visited) in discovered.into_iter().enumerate() {
            if !visited {
                let pkg = index_to_pkg[pkg_index];
                writeln!(stdout, "{}", pkg.name()).unwrap();
            }
        }
    }
}


/// This function was written by stitching together lower-level operations done in the libalpm Rust bindings.
/// The goal was to avoid unnecessary reallocations and checks when looking up a package by name
fn get_pkg_from_dep<'a>(db: &'a Db, dep: &Dep) -> alpm::Result<&'a Package> {

    use alpm_sys::{alpm_depend_t, alpm_db_get_pkg, alpm_errno_t, alpm_errno, alpm_db_get_handle};

    unsafe {
        let name = (*std::mem::transmute::<&Dep, *const alpm_depend_t>(dep)).name;
        let pkg = alpm_db_get_pkg(db.as_ptr(), name);

        if pkg.is_null() {
            Err(std::mem::transmute::<alpm_errno_t, alpm::Error>(alpm_errno(alpm_db_get_handle(db.as_ptr()))))
        } else {
            Ok(&*(pkg as *mut Package))
        }
    }
}


fn push_dependencies(db: &Db, pkg_to_index: &FxHashMap<PkgPtr, PkgIndex>, dep_to_providers: &FxHashMap<DepHash, ProviderList>, dep_hash_to_pkg: &mut FxHashMap<DepHash, PkgIndex>, discovered: &mut [bool], stack: &mut Vec<PkgIndex>, pkg: &Package) {
    
    for hard_dep in pkg.depends() {
        // First check if there is a real package that satisfies this requirement. If not, expect the dependency to be provided as a virtual package
        if let Some(dep_pkg_index) = match dep_hash_to_pkg.entry(DepHash(hard_dep.name_hash())) {
            Entry::Occupied(occupied_entry) => {
                Some(*occupied_entry.get())
            },
            Entry::Vacant(vacant_entry) => {
                if let Ok(pkg) = get_pkg_from_dep(db, hard_dep) {
                    let pkg_index = *pkg_to_index.get(&PkgPtr::from(pkg)).unwrap();
                    vacant_entry.insert(pkg_index);
                    Some(pkg_index)
                } else {
                    None
                }
            },
        } {
            if !discovered[dep_pkg_index.0] {
                discovered[dep_pkg_index.0] = true;
                stack.push(dep_pkg_index);
            }
        } else {
            let providers = dep_to_providers.get(&DepHash(hard_dep.name_hash())).unwrap();

            for provider in providers.iter() {
                if !discovered[provider.0] {
                    discovered[provider.0] = true;
                    stack.push(*provider);
                }
            }
        }
    }

    for opt_dep in pkg.optdepends() {
        // First check if there is a real package that satisfies this requirement. If not, check if the dependency is provided as a virtual package
        if let Some(dep_pkg_index) = match dep_hash_to_pkg.entry(DepHash(opt_dep.name_hash())) {
            Entry::Occupied(occupied_entry) => {
                Some(*occupied_entry.get())
            },
            Entry::Vacant(vacant_entry) => {
                if let Ok(pkg) = get_pkg_from_dep(db, opt_dep) {
                    let pkg_index = *pkg_to_index.get(&PkgPtr::from(pkg)).unwrap();
                    vacant_entry.insert(pkg_index);
                    Some(pkg_index)
                } else {
                    None
                }
            },
        } {
            if !discovered[dep_pkg_index.0] {
                discovered[dep_pkg_index.0] = true;
                stack.push(dep_pkg_index);
            }           
        } else if let Some(providers) = dep_to_providers.get(&DepHash(opt_dep.name_hash())) {

            for provider in providers.iter() {
                if !discovered[provider.0] {
                    discovered[provider.0] = true;
                    stack.push(*provider);
                }
            }
        }
    }
}


#[cfg(test)]
mod tests {
    extern crate test;
    use crate::common::{get_packages, init_alpm};

    use super::*;
    use test::Bencher;


    #[bench]
    fn b_unneeded(b: &mut Bencher) {
        let alpm = init_alpm();
        let (db, packages) = get_packages(&alpm);
        b.iter(|| {
            print_unneeded(db, packages)
        })
    }

}
