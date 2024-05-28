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
    /// Write changes to the buffer.
    /// If `pos` is greater than what the buffer supports, `Err` is returned.
    ///
    /// ## Arguments
    /// * `pos` - [`Vec2`]
    /// * `buf` - [`BufCell`] (new cell)
    pub fn write(&mut self, pos: Vec2, buf: BufCell) -> IOResult<BufState> {
        // get row
        let row = self.vec.get_mut(pos.1 as usize);

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

    /// Like [`write`], but with a str
    pub fn write_str(&mut self, pos: Vec2, buf: &str) -> IOResult<BufState> {
        let chars = buf.chars().collect::<Vec<char>>();

        for i in 0..chars.len() {
            // get pos
            let pos = (pos.0 + (i as u16), pos.1);

            // write char
            self.write(pos, BufCell::from_char(chars.get(i).unwrap().to_owned()))?;
        }

        Ok(BufState::Ok)
    }

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

    /// Commit changes to buffer.
    pub fn commit(&mut self) -> IOResult<BufState> {
        // loop through rows to find changed rows
        // the buffer does NOT represent what is on screen, instead it is just
        // what SHOULD go on screen (we're allowed to lose some data since it'll likely redraw later)
        let empty_row = BufCell::as_row(self.size.0);

        for (y, row) in self.vec.clone().iter().enumerate() {
            let is_changed = row != &empty_row; // if the row is no longer fully empty it changed

            if !is_changed {
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

                // only update if char is different OR state changed
                if screen_vec_char.char == col.char {
                    continue;
                }

                // ...
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
        Ok(BufState::Ok)
    }
}
