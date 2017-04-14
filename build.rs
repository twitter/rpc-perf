#[cfg(feature="git-version")]
extern crate git_build_version;
#[cfg(feature="git-version")]
const PACKAGE_TOP_DIR : &'static str = ".";

fn main() {
    #[cfg(feature="git-version")]
    git_build_version::write_version(PACKAGE_TOP_DIR).expect("Saving git version");
}
