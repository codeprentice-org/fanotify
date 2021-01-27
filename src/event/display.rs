use std::fmt;
use std::fmt::Display;
use std::fmt::Formatter;

use super::event::Event;

pub struct DisplayEvent<'a, 'b>(pub &'a Event<'b>);

impl<'a, 'b> Event<'b> {
    pub fn display(&'a self) -> DisplayEvent<'a, 'b> {
        DisplayEvent(self)
    }
}

impl Display for DisplayEvent<'_, '_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let event = self.0;
        write!(f, "{}, {:?}, {:?}", event.file().variant_name(), event.id().id(), event.mask())?;
        if let Some(path) = event.file().path() {
            write!(f, ": ")?;
            match path {
                Ok(path) => write!(f, "{}", path.display())?,
                Err(e) => write!(f, "{}", e)?,
            }
        }
        Ok(())
    }
}

pub struct DisplayEvents<'a, 'b>(pub &'a Vec<Event<'b>>);

impl Display for DisplayEvents<'_, '_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        writeln!(f, "[")?;
        for event in self.0 {
            writeln!(f, "    {},", event.display())?;
        }
        write!(f, "]")?;
        Ok(())
    }
}
