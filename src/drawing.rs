//! Components
use crate::buffer::{BufferChange, BufferWrite, PseudoBuffer};
use crate::State;

// traits
pub trait Component {
    fn render(&mut self, window_size: Vec2, rect: RectBoundary) -> DrawingResult;
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
pub type DrawingResult = Result<DrawingNode, std::io::Error>;
pub type DrawingNode = (RectBoundary, Vec<BufferChange>);

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
    fn render(&mut self, window_size: Vec2, rect: RectBoundary) -> DrawingResult {
        let pos = rect.pos;
        let mut size = rect.size;

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
        Ok((RectBoundary { pos, size }, self.buffer.get_changes()))
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
    pub fn render_center(&mut self, leaf: TextLeaf, pos: Vec2, parent_width: u16) -> DrawingResult {
        let text = &leaf.text;

        // get center
        let center = get_center((parent_width, 1), (text.len() as u16, 1));

        // draw
        // center.0 + pos.0 so it's offset by the position of what we're centering around
        self.buffer.write_str((center.0 + pos.0, pos.1), text)?;

        // done
        Ok((
            RectBoundary {
                pos,
                size: (text.len() as u16, 1),
            },
            self.buffer.get_changes(),
        ))
    }

    /// Draw text at a given [`Vec2`]
    pub fn render(&mut self, leaf: TextLeaf, pos: Vec2) -> DrawingResult {
        let text = &leaf.text;

        // draw
        // center.0 + pos.0 so it's offset by the position of what we're centering around
        self.buffer.write_str(pos, text)?;

        // done
        Ok((
            RectBoundary {
                pos: (pos.0, pos.1),
                size: (text.len() as u16, 1),
            },
            self.buffer.get_changes(),
        ))
    }

    /// Draw text at a given [`Vec2`] as a button
    pub fn render_button(&mut self, leaf: TextLeaf, pos: Vec2) -> DrawingResult {
        let text = &leaf.text;

        // draw
        // center.0 + pos.0 so it's offset by the position of what we're centering around
        self.buffer
            .write_str(pos, &format!("\x1b[107;30m➚ {text}\x1b[0m"))?;

        // done
        Ok((
            RectBoundary {
                pos: (pos.0, pos.1),
                size: (text.len() as u16, 1),
            },
            self.buffer.get_changes(),
        ))
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
        Ok((
            RectBoundary {
                pos: rect.pos,
                size: (window_size.0, 1),
            },
            self.buffer.get_changes(),
        ))
    }
}

// row
pub struct QuickRow {
    pub buffer: PseudoBuffer,
}

impl Creatable for QuickRow {
    fn new(buffer: PseudoBuffer) -> Self {
        QuickRow { buffer }
    }
}

impl QuickRow {
    /// Get the correct position of the next component.
    fn get_component_position(
        &self,
        prev_component_rect: Option<RectBoundary>,
        mut component_pos: Vec2,
    ) -> Vec2 {
        if prev_component_rect.is_none() {
            // leave component as is if it's the first
            return component_pos;
        }

        let prev_component_rect = prev_component_rect.unwrap();
        component_pos.0 += prev_component_rect.pos.0 + prev_component_rect.size.0; // new position is x + prev x + prev width
                                                                                   // height (size.1) and y (pos.1) is ignored, we don't need that
        component_pos
    }

    /// Render [`QuickRow`]. Components can only be simple text components.
    /// Starts at `rect.pos.0` and fills `components` with no gap.
    /// `components` contains `(content, size)` (`(TextLeaf, Vec2)`)
    pub fn render(
        &mut self,
        rect: RectBoundary,
        components: Vec<(TextLeaf, Vec2)>,
    ) -> DrawingResult {
        let mut prev_rect: Option<RectBoundary> = Option::None; // store previous row item
        let mut global_buffer = self.buffer.clone();

        for component in components {
            // create text component
            let mut text = Text::new(self.buffer.clone());

            // get correct component
            let pos = self.get_component_position(prev_rect.clone(), component.1);

            // render
            let res = text.render(component.0, pos)?;
            global_buffer.set_changes([global_buffer.get_changes(), res.1].concat());
            prev_rect = Option::Some(res.0);
            // concat global_buffer with component changes
        }

        // ...
        Ok((rect, global_buffer.get_changes()))
    }
}

// text leaf (just a small piece of text, not a full component)
#[derive(Debug)]
pub enum TextCommand {
    Reset = 0,
}

#[derive(Debug)]
pub enum TextAttribute {
    Bold = 1,
    Italic = 3,
    Underline = 4,
    Swap = 7,
}

#[derive(Debug)]
pub enum TextColor {
    Black = 30,
    Red = 31,
    Green = 32,
    Yellow = 33,
    Blue = 34,
    Magenta = 35,
    Cyan = 36,
    White = 37,
    BrightBlack = 90,
    BrightRed = 91,
    BrightGreen = 92,
    BrightYellow = 93,
    BrightBlue = 94,
    BrightMagenta = 95,
    BrightCyan = 96,
    BrightWhite = 97,
}

#[derive(Debug)]
pub enum TextBackgroundColor {
    Black = 40,
    Red = 41,
    Green = 42,
    Yellow = 43,
    Blue = 44,
    Magenta = 45,
    Cyan = 46,
    White = 47,
    BrightBlack = 100,
    BrightRed = 101,
    BrightGreen = 102,
    BrightYellow = 103,
    BrightBlue = 104,
    BrightMagenta = 105,
    BrightCyan = 106,
    BrightWhite = 107,
}

pub struct TextLeaf {
    pub text: String,
}

impl TextLeaf {
    pub fn new(text: String, fg: TextColor, bg: TextBackgroundColor) -> Self {
        TextLeaf {
            text: format!(
                "\x1b[{};{}m{text}\x1b[{}m",
                fg as u8,
                bg as u8,
                TextCommand::Reset as u8
            ),
        }
    }
}

impl From<&str> for TextLeaf {
    fn from(value: &str) -> Self {
        TextLeaf {
            text: value.to_string(),
        }
    }
}

impl std::fmt::Display for TextLeaf {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.text)
    }
}
