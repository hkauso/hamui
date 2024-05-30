extern crate hamui;

use hamui::drawing::Creatable;
use hamui::*;
use std::io::{stdout, Write};

fn main() {
    let mut update_needed: bool = true;

    let mut draw = |_state: &mut State, mut buffer: buffer::PseudoBuffer| {
        buffer.set_changes(
            drawing::Text::new(buffer.clone())
                .render("Hello, world!", (0, 0))
                .unwrap()
                .1,
        );

        buffer.to_owned()
    };

    let mut frame = Frame::new(stdout(), &mut draw);

    // enter env
    frame.open_env().unwrap();
    frame.flush().unwrap();

    // draw frame
    loop {
        frame.poll_events().unwrap();

        if update_needed == false {
            // if we don't do some sort of check here, the cursor will always be moving
            // and mouse input will not work ... only update when needed!
            frame.step_no_draw().unwrap();
            continue;
        }

        update_needed = false; // this would get set to true if something happened in that app that made an update be needed
        frame.step().unwrap();
    }
}
