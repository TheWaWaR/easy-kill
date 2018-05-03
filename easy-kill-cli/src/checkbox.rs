use std::io;
use std::ops::Rem;
use std::iter::repeat;

use console::{Key, Term};


/// Renders a multi select checkbox menu.
pub struct Checkbox {
    items: Vec<String>,
    clear: bool,
}

impl Checkbox {
    /// Creates the prompt with a specific text.
    pub fn new() -> Self {
        Checkbox {
            items: vec!["ALL".to_owned()],
            clear: true,
        }
    }

    /// Sets the clear behavior of the checkbox menu.
    ///
    /// The default is to clear the checkbox menu.
    pub fn clear(&mut self, val: bool) -> &mut Self {
        self.clear = val;
        self
    }

    /// Add a single item to the selector.
    pub fn item(&mut self, item: &str) -> &mut Self {
        self.items.push(item.to_string());
        self
    }

    /// Adds multiple items to the selector.
    pub fn items(&mut self, items: &[&str]) -> &mut Self {
        for item in items {
            self.items.push(item.to_string());
        }
        self
    }

    /// Enables user interaction and returns the result.
    ///
    /// The user can select the items with the space bar and on enter
    /// the selected items will be returned.
    pub fn interact(&self) -> io::Result<Vec<usize>> {
        self.interact_on(&Term::stderr())
    }

    /// Like `interact` but allows a specific terminal to be set.
    pub fn interact_on(&self, term: &Term) -> io::Result<Vec<usize>> {
        let mut sel = 0;
        let mut selected: Vec<_> = repeat(false).take(self.items.len()).collect();
        loop {
            for (idx, item) in self.items.iter().enumerate() {
                term.write_line(&format!(
                    "{} [{}] {}",
                    if sel == idx { ">" } else { " " },
                    if selected[idx] { "x" } else { " " },
                    item,
                ))?;
            }
            match term.read_key()? {
                Key::ArrowDown | Key::Char('j') => {
                    if sel == !0 {
                        sel = 0;
                    } else {
                        sel = (sel as u64 + 1).rem(self.items.len() as u64) as usize;
                    }
                }
                Key::ArrowUp | Key::Char('k') => {
                    if sel == !0 {
                        sel = self.items.len() - 1;
                    } else {
                        sel = ((sel as i64 - 1 + self.items.len() as i64) %
                               (self.items.len() as i64)) as usize;
                    }
                }
                Key::Char(' ') => {
                    selected[sel] = !selected[sel];
                    if sel == 0 {
                        let result = selected[0];
                        selected.iter_mut().skip(1).for_each(|x| {
                            *x = result;
                        });
                    }
                    if selected.iter().skip(1).position(|x| !x).is_some() {
                        selected[0] = false;
                    }
                    if selected.iter().skip(1).all(|x| *x) {
                        selected[0] = true;
                    }
                }
                Key::Escape => {
                    if self.clear {
                        term.clear_last_lines(self.items.len())?;
                    }
                    return Ok(vec![]);
                },
                Key::Enter => {
                    if self.clear {
                        term.clear_last_lines(self.items.len())?;
                    }
                    return Ok(
                        selected.into_iter()
                            .skip(1)
                            .enumerate()
                            .filter_map(|(idx, selected)| if selected { Some(idx) } else { None })
                            .collect()
                    );
                }
                _ => {}
            }
            term.clear_last_lines(self.items.len())?;
        }
    }
}
