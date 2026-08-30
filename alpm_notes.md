

# Useful functions


alpm_handle_t * alpm_initialize (const char * root, const char * dbpath, alpm_errno_t * err)
     Initializes the library. Creates handle, connects to database and creates lockfile. This must be called before any other functions
     are called.

     Parameters
         root the root path for all filesystem operations
         dbpath the absolute path to the libalpm database
         err an optional variable to hold any error return codes

     Returns
         a context handle on success, NULL on error, err will be set if provided



const char * alpm_strerror (alpm_errno_t err)
     Returns the string corresponding to an error number.

     Parameters
         err the error code to get the string for

     Returns
         the string relating to the given error code


int alpm_release (alpm_handle_t * handle)
     Release  the library. Disconnects from the database, removes handle and lockfile This should be the last alpm call you make. After
     this returns, handle should be considered invalid and cannot be reused in any way.

     Parameters
         handle the context handle

     Returns
         0 on success, -1 on error


alpm_db_t * alpm_get_localdb (alpm_handle_t * handle)
     Get the database of locally installed packages. The returned pointer points to an internal structure of libalpm which should  only
     be manipulated through libalpm functions.

     Returns
         a reference to the local database


alpm_handle_t * alpm_db_get_handle (alpm_db_t * db)
     Get the handle of a package database.

     Parameters
         db pointer to the package database

     Returns
         the alpm handle that the package database belongs to


int alpm_db_get_valid (alpm_db_t * db)
     Check the validity of a database. This is most useful for sync databases and verifying signature status. If  invalid,  the  handle
     error code will be set accordingly.

     Parameters
         db pointer to the package database

     Returns
         0 if valid, -1 if invalid (pm_errno is set accordingly)


alpm_pkg_t * alpm_db_get_pkg (alpm_db_t * db, const char * name)
     Get a package entry from a package database. Looking up a package is O(1) and will be significantly faster than iterating over the
     pkgcahe.

     Parameters
         db pointer to the package database to get the package from
         name of the package

     Returns
         the package entry on success, NULL on error


alpm_list_t * alpm_db_get_pkgcache (alpm_db_t * db)
     Get the package cache of a package database. This is a list of all packages the db contains.

     Parameters
         db pointer to the package database to get the package from

     Returns
         the list of packages on success, NULL on error


alpm_list_t * alpm_pkg_compute_requiredby (alpm_pkg_t * pkg)
     Computes  the  list  of packages requiring a given package. The return value of this function is a newly allocated list of package
     names (char*), it should be freed by the caller.

     Parameters
         pkg a package

     Returns
         the list of packages requiring pkg


alpm_pkg_t * alpm_pkg_find (alpm_list_t * haystack, const char * needle)
     Find a package in a list by name.

     Parameters
         haystack a list of alpm_pkg_t
         needle the package name

     Returns
         a pointer to the package if found or NULL


alpm_list_t * alpm_pkg_get_depends (alpm_pkg_t * pkg)
     Returns the list of package dependencies as alpm_depend_t.

     Parameters
         pkg a pointer to package

     Returns
         a reference to an internal list of alpm_depend_t structures.


alpm_time_t alpm_pkg_get_installdate (alpm_pkg_t * pkg)
     Returns the install timestamp of the package.

     Parameters
         pkg a pointer to package

     Returns
         the timestamp of the install time


off_t alpm_pkg_get_isize (alpm_pkg_t * pkg)
     Returns the installed size of the package.

     Parameters
         pkg a pointer to package

     Returns
         the total size of files installed by the package.


const char * alpm_pkg_get_name (alpm_pkg_t * pkg)
     Returns the package name.

     Parameters
         pkg a pointer to package

     Returns
         a reference to an internal string


alpm_list_t * alpm_pkg_get_optdepends (alpm_pkg_t * pkg)
     Returns the list of package optional dependencies.

     Parameters
         pkg a pointer to package

     Returns
         a reference to an internal list of alpm_depend_t structures.


alpm_list_t * alpm_pkg_get_provides (alpm_pkg_t * pkg)
     Returns the list of packages provided by pkg.

     Parameters
         pkg a pointer to package

     Returns
         a reference to an internal list of alpm_depend_t structures.


alpm_pkgreason_t alpm_pkg_get_reason (alpm_pkg_t * pkg)
     Returns the package installation reason.

     Parameters
         pkg a pointer to package

     Returns
         an enum member giving the install reason.



# Concept


static alpm_errno_t err;


alpm_handle = alpm_initialize(fs_root, db_path, &err)
if (!alpm_handle)
	string = alpm_strerror(err)
	return

db_handle = alpm_get_localdb(alpm_handle)

success = alpm_db_get_valid(db_handle)
if (!success)
	log error
	cleanup

pkg_list = alpm_db_get_pkgcache(db_handle)

success = alpm_release(alpm_handle)
if (!success)
	log error

