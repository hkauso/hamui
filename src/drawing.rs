//! Components
use crossterm::cursor;
use crossterm::QueueableCommand;
use std::io::{StdoutLock, Write};

use crate::State;

// traits
pub trait Component {
    fn render(&mut self, window_size: (u16, u16), rect: RectBoundary) -> DrawingResult;
}

/// Component can be created with "::new()"
pub trait Creatable {
    fn new(stdout: StdoutLock<'static>) -> Self;
}

/// Component can be clicked
pub trait Clickable {
    /// Handle a click event on the element
    fn on_click(
        &mut self,
        res: RectBoundary,
        state: State,
        run: &mut dyn FnMut(State) -> State,
    ) -> State {
        if check_click(&state, res) {
            return run(state);
        }

        state
    }
}

// types
pub type Vec2 = (u16, u16);
pub type DrawingResult = Result<RectBoundary, std::io::Error>;

#[derive(Clone, Debug)]
pub struct RectBoundary {
    pub pos: Vec2,
    pub size: Vec2,
}

// utility
/// Get the center of the screen based on the size of a box
pub fn get_center(window_size: (u16, u16), size: (u16, u16)) -> (u16, u16) {
    let (w, h) = window_size;
    let (x, y) = size;

    ((w / 2) - (x / 2), (h / 2) - (y / 2))
}

/// Check if a click was inside of a target position and size
pub fn check_click(state: &State, res: RectBoundary) -> bool {
    let (x, y) = state.clicked;

    let range_x = res.pos.0..(res.pos.0 + res.size.0);
    let range_y = res.pos.1..(res.pos.1 + res.size.1);

    if !range_x.contains(&x) | !range_y.contains(&y) {
        return false;
    }

    return true;
}

// line
pub struct DownwardsLine {
    pub stdout: StdoutLock<'static>,
    pub rect: RectBoundary,
}

impl DownwardsLine {
    /// Draw a line going down
    ///
    /// ## Arguments:
    /// * `stdout`
    /// * `height`
    /// * `start` - x, y
    /// * `char` - line character
    /// * `end_char` - line character at the end of the line (for corners)
    pub fn new(
        stdout: &mut StdoutLock<'static>,
        height: u16,
        start: Vec2,
        char: &str,
        end_char: &str,
    ) -> RectBoundary {
        for i in 0..height {
            stdout.queue(cursor::MoveTo(start.0, start.1 + i)).unwrap();

            if i == height - 1 {
                stdout.write(end_char.as_bytes()).unwrap();
                break;
            }

            stdout.write(char.as_bytes()).unwrap();
        }

        // return
        RectBoundary {
            pos: start,
            size: (1, height),
        }
    }
}

// box
pub struct QuickBox {
    pub stdout: StdoutLock<'static>,
}

impl Creatable for QuickBox {
    fn new(stdout: StdoutLock<'static>) -> Self {
        QuickBox { stdout }
    }
}

impl Component for QuickBox {
    /// Draw a box
    ///
    /// ## Arguments:
    /// * `stdout`
    /// * `pos` - x, y
    /// * `size` - x, y
    fn render(&mut self, window_size: (u16, u16), rect: RectBoundary) -> DrawingResult {
        let pos = rect.pos;
        let mut size = rect.size;

        // clear space
        let len_x = size.0; // how far we should write whitespace characters
        let range_y = pos.1..(pos.1 + size.1); // how many lines we should write whitespace characters in

        for row in range_y {
            self.stdout.queue(cursor::MoveTo(pos.0, row))?;
            self.stdout
                .write(" ".repeat(len_x as usize).to_string().as_bytes())?;
        }

        // move cursor
        self.stdout.queue(cursor::MoveTo(pos.0, pos.1))?;

        // make sure size isn't too big
        // if size.0 >= window_size.0 {

        //     size.0 -= window_size.0 - 5;
        // }

        // auto resize (y)
        if size.1 >= window_size.1 {
            size.1 -= size.1 - window_size.1;
        }

        // draw line
        let line_top = format!("╭{}╮", "─".repeat((size.0 - 2) as usize));
        let line_bottom = "─".repeat((size.0 - 2) as usize);

        // write
        self.stdout.write(line_top.as_bytes())?; // top

        DownwardsLine::new(&mut self.stdout, size.1, (pos.0, pos.1 + 1), "│", "╰"); // left
        DownwardsLine::new(
            // right
            &mut self.stdout,
            size.1,
            (pos.0 + size.0 - 1, pos.1 + 1),
            "│",
            "╯",
        );

        self.stdout
            .queue(cursor::MoveTo(pos.0 + 1, pos.1 + size.1))?;
        self.stdout.write(line_bottom.as_bytes())?; // bottom

        // done
        self.stdout.queue(cursor::MoveTo(0, 0))?;
        Ok(RectBoundary { pos, size })
    }
}

// text
pub struct Text {
    pub stdout: StdoutLock<'static>,
}

impl Creatable for Text {
    fn new(stdout: StdoutLock<'static>) -> Self {
        Text { stdout }
    }
}

impl Text {
    /// Draw text at the center of a given [`Vec2`]
    pub fn render_center(&mut self, text: &str, pos: Vec2, parent_width: u16) -> DrawingResult {
        // get center
        let center = get_center((parent_width, 1), (text.len() as u16, 1));

        // draw
        // center.0 + pos.0 so it's offset by the position of what we're centering around
        self.stdout.queue(cursor::MoveTo(center.0 + pos.0, pos.1))?;
        self.stdout.write(text.as_bytes())?;

        // done
        Ok(RectBoundary {
            pos,
            size: (text.len() as u16, 1),
        })
    }

    /// Draw text at a given [`Vec2`]
    pub fn render(&mut self, text: &str, pos: Vec2) -> DrawingResult {
        // draw
        // center.0 + pos.0 so it's offset by the position of what we're centering around
        self.stdout.queue(cursor::MoveTo(pos.0, pos.1))?;
        self.stdout.write(text.as_bytes())?;

        // done
        Ok(RectBoundary {
            pos: (pos.0, pos.1),
            size: (text.len() as u16, 1),
        })
    }

    /// Draw text at a given [`Vec2`] as a button
    pub fn render_button(&mut self, text: &str, pos: Vec2) -> DrawingResult {
        // draw
        // center.0 + pos.0 so it's offset by the position of what we're centering around
        self.stdout.queue(cursor::MoveTo(pos.0, pos.1))?;
        self.stdout
            .write(format!("\x1b[107;30m➚ {text}\x1b[0m").as_bytes())?;

        // done
        Ok(RectBoundary {
            pos: (pos.0, pos.1),
            size: (text.len() as u16, 1),
        })
    }
}

impl Clickable for Text {}

// status line
pub struct StatusLine {
    pub stdout: StdoutLock<'static>,
}

impl Creatable for StatusLine {
    fn new(stdout: StdoutLock<'static>) -> Self {
        StatusLine { stdout }
    }
}

impl Component for StatusLine {
    /// Draw a status line (full width line)
    ///
    /// ## Arguments:
    /// * `stdout`
    /// * `rect` - size(x, y), pos(x, y)
    fn render(&mut self, window_size: (u16, u16), rect: RectBoundary) -> DrawingResult {
        // move to pos
        self.stdout.queue(cursor::MoveTo(rect.pos.0, rect.pos.1))?;

        // draw chars
        self.stdout.write(b"\x1b[107;30m")?; // white backgroud, black text
        self.stdout
            .write(" ".repeat(rect.size.0 as usize).to_string().as_bytes())?;
        self.stdout.write(b"\x1b[0m")?;

        // done
        self.stdout.queue(cursor::MoveTo(0, 0))?;
        Ok(RectBoundary {
            pos: rect.pos,
            size: (window_size.0, 1),
        })
    }
}
