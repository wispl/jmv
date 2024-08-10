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
    execute, queue,
    style::{Color, Print, ResetColor, SetBackgroundColor, SetForegroundColor},
    terminal, QueueableCommand,
};

use crate::state::{ProgramState, PanelSide, PanelState};

mod state;

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
    let value: Value = serde_json::from_str(file).context("Json Deserialization")?;
    let mut program_state = ProgramState::new(&value, terminal::size()?);

    execute!(stdout, cursor::Hide, terminal::EnterAlternateScreen)?;
    terminal::enable_raw_mode()?;

    loop {
        queue!(
            stdout,
            MoveTo(0, 0),
            terminal::Clear(terminal::ClearType::All)
        )?;

        if let Some(left) = program_state.panel_state(PanelSide::Left) {
            render_col(stdout, &left)?;
            render_highlight(stdout, &left)?;
        }
        if let Some(middle) = program_state.panel_state(PanelSide::Middle) {
            render_col(stdout, &middle)?;
            render_highlight(stdout, &middle)?;
        }
        if let Some(right) = program_state.panel_state(PanelSide::Right) {
            render_col(stdout, &right)?;
        }

        stdout.flush()?;

        let event = read()?;
        if let Event::Resize(x, y) = event {
            let (_, new_size) = flush_resize_events((x, y));
            program_state.resize(new_size);
        }

        if event == Event::Key(KeyCode::Char('q').into()) {
            break;
        }
        if event == Event::Key(KeyCode::Char('j').into()) {
            program_state.inc_index();
        }
        if event == Event::Key(KeyCode::Char('k').into()) {
            program_state.dec_index();
        }
        if event == Event::Key(KeyCode::Char('l').into()) {
            program_state.push_path();
        }
        if event == Event::Key(KeyCode::Char('h').into()) {
            program_state.pop_path();
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

fn render_col(stdout: &mut io::Stdout, panel_state: &PanelState) -> Result<()> {
    let column = panel_state.column();
    let width = panel_state.width();

    stdout.queue(cursor::MoveTo(column, 0))?;
    match panel_state.value() {
        Value::Array(vec) => {
            for i in 0..vec.len() {
                queue!(
                    stdout,
                    Print(pad_string(&i.to_string(), width.into())),
                    MoveToNextLine(1),
                    MoveToColumn(column)
                )?;
            }
        }
        Value::Object(map) => {
            for k in map.keys() {
                queue!(
                    stdout,
                    Print(pad_string(k, width.into())),
                    MoveToNextLine(1),
                    MoveToColumn(column)
                )?;
            }
        }
        _ => queue!(stdout, Print(pad_string(&panel_state.text(), width.into())))?,
    }
    Ok(())
}

fn render_highlight(stdout: &mut io::Stdout, panel_state: &PanelState) -> Result<()> {
    queue!(
        stdout,
        cursor::MoveTo(panel_state.column(), panel_state.index()),
        SetBackgroundColor(Color::DarkBlue),
        SetForegroundColor(Color::Black),
        Print(pad_string(panel_state.text(), panel_state.width().into())),
        ResetColor,
    )?;
    Ok(())
}

fn pad_string(str: &str, width: usize) -> String {
    let width = width - 4;
    format!(" {:width$} ", str)
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
