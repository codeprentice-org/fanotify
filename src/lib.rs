pub mod flags;
pub mod descriptor;
mod util;

#[cfg(test)]
mod tests {
    use crate::descriptor::{FanotifyInit, FanotifyError};

    #[test]
    fn it_works() {
        let args = FanotifyInit {
            ..Default::default()
        };
        match args.run() {
            Ok(fd) => {

            },
            Err(e) => {
                assert_eq!(e, FanotifyError::Unsupported);
            }
        }
        assert_eq!(2 + 2, 4);
    }
}
