//! Who owns the plugin directory, and therefore what setup may do with it.

/// The four states the plugin directory can be in when setup starts.
///
/// Two booleans carried this: `CatalogInstall` and `RemoveStaleSymlink`. Between them
/// they admitted a combination that cannot exist (a catalog install that also has a
/// stale symlink to remove) and they had no way to say the third thing that actually
/// happens - the path exists but could not be inspected. `canonicalize` failing for a
/// reason other than "not there" read as "not a catalog install", which is the one
/// answer that lets setup write into a directory it could not identify.
///
/// As a value each case is named, and the report-only cases cannot be confused with
/// the writable ones.
pub(crate) enum Enum {
	/// Nothing is there: setup creates the directory. The fresh-install case.
	Dev,
	/// A symlink from an older install (plugin dir -> runtime home). It has to be
	/// removed before the loader can land as real files.
	DevStaleSymlink,
	/// A real directory under the Hermes home, placed there by
	/// `download.sh`/`download.ps1`: Hermes owns it and setup is report-only.
	Catalog,
	/// Something is there and could not be classified. Report-only, because writing
	/// into a directory we could not identify is the one mistake setup must not make.
	Uninspectable {
		/// Why the path could not be inspected.
		Cause:String,
	},
}
