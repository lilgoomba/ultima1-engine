//! The start-up programs (the title sequence and the character program)
//! as the front end runs them: a display chosen in the options menu on
//! their screens stops the program (a [`Halt`] the front end reports
//! through [`Io::display_restart`]) and it starts again in that display.
//! The original has no such menu; nothing either program shows changes.

use crate::gamedir::{GameDir, VideoMode};
use crate::gen::{Gen, GenEnd, GenFiles};
use crate::io::{Halt, Io, IoResult};
use crate::term::Term;
use crate::title::{self, TitleFiles};
use std::path::Path;

/// The title sequence until it hands over to the character program
/// (`None`), or the screen it was stopped on.
pub fn intro(g: &GameDir, mode: &mut VideoMode, io: &mut dyn Io) -> crate::Result<Option<Term>> {
    let mut f = TitleFiles::load(g, *mode)?;
    loop {
        let mut t = title::blank_term(&f);
        match title::run_intro(&f, &mut t, io) {
            Ok(_) => return Ok(None),
            Err(Halt) => match io.display_restart() {
                Some(m) => match TitleFiles::load(g, m) {
                    Ok(nf) => {
                        f = nf;
                        *mode = m;
                    }
                    // Kept in the display it had.
                    Err(_) => io.video_shown(*mode),
                },
                None => return Ok(Some(t)),
            },
        }
    }
}

/// The character program from its first menu; `clock` makes each run's
/// clock. Returns the last screen and how the program ended.
pub fn character(g: &GameDir, dir: &Path, mode: &mut VideoMode, clock: &mut dyn FnMut() -> Box<dyn FnMut() -> [u8; 4]>, io: &mut dyn Io) -> crate::Result<(Term, IoResult<GenEnd>)> {
    let mut f = GenFiles::load(g, *mode)?;
    loop {
        let mut gen = Gen::new(&f, dir, clock());
        let mut t = Term::with_cga(*mode == VideoMode::Cga);
        let r = gen.run(&mut t, io);
        drop(gen);
        match r {
            Err(Halt) => match io.display_restart() {
                Some(m) => match GenFiles::load(g, m) {
                    Ok(nf) => {
                        f = nf;
                        *mode = m;
                    }
                    Err(_) => io.video_shown(*mode),
                },
                None => return Ok((t, Err(Halt))),
            },
            ok => return Ok((t, ok)),
        }
    }
}
