extern crate hamui;

use hamui::*;
use std::io::{stdout, Write};

fn main() {
    let mut draw = |state: &mut State, buffer: &mut buffer::Buffer| {};

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
