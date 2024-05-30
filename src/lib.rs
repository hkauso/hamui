pub mod buffer;
pub mod drawing;

use crossterm::event::{poll, read, Event, KeyCode, KeyModifiers, MouseEventKind};
use crossterm::QueueableCommand;
use crossterm::{cursor, terminal};
use std::io::{Result as IOResult, Stdout, Write};

use crate::buffer::BufferWrite;

/// Main UI state
pub struct State {
    /// Window size as a [`Vec2`]
    pub window_size: drawing::Vec2,
    /// If we are in mouse cursor mode or keyboard mode
    pub keyboard_input_mode: bool,
    /// Where the user has clicked on the screen (in cursor mode)
    /// Where we are typing (in keyboard mode)
    pub clicked: drawing::Vec2,
    /// Current text input (in keyboard mode)
    pub input: String,
    /// Where the cursor currently is (in cursor mode)
    pub cursor_pos: drawing::Vec2,
    /// Minimum cursor X value
    pub min_x: u16,
}

pub type Drawfn = dyn FnMut(&mut State, buffer::PseudoBuffer) -> buffer::PseudoBuffer;

/// UI Frame
pub struct Frame<'a> {
    stdout: Stdout,
    draw_fn: &'a mut Drawfn,
    buffer: buffer::Buffer,
    state: State,
}

impl Frame<'_> {
    /// Create a new [`UIFrame`]
    pub fn new(stdout: Stdout, draw_fn: &'_ mut Drawfn) -> Frame {
        let window_size = terminal::size().unwrap();

        // ...
        Frame {
            stdout,
            draw_fn,
            buffer: buffer::Buffer::new(std::io::stdout(), window_size),
            state: State {
                window_size,
                keyboard_input_mode: false, // mouse by default
                clicked: (0, 0),
                input: String::new(),
                cursor_pos: (0, 0),
                min_x: 0,
            },
        }
    }

    /// Step rendering without redrawing components
    pub fn step_no_draw(&mut self) -> IOResult<buffer::BufState> {
        // commit changes
        self.buffer.commit()?; // push buffer to screen
        self.move_cursor(self.state.cursor_pos)?; // sync actual cursor and cusor_pos
        Ok(buffer::BufState::Ok)
    }

    /// Step rendering
    pub fn step(&mut self) -> IOResult<buffer::BufState> {
        // call function and consume changes
        let pseudo = (self.draw_fn)(&mut self.state, buffer::PseudoBuffer::new(self.buffer.size));
        self.buffer.consume_changes(pseudo.get_changes())?; // move changes to buffer

        // commit changes
        self.step_no_draw()
    }

    /// Move cursor
    pub fn move_cursor(&mut self, pos: drawing::Vec2) -> IOResult<buffer::BufState> {
        self.stdout.queue(cursor::MoveTo(pos.0, pos.1))?;
        Ok(buffer::BufState::Ok)
    }

    /// Open frame environment
    pub fn open_env(&mut self) -> IOResult<()> {
        self.stdout.queue(terminal::EnterAlternateScreen)?;
        self.stdout.queue(cursor::MoveTo(0, 0))?;
        terminal::enable_raw_mode().unwrap();
        self.stdout
            .queue(crossterm::event::EnableMouseCapture)
            .unwrap();
        Ok(())
    }

    /// Exit frame
    pub fn exit(&mut self) -> () {
        terminal::disable_raw_mode().unwrap();
        self.stdout.queue(terminal::LeaveAlternateScreen).unwrap();
        self.stdout
            .queue(crossterm::event::DisableMouseCapture)
            .unwrap();
        self.stdout.flush().unwrap();
        std::process::exit(0);
    }

    /// Handle all events
    pub fn poll_events(&mut self) -> IOResult<buffer::BufState> {
        let window_size = self.buffer.size;
        if poll(std::time::Duration::from_millis(0)).expect("Failed to poll events!") {
            match read().expect("Failed to read event!") {
                // handle window resize
                Event::Resize(width, height) => {
                    // sync buffer and window
                    self.buffer.resize((width, height))?;

                    // clear
                    self.stdout
                        .queue(terminal::Clear(terminal::ClearType::All))
                        .unwrap();

                    // redraw
                    // we're not drawing every frame, instead we only draw when needed
                    self.step()?;
                }
                // handle keyboard events
                Event::Key(event) => {
                    match event.code {
                        KeyCode::Char(c) => {
                            if event.modifiers.contains(KeyModifiers::CONTROL) {
                                match c {
                                    'c' => {
                                        // Ctrl+C
                                        // handle smooth exit
                                        self.exit();
                                    }
                                    _ => {}
                                }
                            } else {
                                if self.state.keyboard_input_mode == false {
                                    return Ok(buffer::BufState::Ok);
                                }

                                // add to prompt
                                let write_at = self.state.clicked.0;
                                let real_pos = self.state.cursor_pos.0 - write_at; // where we are in the prompt

                                if real_pos > self.state.input.len() as u16 {
                                    return Ok(buffer::BufState::Ok);
                                }

                                // write char to input
                                self.state.input.insert(real_pos as usize, c);

                                // update screen
                                let old_loc = self.state.cursor_pos.0;

                                self.state.cursor_pos = (write_at, self.state.cursor_pos.1); // move to line start
                                self.move_cursor(self.state.cursor_pos)?;

                                // actual write
                                self.buffer.write_str(
                                    (write_at, self.state.cursor_pos.1),
                                    &self.state.input,
                                )?;

                                // move cursor back
                                self.state.cursor_pos = (old_loc, self.state.cursor_pos.1); // restore position
                                self.move_cursor(self.state.cursor_pos)?;

                                // move cursor
                                self.state.cursor_pos.0 += 1;

                                // redraw
                                self.step()?;

                                // ...
                                return Ok(buffer::BufState::Ok);
                            }
                        }
                        // Toggle Mouse Mode
                        KeyCode::Esc => {
                            self.state.keyboard_input_mode = !self.state.keyboard_input_mode;

                            if self.state.keyboard_input_mode == true {
                                // we use the x of clicked to tell where we're typing,
                                // setting this to the current cursor position will make
                                // us type in the correct location
                                self.state.clicked.0 = self.state.cursor_pos.0;
                            } else {
                                // TODO: do something to expose the input
                                self.state.input = String::new(); // clear input
                            }
                        }
                        // Submit
                        KeyCode::Enter => {
                            // let res = inter_stdin(prompt.clone(), global_state);
                            // global_state = res.0; // update global state

                            // map_result(&res.1);

                            // clear prompt
                            self.state.input = String::new();

                            // if we're at the end of the frame, clear
                            if (self.state.cursor_pos.1 + 1) == window_size.1 {
                                // TODO: clear buffer here
                                self.stdout
                                    .queue(terminal::Clear(terminal::ClearType::All))
                                    .unwrap();

                                self.state.cursor_pos = (0, 0);
                                self.move_cursor(self.state.cursor_pos)?;
                            } else {
                                // line down from clicked.1 at clicked.0 (write_at)
                                self.state.clicked.1 += 1;
                                self.state.cursor_pos = self.state.clicked.clone();
                            }

                            // redraw
                            self.step()?;
                        }
                        // Move Left
                        KeyCode::Left => {
                            if self.state.cursor_pos.0 == self.state.min_x {
                                // cannot go through prompt
                                return Ok(buffer::BufState::Ok);
                            }

                            self.state.cursor_pos.0 -= 1;
                        }
                        // Move Right
                        KeyCode::Right => {
                            if self.state.cursor_pos.0 == (window_size.0 - 51) {
                                // cannot go through side windows (50 cells wide)
                                return Ok(buffer::BufState::Ok);
                            }

                            self.state.cursor_pos.0 += 1;
                        }
                        // Backspace
                        KeyCode::Backspace => {
                            if self.state.cursor_pos.0 == self.state.min_x {
                                // cannot go through prompt
                                return Ok(buffer::BufState::Ok);
                            }

                            // make sure we are within the prompt
                            let write_at = self.state.clicked.0;
                            let real_pos = self.state.cursor_pos.0 - write_at; // where we are in the prompt

                            if (real_pos > self.state.input.len() as u16) | (real_pos == 0) {
                                return Ok(buffer::BufState::Ok);
                            }

                            self.state.input.remove((real_pos - 1) as usize); // remove character

                            // move cursor back
                            self.state.cursor_pos.0 -= 1;

                            // update screen
                            let old_loc = self.state.cursor_pos.0.clone();

                            // write the whole input + a space so the character gets erased
                            self.buffer.fill_range(
                                write_at,
                                (self.state.input.len() + 1) as u16,
                                self.state.cursor_pos.1,
                                buffer::BufCell::EMPTY,
                            )?;

                            self.buffer.write_str(
                                (write_at, self.state.cursor_pos.1),
                                &" ".repeat(self.state.input.len() + 1),
                            )?;

                            self.buffer.write_str(
                                (write_at, self.state.cursor_pos.1),
                                &self.state.input,
                            )?;

                            // ...
                            self.state.cursor_pos = (old_loc, self.state.cursor_pos.1); // restore position
                            self.move_cursor(self.state.cursor_pos)?;

                            // redraw
                            self.step()?;
                        }
                        // ...
                        _ => {}
                    }
                }
                // handle mouse events
                Event::Mouse(event) => {
                    if self.state.keyboard_input_mode == true {
                        return Ok(buffer::BufState::Ok);
                    }

                    // ...
                    if event.kind == MouseEventKind::Up(crossterm::event::MouseButton::Left) {
                        // handle click
                        self.state.clicked = (event.column, event.row);

                        // redraw
                        self.stdout.queue(cursor::SavePosition).unwrap();
                        self.step()?;
                        self.stdout.queue(cursor::RestorePosition).unwrap();
                    } else if event.kind == MouseEventKind::Moved {
                        // move cursor to position (like a cursor)
                        self.state.cursor_pos = (event.column, event.row);
                        self.move_cursor(self.state.cursor_pos)?;
                    }
                }
                // drop everything else
                _ => (),
            };
        }

        Ok(buffer::BufState::Ok)
    }
}

impl Write for Frame<'_> {
    // just forward everything to the stdout, this is just for convenience
    fn write(&mut self, buf: &[u8]) -> IOResult<usize> {
        self.stdout.write(buf)
    }

    fn flush(&mut self) -> IOResult<()> {
        self.stdout.flush()
    }
}
