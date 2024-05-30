//! Display buffer
//!
//! Write are written to the buffer first and then only the needed area is updated.
use crossterm::cursor;
use crossterm::QueueableCommand;
use std::io::{Result as IOResult, Stdout, Write};

use super::drawing::Vec2;

// extras
pub enum BufState {
    /// Operation success
    Ok,
}

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct BufCell {
    pub char: char,
    pub empty: bool,
}

impl BufCell {
    pub const EMPTY: BufCell = BufCell {
        char: ' ',
        empty: true,
    };

    /// Create [`BufCell`] from a [`char`]
    pub fn from_char(char: char) -> BufCell {
        BufCell {
            char,
            empty: char == ' ',
        }
    }

    /// Create a row of buffers with the specified width
    pub fn as_row(width: u16) -> Row {
        let mut vec = Vec::new();
        vec.resize(width as usize, BufCell::EMPTY);
        vec
    }
}

pub type Row = Vec<BufCell>;

// traits
pub trait BufferWrite {
    /// Write changes to the buffer.
    /// If `pos` is greater than what the buffer supports, `Err` is returned.
    ///
    /// ## Arguments
    /// * `pos` - [`Vec2`]
    /// * `buf` - [`BufCell`] (new cell)
    fn write_cell(&mut self, pos: Vec2, buf: BufCell) -> IOResult<BufState>;
    /// Like [`write`], but with a str
    fn write_str(&mut self, pos: Vec2, buf: &str) -> IOResult<BufState> {
        let chars = buf.chars().collect::<Vec<char>>();

        for i in 0..chars.len() {
            // get pos
            let pos = (pos.0 + (i as u16), pos.1);

            // write char
            self.write_cell(pos, BufCell::from_char(chars.get(i).unwrap().to_owned()))?;
        }

        Ok(BufState::Ok)
    }
}

// main buffer
pub struct Buffer {
    stdout: Stdout,
    pub size: Vec2,
    /// Vector of [`Row`]s, pre commit
    pub vec: Vec<Row>,
    /// Vector of [`Row`]s, what's on screen
    pub screen_vec: Vec<Row>,
}

impl Buffer {
    // init
    /// Create a new buffer with a [`Vec2`].
    ///
    /// ## Arguments
    /// * `stdout`: [`Stdout`]
    /// * `size`: [`Vec2`]
    pub fn new(stdout: Stdout, size: Vec2) -> Buffer {
        let mut vec = Vec::new();
        vec.resize(size.1 as usize, BufCell::as_row(size.0));

        // ...
        Buffer {
            stdout,
            size,
            vec: vec.clone(),
            screen_vec: vec.clone(),
        }
    }

    /// Stdout thing
    pub fn queue(&mut self, cmd: impl crossterm::Command) -> IOResult<&mut Stdout> {
        self.stdout.queue(cmd)
    }

    /// Get a cell in the `screen_vec` using its [`Vec2`] position
    pub fn get_cell(&mut self, pos: Vec2) -> IOResult<BufCell> {
        // get row
        let row = self.screen_vec.get_mut(pos.1 as usize);

        if row.is_none() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "Row is invalid.",
            ));
        }

        let row = row.unwrap();

        // get col
        let col = row.get(pos.0 as usize);

        if col.is_none() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "Column is invalid.",
            ));
        }

        // return
        Ok(col.unwrap().to_owned())
    }

    /// Resize a single vector to match screen size
    fn resize_vec(&mut self, mut vec: Vec<Row>, size: Vec2) -> IOResult<Vec<Row>> {
        // resize x
        let rows_to_edit = 0..vec.len();

        for i in rows_to_edit {
            let r = vec.get_mut(i).unwrap();
            r.resize(size.0 as usize, BufCell::EMPTY);
        }

        // resize y
        vec.resize(size.1 as usize, BufCell::as_row(size.0));

        // return
        Ok(vec)
    }

    /// Resize buffer with a [`Vec2`].
    /// If larger than current buffer, new characters are added as whitespace.
    ///
    /// ## Arguments
    /// * `size`: [`Vec2`]
    pub fn resize(&mut self, size: Vec2) -> IOResult<BufState> {
        self.vec = self.resize_vec(self.vec.clone(), size)?;
        self.screen_vec = self.resize_vec(self.screen_vec.clone(), size)?;

        // ...
        self.size = size; // update size
        Ok(BufState::Ok)
    }

    // writing
    /// Like [`write`], but with a range of columns
    pub fn fill_range(
        &mut self,
        start: u16,
        end: u16,
        row_y: u16,
        buf: BufCell,
    ) -> IOResult<BufState> {
        // get row
        let row = self.vec.get_mut(row_y as usize);

        if row.is_none() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "Row is invalid.",
            ));
        }

        let row: &mut Row = row.unwrap();

        // update every column in this range
        let r = start..end;

        for col in r {
            row[col as usize] = buf.clone();
        }

        // return
        Ok(BufState::Ok)
    }

    /// Consume changes from a [`PseudoBuffer`]
    pub fn consume_changes(&mut self, changes: Vec<BufferChange>) -> IOResult<BufState> {
        for change in changes {
            // make sure change is ACTUALLY a change
            let cell = self.get_cell(change.loc)?;
            let is_changed: bool = cell != change.cell;

            if is_changed == false {
                continue;
            }

            // ...
            self.write_cell(change.loc, change.cell)?;
        }

        Ok(BufState::Ok)
    }

    /// Commit changes to buffer.
    pub fn commit(&mut self) -> IOResult<BufState> {
        // self.queue(crossterm::terminal::BeginSynchronizedUpdate)?; // commit all changes at once

        // loop through rows to find changed rows
        // the buffer does NOT represent what is on screen, instead it is just
        // what SHOULD go on screen (we're allowed to lose some data since it'll likely redraw later)
        let empty_row = BufCell::as_row(self.size.0);

        for (y, row) in self.vec.clone().iter().enumerate() {
            let is_empty = row != &empty_row;

            if !is_empty {
                continue;
            }

            // get screen_vec version of this row
            // if the row doesn't exist, the buf was likely resized ...
            // we're going to skip this row if it doesn't exist on screen
            let screen_vec_row = self.screen_vec.get_mut(y);

            if screen_vec_row.is_none() {
                continue;
            }

            let screen_vec_row = screen_vec_row.unwrap();

            // make sure something in the row ACTUALLY changed so we don't
            // pointlessly move the cursor (which stops mouse events)
            if screen_vec_row == row {
                continue;
            }

            // move cursor
            self.stdout.queue(cursor::MoveTo(0, y as u16))?;

            // build full line
            for (x, col) in row.iter().enumerate() {
                // get screen_vec_char (same deal as screen_vec_row)
                let screen_vec_char = screen_vec_row.get_mut(x);

                if screen_vec_char.is_none() {
                    continue;
                }

                let screen_vec_char = screen_vec_char.unwrap();

                // if screen_vec_char is not empty but this one is, skip
                // we should directly write to the screen vec if we want to clear things
                if (col.empty == true) && (screen_vec_char.empty == false) {
                    continue;
                }

                // only update if char is different OR state changed
                if screen_vec_char.char == col.char {
                    continue;
                }

                // move vec row changes to screen_vec_row
                screen_vec_row[x] = col.to_owned();
            }

            // build text line from screen_vec_row
            let mut line: String = String::new();

            for cell in screen_vec_row {
                line.push(cell.char);
            }

            // write line
            self.stdout.write(line.as_bytes())?;
        }

        // flush stdout
        self.stdout.flush()?;

        // return
        self.vec.fill(BufCell::as_row(self.size.0));
        // self.queue(crossterm::terminal::EndSynchronizedUpdate)?; // commit to screen
        Ok(BufState::Ok)
    }
}

impl Write for Buffer {
    // just forward everything to the stdout, this is just for convenience
    fn write(&mut self, buf: &[u8]) -> IOResult<usize> {
        self.stdout.write(buf)
    }

    fn flush(&mut self) -> IOResult<()> {
        self.stdout.flush()
    }
}

impl BufferWrite for Buffer {
    fn write_cell(&mut self, pos: Vec2, buf: BufCell) -> IOResult<BufState> {
        // if we're writing an empty character, skip vec and write straight to screen
        // this fixes issues with keyboard mode backspace and some random crashes (???)
        let vec = if buf.empty == true {
            &mut self.screen_vec
        } else {
            &mut self.vec
        };

        // get row
        let row = vec.get_mut(pos.1 as usize);

        if row.is_none() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "Row is invalid.",
            ));
        }

        let row: &mut Row = row.unwrap();

        // update col
        if pos.0 > row.len() as u16 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "Column index is too large.",
            ));
        }

        row[pos.0 as usize] = buf;

        // return
        Ok(BufState::Ok)
    }
}

// pseudobuffer
#[derive(Debug, Clone)]
pub struct BufferChange {
    pub loc: Vec2,
    pub cell: BufCell,
}

/// This buffer receives changes like a normal buffer, but just stores them in a
/// vector which can be pulled with `.get_changes()`.
///
/// This is not meant to be used internally. It exists because [`Buffer`] can't impl Clone.
#[derive(Clone)]
pub struct PseudoBuffer {
    pub window_size: Vec2,
    /// Changes is append ONLY. If you must undo a change, just overwrite it.
    changes: Vec<BufferChange>,
}

impl PseudoBuffer {
    pub fn new(window_size: Vec2) -> PseudoBuffer {
        PseudoBuffer {
            window_size,
            changes: Vec::new(),
        }
    }

    /// Get all changes to the buffer
    pub fn get_changes(&self) -> Vec<BufferChange> {
        self.changes.clone()
    }

    /// We can only append or overwrite the whole thing
    pub fn set_changes(&mut self, changes: Vec<BufferChange>) -> () {
        self.changes = changes;
    }
}

impl BufferWrite for PseudoBuffer {
    fn write_cell(&mut self, pos: Vec2, buf: BufCell) -> IOResult<BufState> {
        self.changes.push(BufferChange {
            loc: pos,
            cell: buf,
        });

        Ok(BufState::Ok)
    }
}
