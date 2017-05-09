// Copyright 2017 Mozilla
//
// Licensed under the Apache License, Version 2.0 (the "License"); you may not use
// this file except in compliance with the License. You may obtain a copy of the
// License at http://www.apache.org/licenses/LICENSE-2.0
// Unless required by applicable law or agreed to in writing, software distributed
// under the License is distributed on an "AS IS" BASIS, WITHOUT WARRANTIES OR
// CONDITIONS OF ANY KIND, either express or implied. See the License for the
// specific language governing permissions and limitations under the License.

use std::io::{self, stdin, BufRead, BufReader};

use linefeed::Reader;
use linefeed::terminal::DefaultTerminal;

use self::InputResult::*;

/// Possible results from reading input from `InputReader`
#[derive(Clone, Debug)]
pub enum InputResult {
    /// rusti command as input; (name, rest of line)
    Command(String, Option<String>),
    /// An empty line
    Empty,
    /// Needs more input; i.e. there is an unclosed delimiter
    More,
    /// End of file reached
    Eof,
}

/// Reads input from `stdin`
pub struct InputReader {
    buffer: String,
    reader: Option<Reader<DefaultTerminal>>,
}

impl InputReader {
    /// Constructs a new `InputReader` reading from `stdin`.
    pub fn new() -> InputReader {
        let r = match Reader::new("mentat") {
            Ok(mut r) => {
                r.set_word_break_chars(" \t\n!\"#$%&'()*+,-./:;<=>?@[\\]^`");
                Some(r)
            }
            Err(_) => None
        };

        InputReader{
            buffer: String::new(),
            reader: r,
        }
    }

    /// Returns whether the `InputReader` is reading from a TTY.
    pub fn is_tty(&self) -> bool {
        self.reader.is_some()
    }

    /// Reads a single command, item, or statement from `stdin`.
    /// Returns `More` if further input is required for a complete result.
    /// In this case, the input received so far is buffered internally.
    pub fn read_input(&mut self, prompt: &str) -> InputResult {
        let line = match self.read_line(prompt) {
            Some(s) => s,
            None => return Eof,
        };

        self.buffer.push_str(&line);

        if self.buffer.is_empty() {
            return Empty;
        }

        self.add_history(&line);

        let res = More;

        match res {
            More => (),
            _ => self.buffer.clear(),
        };

        res
    }

    fn read_line(&mut self, prompt: &str) -> Option<String> {
        match self.reader {
            Some(ref mut r) => {
                r.set_prompt(prompt);
                r.read_line().ok().and_then(|line| line)
            }
            None => self.read_stdin()
        }
    }

    fn read_stdin(&self) -> Option<String> {
        let mut s = String::new();

        match stdin().read_line(&mut s) {
            Ok(0) | Err(_) => None,
            Ok(_) => Some(s)
        }
    }

    fn add_history(&mut self, line: &str) {
        if let Some(ref mut r) = self.reader {
            r.add_history(line.to_owned());
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
    }
}
