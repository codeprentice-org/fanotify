use fanotify::init::Flags;
use fanotify::init::Init;

pub mod driver;
pub mod supported;

pub type AnyResult<T = ()> = anyhow::Result<T>;

pub const fn get_init() -> Init {
    Init {
        flags: Flags::unlimited(),
        ..Init::const_default()
    }
}
