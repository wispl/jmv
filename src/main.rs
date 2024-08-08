use std::{
    env, fs,
    io::{self, Write},
    time::Duration,
};

use anyhow::{Context, Result};

use serde_json::Value;

use crossterm::{
    cursor::{self, MoveTo, MoveToColumn, MoveToNextLine},
    event::{poll, read, Event, KeyCode},
    execute,
    queue,
    style::{Print, ResetColor, Color, SetForegroundColor},
    terminal,
    // ExecutableCommand,
    QueueableCommand,
};

struct RenderData<'a> {
    value: &'a Value,
    prev_node: Option<&'a Value>,
    curr_node: &'a Value,
    index: usize,
    old_indicies: Vec<usize>,
    path: Vec<String>,
}

impl<'a> RenderData<'a> {
    fn new(value: &'a Value) -> RenderData {
        RenderData {
            value,
            prev_node: None,
            curr_node: value,
            index: 0,
            old_indicies: Vec::new(),
            path: Vec::new(),
        }
    }

    fn prev_node(&self) -> Option<&'a Value> {
        self.prev_node
    }

    fn curr_node(&self) -> &'a Value {
        self.curr_node
    }

    fn index(&self) -> usize {
        self.index
    }

    fn path(&self) -> String {
        if self.path.is_empty() {
            return String::new();
        }
        "/".to_owned() + &self.path.join("/")
    }

    fn push_path(&mut self) {
        self.old_indicies.push(self.index);
        self.index = 0;
        // self.path.push(self.current.clone());
    }

    fn pop_path(&mut self) {
        if !self.old_indicies.is_empty() {
            self.index = self.old_indicies.pop().unwrap_or(0);
            self.path.pop();
        }
    }

    fn inc_index(&mut self) {
        if self.index < node_size(self.curr_node) - 1 {
            self.index += 1;
        }
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
    if let Err(e) = main_loop(&mut stdout, &file) {
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
fn main_loop(stdout: &mut io::Stdout, file: &str) -> Result<()> {
    // let (columns, rows) = terminal::size()?;
    let value: Value = serde_json::from_str(file).context("Json Deserialization")?;
    let mut render_data = RenderData::new(&value);

    execute!(stdout, cursor::Hide, terminal::EnterAlternateScreen)?;
    terminal::enable_raw_mode()?;

    loop {
        queue!(stdout, MoveTo(0, 0), terminal::Clear(terminal::ClearType::All))?;
        if let Some(prev) = render_data.prev_node() {
            render_keys(stdout, prev, 0, 0)?;
        }
        stdout.queue(cursor::MoveTo(0, 0))?;
        render_keys(stdout, render_data.curr_node(), render_data.index(), 24)?;

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
        // if event == Event::Key(KeyCode::Char('l').into()) {
            // render_data.push_path();
            // let path = render_data.path();
            // node = value
            //     .pointer(&path)
            //     .context(format!("invalid path: {path}"))?;
        // }
        // if event == Event::Key(KeyCode::Char('h').into()) {
            // render_data.pop_path();
            // let path = render_data.path();
            // node = value
            //     .pointer(&path)
            //     .context(format!("invalid path: {path}"))?;
        // }
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

fn render_keys(stdout: &mut io::Stdout, node: &Value, index: usize, column: u16) -> Result<()> {
    match node {
        Value::Array(vec) => {
            for i in 0..vec.len() {
                if i == index {
                    queue!(stdout, SetForegroundColor(Color::Blue))?;
                }
                queue!(stdout, MoveToColumn(column), Print(i), MoveToNextLine(1))?;
                queue!(stdout, SetForegroundColor(Color::White))?;
            }
        }
        Value::Object(map) => {
            for (i, k) in map.keys().enumerate() {
                if i == index {
                    queue!(stdout, SetForegroundColor(Color::Blue))?;
                }
                queue!(stdout,  cursor::MoveToColumn(column), Print(k), cursor::MoveToNextLine(1))?;
                queue!(stdout, SetForegroundColor(Color::White))?;
            }
        },
        Value::Bool(v) => queue!(stdout, Print(v))?,
        Value::String(v) => queue!(stdout, Print(v))?,
        Value::Number(v) => queue!(stdout, Print(v))?,
        Value::Null => queue!(stdout, Print("null"))?,
    }
    Ok(())
}

fn render_values(stdout: &mut io::Stdout, node: &Value, column: u16) -> Result<()> {
    match node {
        Value::Array(vec) => {
            for v in vec {
                queue!(stdout, cursor::MoveToColumn(column), Print(v), cursor::MoveToNextLine(1))?;
            }
        }
        Value::Object(map) => {
            for v in map.values() {
                queue!(stdout, cursor::MoveToColumn(column), Print(v), cursor::MoveToNextLine(1))?;
            }
        },
        Value::Bool(v) => queue!(stdout, Print(v))?,
        Value::String(v) => queue!(stdout, Print(v))?,
        Value::Number(v) => queue!(stdout, Print(v))?,
        Value::Null => queue!(stdout, Print("null"))?,
    }
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

fn node_size(node: &Value) -> usize {
    match node {
        Value::Object(map) => map.len(),
        Value::Array(arr) => arr.len(),
        Value::Null => 0,
        _ => 1,
    }
}

