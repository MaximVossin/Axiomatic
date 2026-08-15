// ============= 1. ПОЗИЦИЯ В ТЕКУЩЕМ КОДЕ =============
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Position {
    pub line: usize,
    pub column: usize,
    pub index: usize,
}

impl Position {
    pub fn new() -> Self {
        Self {
            line: 1,
            column: 1,
            index: 0,
        }
    }
}

// ============= 2. КАСТОМНАЯ ОШИБКА =============
#[derive(Debug, Clone)]
pub struct CompError {
    pub message: String,
    pub pos: Position,
    pub len: usize, // сколько символов вызвало ошибку
}

impl CompError {
    pub fn new(message: impl Into<String>, pos: Position, len: usize) -> Self {
        Self {
            message: message.into(),
            pos,
            len
        }
    }

    // Красивый вывод с подсветкой места ошибки
    pub fn pretty_print(&self, source: &str) -> String {
        let lines: Vec<&str> = source.lines().collect();
        let line_idx = self.pos.line - 1;

        if line_idx >= lines.len() {
            return format!("Error at position {}: {}", self.pos.line, self.message);
        }

        let line = lines[line_idx];
        let column_start = self.pos.column - 1;
        let column_end = (self.pos.column + self.len - 1).min(line.len());

        let mut result = String::new();
        result.push_str(&format!("--> {}:{}:{}\n", "input", self.pos.line, self.pos.column));
        result.push_str(&format!(" {}\n", line));

        // Рисуем стрелочки-указатели
        if column_start < line.len() {
            result.push_str(" ");
            for _ in 0..column_start {
                result.push(' ');
            }
            for _ in column_start..column_end.min(line.len()) {
                result.push('^');
            }
            result.push('\n');
        }

        result.push_str(&format!("error: {}\n", self.message));
        result
    }
}