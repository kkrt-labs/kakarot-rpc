/// Macro to find the root path of the project.
///
/// This macro utilizes the `find_project_root_path` function from the `utils` module.
/// This function works by identifying the root directory of the current git repository.
/// It starts at the current working directory and traverses up the directory tree until it finds a
/// directory containing a `.git` folder. If no such directory is found, it uses the current
/// directory as the root.
///
/// After determining the project root, the macro creates a new path by joining the given relative
/// path with the found project root path.
///
/// The relative path must be specified as a string literal argument to the macro.
///
/// # Examples
///
/// ```ignore
/// let full_path = root_project_path!("src/main.rs");
/// println!("Full path to main.rs: {:?}", full_path);
/// ```
///
/// # Panics
///
/// This macro will panic if it fails to find the root path of the project or if the root path
/// cannot be represented as a UTF-8 string.
#[macro_export]
macro_rules! root_project_path {
    ($relative_path:expr) => {{
        find_project_root_path(None).expect("Failed to find project root").join(Path::new(&$relative_path))
    }};
}
