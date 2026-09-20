use std::collections::{HashMap, hash_map::Entry};

use alpm::{AlpmList, Db, Package, PackageReason};
use rustc_hash::FxHashMap;

use crate::common::{DepHash, PackageIndex, PkgPtr, ProviderList};


pub fn print_unneeded_stdhash(db: &Db, alpm_packages: AlpmList<&Package>) {
    let pkg_count = alpm_packages.len();

    let mut pkg_to_index: HashMap<PkgPtr, PackageIndex> = HashMap::with_capacity(pkg_count);
    // Use this structure for fast iteration
    let mut index_to_pkg: Vec<&Package> = Vec::with_capacity(pkg_count);
    // We do not know how many packages are provided by the local packages
    // TODO: estimate the size empirically based on the number of installed packages?
    let mut dep_to_providers: HashMap<DepHash, ProviderList>  = HashMap::new();
    // Pre-allocate the HashMap with a capacity chosen through empirical tests
    let mut dep_hash_to_pkg: HashMap<DepHash, PackageIndex> = HashMap::with_capacity(pkg_count);

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

    let mut visited: Vec<bool> = vec![false; pkg_count];
    let mut stack: Vec<PackageIndex> = Vec::new();

    for &pkg in &index_to_pkg {
        let pkg_index = pkg_to_index[&PkgPtr::from(pkg)];

        // Skip already visited packages (we already know they are needed) and non-explicit packages
        if visited[pkg_index.0] || matches!(pkg.reason(), PackageReason::Depend) {
            continue;
        }
        // In this branch, the package is explicitly installed and not visited yet
        visited[pkg_index.0] = true;

        // Push its dependencies onto the stack for DFS
        push_dependencies_stdhash(db, &pkg_to_index, &dep_to_providers, &mut dep_hash_to_pkg, &visited, &mut stack, pkg);
        
        while let Some(pkg_index) = stack.pop() {
            if visited[pkg_index.0] {
                continue;
            }
            visited[pkg_index.0] = true;

            push_dependencies_stdhash(db, &pkg_to_index, &dep_to_providers, &mut dep_hash_to_pkg, &visited, &mut stack, index_to_pkg[pkg_index.0]);
        }
    }

    if !cfg!(test) {
        for (pkg_index, visited) in visited.into_iter().enumerate() {
            if !visited {
                let pkg = index_to_pkg[pkg_index];
                println!("{}", pkg.name());
            }
        }
    }
}


pub fn print_unneeded_prealloc_stdhash(db: &Db, alpm_packages: AlpmList<&Package>) {
    let pkg_count = alpm_packages.len();

    let mut pkg_to_index: HashMap<PkgPtr, PackageIndex> = HashMap::with_capacity(pkg_count);
    // Use this structure for fast iteration
    let mut index_to_pkg: Vec<&Package> = Vec::with_capacity(pkg_count);
    // We do not know how many packages are provided by the local packages
    // TODO: estimate the size empirically based on the number of installed packages?
    let mut dep_to_providers: HashMap<DepHash, ProviderList>  = HashMap::new();
    // Pre-allocate the HashMap with a capacity chosen through empirical tests
    let mut dep_hash_to_pkg: HashMap<DepHash, PackageIndex> = HashMap::with_capacity(pkg_count);

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

    let mut visited: Vec<bool> = vec![false; pkg_count];
    let mut stack: Vec<PackageIndex> = Vec::new();

    for &pkg in &index_to_pkg {
        let pkg_index = pkg_to_index[&PkgPtr::from(pkg)];

        // Skip already visited packages (we already know they are needed) and non-explicit packages
        if visited[pkg_index.0] || matches!(pkg.reason(), PackageReason::Depend) {
            continue;
        }
        // In this branch, the package is explicitly installed and not visited yet
        visited[pkg_index.0] = true;

        // Push its dependencies onto the stack for DFS
        push_dependencies_stdhash(db, &pkg_to_index, &dep_to_providers, &mut dep_hash_to_pkg, &visited, &mut stack, pkg);
        
        while let Some(pkg_index) = stack.pop() {
            if visited[pkg_index.0] {
                continue;
            }
            visited[pkg_index.0] = true;

            push_dependencies_stdhash(db, &pkg_to_index, &dep_to_providers, &mut dep_hash_to_pkg, &visited, &mut stack, index_to_pkg[pkg_index.0]);
        }
    }

    if !cfg!(test) {
        for (pkg_index, visited) in visited.into_iter().enumerate() {
            if !visited {
                let pkg = index_to_pkg[pkg_index];
                println!("{}", pkg.name());
            }
        }
    }
}


pub fn print_unneeded_prealloc_fxhash(db: &Db, alpm_packages: AlpmList<&Package>) {
    let pkg_count = alpm_packages.len();

    let mut pkg_to_index: FxHashMap<PkgPtr, PackageIndex> = FxHashMap::default();
    pkg_to_index.reserve(pkg_count);
    // Use this structure for fast iteration
    let mut index_to_pkg: Vec<&Package> = Vec::with_capacity(pkg_count);
    // We do not know how many packages are provided by the local packages
    // TODO: estimate the size empirically based on the number of installed packages?
    let mut dep_to_providers: FxHashMap<DepHash, ProviderList>  = FxHashMap::default();
    // Pre-allocate the HashMap with a capacity chosen through empirical tests
    let mut dep_hash_to_pkg: FxHashMap<DepHash, PackageIndex> = FxHashMap::default();
    dep_hash_to_pkg.reserve(pkg_count);

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

    let mut visited: Vec<bool> = vec![false; pkg_count];
    let mut stack: Vec<PackageIndex> = Vec::new();

    for &pkg in &index_to_pkg {
        let pkg_index = pkg_to_index[&PkgPtr::from(pkg)];

        // Skip already visited packages (we already know they are needed) and non-explicit packages
        if visited[pkg_index.0] || matches!(pkg.reason(), PackageReason::Depend) {
            continue;
        }
        // In this branch, the package is explicitly installed and not visited yet
        visited[pkg_index.0] = true;

        // Push its dependencies onto the stack for DFS
        push_dependencies_fxhash(db, &pkg_to_index, &dep_to_providers, &mut dep_hash_to_pkg, &visited, &mut stack, pkg);
        
        while let Some(pkg_index) = stack.pop() {
            if visited[pkg_index.0] {
                continue;
            }
            visited[pkg_index.0] = true;

            push_dependencies_fxhash(db, &pkg_to_index, &dep_to_providers, &mut dep_hash_to_pkg, &visited, &mut stack, index_to_pkg[pkg_index.0]);
        }
    }

    if !cfg!(test) {
        for (pkg_index, visited) in visited.into_iter().enumerate() {
            if !visited {
                let pkg = index_to_pkg[pkg_index];
                println!("{}", pkg.name());
            }
        }
    }
}


pub fn print_unneeded_fxhash(db: &Db, alpm_packages: AlpmList<&Package>) {
    
    let pkg_count = alpm_packages.len();

    let mut pkg_to_index: FxHashMap<PkgPtr, PackageIndex> = FxHashMap::default();
    pkg_to_index.reserve(pkg_count);

    // Use this structure for fast iteration
    let mut index_to_pkg: Vec<&Package> = Vec::with_capacity(pkg_count);

    // We do not know how many packages are provided by the local packages
    let mut dep_to_providers: FxHashMap<DepHash, ProviderList>  = FxHashMap::default();

    let mut dep_hash_to_pkg: FxHashMap<DepHash, PackageIndex> = FxHashMap::default();

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

    let mut visited: Vec<bool> = vec![false; pkg_count];
    let mut stack: Vec<PackageIndex> = Vec::new();

    for &pkg in &index_to_pkg {
        let pkg_index = pkg_to_index[&PkgPtr::from(pkg)];

        // Skip already visited packages (we already know they are needed) and non-explicit packages
        if visited[pkg_index.0] || matches!(pkg.reason(), PackageReason::Depend) {
            continue;
        }
        // In this branch, the package is explicitly installed and not visited yet
        visited[pkg_index.0] = true;

        // Push its dependencies onto the stack for DFS
        push_dependencies_fxhash(db, &pkg_to_index, &dep_to_providers, &mut dep_hash_to_pkg, &visited, &mut stack, pkg);
        
        while let Some(pkg_index) = stack.pop() {
            if visited[pkg_index.0] {
                continue;
            }
            visited[pkg_index.0] = true;

            push_dependencies_fxhash(db, &pkg_to_index, &dep_to_providers, &mut dep_hash_to_pkg, &visited, &mut stack, index_to_pkg[pkg_index.0]);
        }
    }

    if !cfg!(test) {
        for (pkg_index, visited) in visited.into_iter().enumerate() {
            if !visited {
                let pkg = index_to_pkg[pkg_index];
                println!("{}", pkg.name());
            }
        }
    }
}


fn push_dependencies_stdhash(db: &Db, pkg_to_index: &HashMap<PkgPtr, PackageIndex>, dep_to_providers: &HashMap<DepHash, ProviderList>, dep_hash_to_pkg: &mut HashMap<DepHash, PackageIndex>, visited: &Vec<bool>, stack: &mut Vec<PackageIndex>, pkg: &Package) {
    for hard_dep in pkg.depends() {
        // First check if there is a real package that satisfies this requirement. If not, expect the dependency to be provided as a virtual package
        if let Some(dep_pkg_index) = match dep_hash_to_pkg.entry(DepHash(hard_dep.name_hash())) {
            Entry::Occupied(occupied_entry) => {
                Some(*occupied_entry.get())
            },
            Entry::Vacant(vacant_entry) => {
                if let Ok(pkg) = db.pkg(hard_dep.name()) {
                    let pkg_ptr = PkgPtr::from(pkg);
                    let pkg_index = *pkg_to_index.get(&pkg_ptr).unwrap();
                    vacant_entry.insert(pkg_index);
                    Some(pkg_index)
                } else {
                    None
                }
            },
        } {
            if !visited[dep_pkg_index.0] {
                stack.push(dep_pkg_index);
            }
        } else {
            let providers = dep_to_providers.get(&DepHash(hard_dep.name_hash())).unwrap();

            // Debug:
            // if providers.iter().count() > 1 {
            //     println!("Hard dependency `{}` has multiple providers: {:?}", hard_dep.name(), providers.iter().map(|p| index_to_pkg[p.0].name()).collect::<Vec<_>>());
            // }

            for provider in providers.iter() {
                // TODO: filter providers by version constraints and architecture
                if !visited[provider.0] {
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
                if let Ok(pkg) = db.pkg(opt_dep.name()) {
                    let pkg_ptr = PkgPtr::from(pkg);
                    let pkg_index = *pkg_to_index.get(&pkg_ptr).unwrap();
                    vacant_entry.insert(pkg_index);
                    Some(pkg_index)
                } else {
                    None
                }
            },
        } {
            if !visited[dep_pkg_index.0] {
                stack.push(dep_pkg_index);
            }           
        } else if let Some(providers) = dep_to_providers.get(&DepHash(opt_dep.name_hash())) {

            // Debug:
            // if providers.iter().count() > 1 {
            //     println!("Optional dependency `{}` has multiple providers: {:?}", opt_dep.name(), providers.iter().map(|p| index_to_pkg[p.0].name()).collect::<Vec<_>>());
            // }

            for provider in providers.iter() {
                // TODO: filter providers by version constraints and architecture
                if !visited[provider.0] {
                    stack.push(*provider);
                }
            }
        }
    }
}


fn push_dependencies_fxhash(db: &Db, pkg_to_index: &FxHashMap<PkgPtr, PackageIndex>, dep_to_providers: &FxHashMap<DepHash, ProviderList>, dep_hash_to_pkg: &mut FxHashMap<DepHash, PackageIndex>, visited: &Vec<bool>, stack: &mut Vec<PackageIndex>, pkg: &Package) {
    for hard_dep in pkg.depends() {
        // First check if there is a real package that satisfies this requirement. If not, expect the dependency to be provided as a virtual package
        if let Some(dep_pkg_index) = match dep_hash_to_pkg.entry(DepHash(hard_dep.name_hash())) {
            Entry::Occupied(occupied_entry) => {
                Some(*occupied_entry.get())
            },
            Entry::Vacant(vacant_entry) => {
                // TODO: Here we convert a C const char* into a Rust &str only to copy it into a CString and convert it back to a C const char*
                // We should avoid all these useless conversions at the language boundary
                if let Ok(pkg) = db.pkg(hard_dep.name()) {
                    let pkg_ptr = PkgPtr::from(pkg);
                    let pkg_index = *pkg_to_index.get(&pkg_ptr).unwrap();
                    vacant_entry.insert(pkg_index);
                    Some(pkg_index)
                } else {
                    None
                }
            },
        } {
            if !visited[dep_pkg_index.0] {
                stack.push(dep_pkg_index);
            }
        } else {
            let providers = dep_to_providers.get(&DepHash(hard_dep.name_hash())).unwrap();

            // Debug:
            // if providers.iter().count() > 1 {
            //     println!("Hard dependency `{}` has multiple providers: {:?}", hard_dep.name(), providers.iter().map(|p| index_to_pkg[p.0].name()).collect::<Vec<_>>());
            // }

            for provider in providers.iter() {
                // TODO: filter providers by version constraints and architecture
                if !visited[provider.0] {
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
                if let Ok(pkg) = db.pkg(opt_dep.name()) {
                    let pkg_ptr = PkgPtr::from(pkg);
                    let pkg_index = *pkg_to_index.get(&pkg_ptr).unwrap();
                    vacant_entry.insert(pkg_index);
                    Some(pkg_index)
                } else {
                    None
                }
            },
        } {
            if !visited[dep_pkg_index.0] {
                stack.push(dep_pkg_index);
            }           
        } else if let Some(providers) = dep_to_providers.get(&DepHash(opt_dep.name_hash())) {

            // Debug:
            // if providers.iter().count() > 1 {
            //     println!("Optional dependency `{}` has multiple providers: {:?}", opt_dep.name(), providers.iter().map(|p| index_to_pkg[p.0].name()).collect::<Vec<_>>());
            // }

            for provider in providers.iter() {
                // TODO: filter providers by version constraints and architecture
                if !visited[provider.0] {
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
    fn b_unneeded_prealloc_stdhash(b: &mut Bencher) {
        let alpm = init_alpm();
        let (db, packages) = get_packages(&alpm);
        b.iter(|| {
            print_unneeded_prealloc_stdhash(db, packages)
        })
    }


    #[bench]
    fn b_unneeded_stdhash(b: &mut Bencher) {
        let alpm = init_alpm();
        let (db, packages) = get_packages(&alpm);
        b.iter(|| {
            print_unneeded_stdhash(db, packages)
        })
    }


    #[bench]
    fn b_unneeded_fxhash(b: &mut Bencher) {
        let alpm = init_alpm();
        let (db, packages) = get_packages(&alpm);
        b.iter(|| {
            print_unneeded_fxhash(db, packages)
        })
    }


    #[bench]
    fn b_unneeded_prealloc_fxhash(b: &mut Bencher) {
        let alpm = init_alpm();
        let (db, packages) = get_packages(&alpm);
        b.iter(|| {
            print_unneeded_prealloc_fxhash(db, packages)
        })
    }

}
