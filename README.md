# Dependency Graph Analyzer

This program analyzes the dependency graph of all locally installed packages to determine which packages are currently not needed (and can be safely removed).
This program is able to detect unneeded circular dependencies, which `pacman -Qdt` cannot detect.

A package is determined to be needed if any of the following conditions are met:
* the package is marked as explicitly installed
* the package is (optionally) required by a needed package

Any package that is not marked as needed, is treated as unneeded.

> Note 1: optional dependencies are treated as regular dependencies. This means that, let `A` be a needed package, if `A` optionally depends on `B`, `B` is also marked as needed. This behavior is intended, as you don't want to accidentally remove optional functionality users may rely on. It's up to the system maintainer to choose whether an optional dependency is really needed.

> Note 2: this program currently does not filter virtual package providers by architecture and version. This means that, let `A` be a needed virtual package, all its providers will be marked as needed. This is usually not a problem.
 
> Note 3: This program does not check for misconfigured package databases. If the user actually relies on a package that is not marked as explicitly installed, this program may flag it as unneeded. It's up to the system maintainer to ensure the package database is configured correctly. Always double-check before removing any package.

## Usage

Just run the program without any arguments

Run with Cargo:
```bash
cargo run --release
```

Run the executable directly:
```bash
/path/to/exe
```

Unneeded packages will be printed to the console, separated by newlines.

You can print a help page with the `--help` flag:
```bash
cargo run --release -- --help
```
Or
```bash
/path/to/exe --help
```

This program does not make any changes to the system: it only lists unneeded packages. It's up to the system maintainer to remove them and check for any mistakes.

You can feed this program's output to `pacman` to automatically remove unneeded packages:
```bash
sudo pacman -R $(/path/to/exe)
```

## How it works (overview)

This program currently performs a DFS search on the dependency graph, starting from all explicitly installed packages and marking all their dependencies as needed. A package is determined to be unneeded if it wasn't visited duing the search.

Caching and short-circuiting are applied where needed to avoid duplicated work. 

### Previous approaches

An older version of this program would build a dense dependency matrix from the Alpm database as an intermediate representation. It would then apply the same DFS algorithm to the matrix.

This approach was determined to be slower in benchmarks.