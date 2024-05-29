//! Components
use crate::buffer::{BufferWrite, PseudoBuffer};
use crate::State;

// traits
pub trait Component {
    fn render(&mut self, window_size: (u16, u16), rect: RectBoundary) -> DrawingResult;
}

/// Component can be created with "::new()"
pub trait Creatable {
    fn new(buffer: PseudoBuffer) -> Self;
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
        buffer: &mut PseudoBuffer,
        height: u16,
        start: Vec2,
        char: &str,
        end_char: &str,
    ) -> RectBoundary {
        for i in 0..height {
            if i == height - 1 {
                buffer.write_str((start.0, start.1 + i), end_char).unwrap();
                break;
            }

            buffer.write_str((start.0, start.1 + i), char).unwrap();
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
    pub buffer: PseudoBuffer,
}

impl Creatable for QuickBox {
    fn new(buffer: PseudoBuffer) -> Self {
        QuickBox { buffer }
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
            self.buffer
                .write_str((pos.0, row), &" ".repeat(len_x as usize))?;
        }

        // auto resize (y)
        if size.1 >= window_size.1 {
            size.1 -= size.1 - window_size.1;
        }

        // draw line
        let line_top = format!("╭{}╮", "─".repeat((size.0 - 2) as usize));
        let line_bottom = "─".repeat((size.0 - 2) as usize);

        // write
        self.buffer.write_str(pos, &line_top)?; // top

        DownwardsLine::new(&mut self.buffer, size.1, (pos.0, pos.1 + 1), "│", "╰"); // left
        DownwardsLine::new(
            // right
            &mut self.buffer,
            size.1,
            (pos.0 + size.0 - 1, pos.1 + 1),
            "│",
            "╯",
        );

        self.buffer
            .write_str((pos.0 + 1, pos.1 + size.1), &line_bottom)?; // bottom

        // done
        Ok(RectBoundary { pos, size })
    }
}

// text
pub struct Text {
    pub buffer: PseudoBuffer,
}

impl Creatable for Text {
    fn new(buffer: PseudoBuffer) -> Self {
        Text { buffer }
    }
}

impl Text {
    /// Draw text at the center of a given [`Vec2`]
    pub fn render_center(&mut self, text: &str, pos: Vec2, parent_width: u16) -> DrawingResult {
        // get center
        let center = get_center((parent_width, 1), (text.len() as u16, 1));

        // draw
        // center.0 + pos.0 so it's offset by the position of what we're centering around
        self.buffer.write_str((center.0 + pos.0, pos.1), text)?;

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
        self.buffer.write_str(pos, text)?;

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
        self.buffer
            .write_str(pos, &format!("\x1b[107;30m➚ {text}\x1b[0m"))?;

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
    pub buffer: PseudoBuffer,
}

impl Creatable for StatusLine {
    fn new(buffer: PseudoBuffer) -> Self {
        StatusLine { buffer }
    }
}

impl Component for StatusLine {
    /// Draw a status line (full width line)
    ///
    /// ## Arguments:
    /// * `stdout`
    /// * `rect` - size(x, y), pos(x, y)
    fn render(&mut self, window_size: (u16, u16), rect: RectBoundary) -> DrawingResult {
        // draw chars
        self.buffer.write_str(rect.pos, "\x1b[107;30m")?; // white backgroud, black text
        self.buffer
            .write_str(rect.pos, &" ".repeat(rect.size.0 as usize))?;
        self.buffer
            .write_str((rect.pos.0 + rect.size.0, rect.pos.1), "\x1b[0m")?;

        // done
        Ok(RectBoundary {
            pos: rect.pos,
            size: (window_size.0, 1),
        })
    }
}
