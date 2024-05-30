extern crate hamui;

use hamui::drawing::{Component, Creatable, RectBoundary};
use hamui::*;
use std::io::{stdout, Write};

fn main() {
    let mut draw = |state: &mut State, mut buffer: buffer::PseudoBuffer| {
        buffer.set_changes(
            drawing::Text::new(buffer.clone())
                .render("Hello, world!", (0, 0))
                .unwrap()
                .1,
        );

        buffer.set_changes(
            drawing::QuickBox::new(buffer.clone())
                .render(
                    state.window_size,
                    RectBoundary {
                        pos: (10, 5),
                        size: (12, 5),
                    },
                )
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
        frame.step().unwrap();
    }
}
