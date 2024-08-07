use std::{
    env, fs,
    io::{self, Write},
    string::ToString,
    time::Duration,
};

use anyhow::{Context, Result};

use serde_json::Value;

use crossterm::{
    cursor,
    event::{poll, read, Event, KeyCode},
    execute,
    queue,
    style::{Color, Print, ResetColor, SetForegroundColor},
    terminal,
    // ExecutableCommand,
    QueueableCommand,
};

#[derive(Default)]
struct RenderData {
    level: usize,
    index: usize,
    current: String,
    old_indicies: Vec<usize>,
    path: Vec<String>,
}

impl RenderData {
    fn index(&self) -> usize {
        self.index
    }

    // TODO: implement
    fn indent(&self) -> u16 {
        0
        // (self.level * 4).try_into().unwrap()
    }

    fn path(&self) -> String {
        if self.path.is_empty() {
            return String::new();
        }
        "/".to_owned() + &self.path.join("/")
    }

    fn set_current(&mut self, new: &str) {
        self.current = new.to_string();
    }

    fn push_path(&mut self) {
        self.old_indicies.push(self.index);
        self.index = 0;
        self.level += 1;
        self.path.push(self.current.clone());
    }

    fn pop_path(&mut self) {
        if !self.old_indicies.is_empty() {
            self.index = self.old_indicies.pop().unwrap_or(0);
            self.level -= 1;
            self.path.pop();
        }
    }

    fn inc_index(&mut self) {
        self.index += 1;
    }

    fn dec_index(&mut self) {
        self.index = self.index.saturating_sub(1);
    }
}

fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();
    let path = &args[1];
    let file = fs::read_to_string(path).context("File Input")?;

    let mut stdout = io::stdout();
    if let Err(e) = main_loop(&stdout, &file) {
        execute!(
            stdout,
            cursor::Show,
            ResetColor,
            terminal::LeaveAlternateScreen
        )?;
        terminal::disable_raw_mode()?;
        println!("Error: {e:?}\r");
    }

    Ok(())
}

#[allow(clippy::too_many_lines)]
fn main_loop(mut stdout: &io::Stdout, file: &str) -> Result<()> {
    let value: Value = serde_json::from_str(file).context("Json Deserialization")?;
    // let value = value.as_object().unwrap();

    let mut render_data = RenderData::default();
    let mut node = &value;

    execute!(stdout, cursor::Hide, terminal::EnterAlternateScreen)?;
    terminal::enable_raw_mode()?;

    loop {
        queue!(
            stdout,
            cursor::MoveTo(0, 0),
            terminal::Clear(terminal::ClearType::All)
        )?;
        match node {
            Value::Array(vec) => {
                for (i, v) in vec.iter().enumerate() {
                    if i == render_data.index() {
                        let s = i.to_string();
                        render_data.set_current(&s);
                        queue!(stdout, SetForegroundColor(Color::Blue))?;
                    }
                    let str = match v {
                        Value::Object(map) => {
                            map.keys().fold("{ ".to_owned(), |acc, k| acc + k) + " }"
                        }
                        Value::Array(arr) => {
                            "[ ".to_owned()
                                + &arr.first().map_or(String::new(), ToString::to_string)
                                + " ]"
                        }
                        Value::Bool(val) => val.to_string(),
                        Value::String(val) => val.to_string(),
                        Value::Number(val) => val.to_string(),
                        Value::Null => "null".to_string(),
                    };
                    queue!(
                        stdout,
                        Print(str),
                        SetForegroundColor(Color::White),
                        cursor::MoveToNextLine(1),
                        cursor::MoveToColumn(render_data.indent()),
                    )?;
                }
            }
            Value::Object(obj) => {
                for (i, (k, v)) in obj.iter().enumerate() {
                    if i == render_data.index() {
                        render_data.set_current(k);
                        queue!(
                            stdout,
                            SetForegroundColor(Color::Blue),
                            Print(k),
                            SetForegroundColor(Color::White),
                        )?;
                    } else {
                        stdout.queue(Print(k))?;
                    }
                    if !(v.is_array() || v.is_object()) {
                        queue!(stdout, Print(": "), Print(v))?;
                    }
                    queue!(
                        stdout,
                        cursor::MoveToNextLine(1),
                        cursor::MoveToColumn(render_data.indent())
                    )?;
                }
            }
            Value::Bool(v) => queue!(stdout, Print(v))?,
            Value::String(v) => queue!(stdout, Print(v))?,
            Value::Number(v) => queue!(stdout, Print(v))?,
            Value::Null => queue!(stdout, Print("null"))?,
        }
        stdout.flush()?;

        let event = read()?;
        if let Event::Resize(x, y) = event {
            let (_original_size, _new_size) = flush_resize_events((x, y));
        }

        if event == Event::Key(KeyCode::Char('q').into()) {
            break;
        }
        if event == Event::Key(KeyCode::Char('j').into()) {
            render_data.inc_index();
        }
        if event == Event::Key(KeyCode::Char('k').into()) {
            render_data.dec_index();
        }
        if event == Event::Key(KeyCode::Char('l').into()) {
            render_data.push_path();
            let path = render_data.path();
            node = value
                .pointer(&path)
                .context(format!("invalid path: {path}"))?;
        }
        if event == Event::Key(KeyCode::Char('h').into()) {
            render_data.pop_path();
            let path = render_data.path();
            node = value
                .pointer(&path)
                .context(format!("invalid path: {path}"))?;
        }
    }

    execute!(
        stdout,
        cursor::Show,
        ResetColor,
        terminal::LeaveAlternateScreen
    )?;
    terminal::disable_raw_mode()?;
    Ok(())
}

fn flush_resize_events(first_resize: (u16, u16)) -> ((u16, u16), (u16, u16)) {
    let mut last_resize = first_resize;
    while let Ok(true) = poll(Duration::from_millis(50)) {
        if let Ok(Event::Resize(x, y)) = read() {
            last_resize = (x, y);
        }
    }

    (first_resize, last_resize)
}
