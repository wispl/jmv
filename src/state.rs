use serde_json::Value;

#[derive(Copy, Clone)]
pub enum PanelSide {
    Left,
    Middle,
    Right,
}

pub struct PanelState<'a> {
    value: &'a Value,
    text: String,
    column: u16,
    width: u16,
    index: u16,
}

impl<'a> PanelState<'a> {
    pub fn value(&self) -> &Value {
        self.value
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    pub fn column(&self) -> u16 {
        self.column
    }

    pub fn width(&self) -> u16 {
        self.width
    }

    pub fn index(&self) -> u16 {
        self.index
    }
}

pub struct ProgramState<'a> {
    size: (u16, u16),
    value: &'a Value,
    index: usize,
    paths: Vec<String>,
    values: Vec<&'a Value>,
    indices: Vec<usize>,
}

impl<'a> ProgramState<'a> {
    pub fn new(value: &'a Value, size: (u16, u16)) -> ProgramState {
        ProgramState {
            size,
            value,
            index: 0,
            paths: Vec::new(),
            values: Vec::new(),
            indices: Vec::new(),
        }
    }

    pub fn panel_state(&self, panel_side: PanelSide) -> Option<PanelState> {
        let (cols, _) = self.size;
        let width = cols / 3;

        let column = match panel_side {
            PanelSide::Left => 0,
            PanelSide::Middle => width,
            PanelSide::Right => width * 2,
        };

        let index = match panel_side {
            PanelSide::Left => *self.indices.last()?,
            PanelSide::Middle => self.index,
            PanelSide::Right => 0,
        };

        let value = match panel_side {
            PanelSide::Left => self.values.last()?,
            PanelSide::Middle => self.value,
            PanelSide::Right => match self.value {
                Value::Object(map) => map.values().nth(self.index)?,
                Value::Array(arr) => arr.get(self.index)?,
                _ => return None,
            },
        };

        let text = get_value_key(value, index);

        Some(PanelState {
            value,
            text,
            column,
            width,
            index: index.try_into().unwrap(),
        })
    }

    pub fn resize(&mut self, size: (u16, u16)) {
        self.size = size;
    }

    pub fn push_path(&mut self) {
        let value = match self.value {
            Value::Object(map) => map.values().nth(self.index),
            Value::Array(arr) => arr.get(self.index),
            _ => None,
        };

        if let Some(val) = value {
            self.indices.push(self.index);
            self.values.push(self.value);

            self.index = 0;
            self.value = val;

            let text = get_value_key(val, self.index);
            self.paths.push(text);
        }
    }

    pub fn pop_path(&mut self) {
        if !self.paths.is_empty() {
            self.index = self.indices.pop().unwrap();
            self.value = self.values.pop().unwrap();
            self.paths.pop();
        }
    }

    pub fn inc_index(&mut self) {
        if self.index < get_value_size(self.value) - 1 {
            self.index += 1;
        }
    }

    pub fn dec_index(&mut self) {
        self.index = self.index.saturating_sub(1);
    }
}

fn get_value_size(value: &Value) -> usize {
    match value {
        Value::Object(map) => map.len(),
        Value::Array(arr) => arr.len(),
        _ => 1,
    }
}

fn get_value_key(node: &Value, index: usize) -> String {
    match node {
        Value::Object(map) => map
            .keys()
            .nth(index)
            .expect("Index of of bounds")
            .to_string(),
        Value::Array(_) => index.to_string(),
        Value::Bool(v) => v.to_string(),
        Value::String(v) => v.to_owned(),
        Value::Number(v) => v.to_string(),
        Value::Null => "null".to_owned(),
    }
}
