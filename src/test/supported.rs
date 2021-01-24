use apply::Apply;
use semver::Version;

#[derive(Debug, Eq, PartialEq)]
enum Supported {
    None,
    Partial,
    Full,
}

impl Default for Supported {
    fn default() -> Self {
        Self::None
    }
}

impl Supported {
    pub fn get() -> Self {
        let uname = nix::sys::utsname::uname();
        if uname.sysname() != "Linux" {
            return Default::default();
        }
        uname.release()
            .apply(Version::parse)
            .map(|version| {
                if version >= Version::new(5, 1, 0) {
                    Self::Full
                } else if version >= Version::new(4, 19, 0) {
                    Self::Partial
                } else {
                    Self::None
                }
            }).unwrap_or_default()
    }
}
