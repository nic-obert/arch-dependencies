use alpm::Package;


/// Wraps a package pointer
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct PkgPtr(pub *const Package);

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
pub struct DepHash(pub u64);


/// Index of a package in the dependency matrix
#[derive(Clone, Copy)]
pub struct PackageIndex(pub usize);


/// A node in a linked list of virtual package providers.
/// Most virtual packages only have one provider (on my system, there are 834 virtual packages and only about 90 have more than one provider)
/// Still, we cannot assume a strict maximum number of providers and we should not over-allocate for a worst-case scenario.
struct ProviderNode {
    pub pkg: PackageIndex,
    pub next: Option<Box<ProviderNode>>
}


/// A singly-linked list of provider nodes.
/// There is always at least one provider.
pub struct ProviderList {
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
