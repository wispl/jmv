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
    QueueableCommand,
};

struct RenderData<'a> {
    curr_node: &'a Value,
    prev_str: String,
    index: usize,
    old_indicies: Vec<usize>,
    path: Vec<&'a Value>,
}

impl<'a> RenderData<'a> {
    fn new(value: &'a Value) -> RenderData {
        RenderData {
            curr_node: value,
            index: 0,
            prev_str: String::new(),
            old_indicies: Vec::new(),
            path: Vec::new(),
        }
    }

    fn indexed_str(&self) -> String {
        match self.curr_node {
            Value::Object(map) => map.iter().nth(self.index).map(|(k, _)| k.to_string()).unwrap(),
            Value::Array(_) => self.index.to_string(),
            Value::Bool(v) => v.to_string(),
            Value::String(v) => v.to_owned(),
            Value::Number(v) => v.to_string(),
            Value::Null => "null".to_owned(),
        }
    }

    fn prev_str(&self) -> &str {
        &self.prev_str
    }

    fn indexed_val(&self) -> Option<&'a Value> {
        match self.curr_node {
            Value::Object(map) => map.iter().nth(self.index).map(|(_, v)| v),
            Value::Array(arr) => arr.get(self.index),
            _ => None
        }
    }

    fn prev_node(&self) -> Option<&&'a Value> {
        self.path.last()
    }

    fn curr_node(&self) -> &'a Value {
        self.curr_node
    }

    fn index(&self) -> usize {
        self.index
    }

    fn prev_index(&self) -> Option<&usize> {
        self.old_indicies.last()
    }

    fn push_path(&mut self) {
        if let Some(val) = self.indexed_val() {
            self.path.push(self.curr_node);
            self.old_indicies.push(self.index);

            self.prev_str = self.indexed_str();
            self.index = 0;
            self.curr_node = val;
        }
    }

    fn pop_path(&mut self) {
        if !self.path.is_empty() {
            self.index = self.old_indicies.pop().unwrap();
            self.curr_node = self.path.pop().unwrap();
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
    let (columns, rows) = terminal::size()?;
    let column_length = columns / 3;

    let value: Value = serde_json::from_str(file).context("Json Deserialization")?;
    let mut render_data = RenderData::new(&value);

    execute!(stdout, cursor::Hide, terminal::EnterAlternateScreen)?;
    terminal::enable_raw_mode()?;

    loop {
        queue!(stdout, MoveTo(0, 0), terminal::Clear(terminal::ClearType::All))?;

        if let Some(prev) = render_data.prev_node() {
            render_keys(stdout, prev, 0)?;
        }
        if let Some(index) = render_data.prev_index() {
            queue!(
                stdout,
                cursor::MoveTo(0, (*index).try_into().unwrap()),
                SetForegroundColor(Color::Blue),
                Print(render_data.prev_str()),
                SetForegroundColor(Color::White)
            )?;
        }

        render_keys(stdout, render_data.curr_node(), column_length)?;
        queue!(
            stdout,
            cursor::MoveTo(column_length, render_data.index().try_into().unwrap()),
            SetForegroundColor(Color::Blue),
            Print(render_data.indexed_str()),
            SetForegroundColor(Color::White)
        )?;

        if let Some(val) = render_data.indexed_val() {
            render_keys(stdout, val, column_length * 2)?;
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
        }
        if event == Event::Key(KeyCode::Char('h').into()) {
            render_data.pop_path();
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

fn render_keys(stdout: &mut io::Stdout, node: &Value, column: u16) -> Result<()> {
    stdout.queue(cursor::MoveTo(column, 0))?;
    match node {
        Value::Array(vec) => {
            for i in 0..vec.len() {
                queue!(stdout, Print(i), MoveToNextLine(1), MoveToColumn(column))?;
            }
        }
        Value::Object(map) => {
            for k in map.keys() {
                queue!(stdout, Print(k), MoveToNextLine(1), MoveToColumn(column))?;
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

