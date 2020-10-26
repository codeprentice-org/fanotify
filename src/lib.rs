pub mod flags;
pub mod descriptor;
mod util;

#[cfg(test)]
mod tests {
    use crate::flags::init::Init;
    use crate::descriptor::InitError;

    #[test]
    fn it_works() {
        let args = Init {
            ..Default::default()
        };
        match args.run() {
            Ok(_fd) => {}
            Err(e) => {
                assert_eq!(e, InitError::Unsupported);
            }
        }
        assert_eq!(2 + 2, 4);
    }
}
