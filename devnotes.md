# Dependency Graph Analyzer

Create a system-wide dependency graph to find unneeded packages and circular dependencies.
Motivation: pacman and pactree cannot find unneeded circular dependencies

## How to 

Use alpm to interface with the package database. The alpm crate provides rust bindings for libalpm

Once we obtain all package data, create a directed graph structure 

Perform a full graph walk to find cycles between packages installed as dependencies.
I may need to study some graph theory and algorithms.


# Notes 

Consider converting the dependency graph to a matrix for faster manipulation
Since we know the exact size of the package list, we could pre-allocate the whole matrix
We can convert the package pointer into an index to be used in the matrix, assuming the alpm_pkg_t never gets moved

Treat optional dependencies as regular dependencies. For instance, we don't want to remove libnotify because it's just an optional dependency.
The dependency matrix only needs to store a binary condition: given package A and package B, either A directly depends on B (hard or optional) or A does not depend on B.
This means that the dependency matrix could be compressed by using bit fields.

First implement the matrix using no bitfield compression. Apply this optimization only after the program is working correctly and perform benchamrks to see if there are actual benefits.

Consider which configuration (columns vs rows) is better for cache locality.

Note: we assume that all hard dependencies are installed on the system
We shoule not assume optional dependencies are installed


Alpm stores dependencies with an associated name hash (u64)

TODO: Consider using specialized integer hash functions for improved hash speeds. Benchmarks needed

Potential limitation: if both packages A and B provide a virtual package C and a package D depends on C, then D may be listed as depending on both A and B instead of depending on only one of them. In some cases, it's best to leave it be. For instance, both vulkan-intel and nvidia-utils provide a vulkan-driver virtual package. We don't want to remove one of them because different graphics processors on the same system may use different drivers.

TODO: Consider parallelizing some work with rayon
