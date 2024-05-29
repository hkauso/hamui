extern crate hamui;

use hamui::drawing::Creatable;
use hamui::*;
use std::io::{stdout, Write};

fn main() {
    let mut draw = |state: &mut State, buffer: &mut buffer::PseudoBuffer| {
        // TODO: fix this
        drawing::Text::new(buffer)
            .render("Hello, world!", (0, 0))
            .unwrap();
        buffer.to_owned()
    };

    let mut frame = Frame::new(stdout(), &mut draw);

    // enter env
    frame.open_env().unwrap();
    frame.flush().unwrap();

    // draw frame
    loop {
        frame.poll_events().unwrap();
        frame.step().unwrap();
    }
}
