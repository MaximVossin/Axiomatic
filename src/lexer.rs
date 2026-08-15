use std::fmt;
use crate::error::{Position, CompError};

// ============= 1. ТОКЕН С ПРИВЯЗКОЙ К ПОЗИЦИИ =============
#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub kind: TokenKind,
    pub pos: Position,
    pub len: usize,  // длина токена в символах
}

impl Token {
    pub fn new(kind: TokenKind, pos: Position, len: usize) -> Self {
        Self { kind, pos, len }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    // Ключевые слова
    Axiom,
    Schema,

    // Базовые термы FOL
    Ident(String),
    ForAll,
    Exist,
    And,
    Or,
    Not,
    Impl,
    LParen,
    RParen,

    // Встроенные операторы и пунтакция
    Number(i64),
    Eq,
    Plus,
    Minus,
    Star,
    Slash,
    Dot,
    Point,

    // Специальные
    EOF,
}

impl fmt::Display for TokenKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TokenKind::Axiom => write!(f, "axiom"),
            TokenKind::Schema => write!(f, "schema"),
            TokenKind::Ident(name) => write!(f, "{}", name),
            TokenKind::ForAll => write!(f, "forall"),
            TokenKind::Exist => write!(f, "exist"),
            TokenKind::And => write!(f, "/\\"),
            TokenKind::Or => write!(f, "\\/"),
            TokenKind::Not => write!(f, "~"),
            TokenKind::Impl => write!(f, "->"),
            TokenKind::LParen => write!(f, "("),
            TokenKind::RParen => write!(f, ")"),
            TokenKind::Number(n) => write!(f, "{}", n),
            TokenKind::Eq => write!(f, "="),
            TokenKind::Plus => write!(f, "+"),
            TokenKind::Minus => write!(f, "-"),
            TokenKind::Star => write!(f, "*"),
            TokenKind::Slash => write!(f, "/"),
            TokenKind::Dot => write!(f, ","),
            TokenKind::Point => write!(f, "."),
            TokenKind::EOF => write!(f, "EOF"),
        }
    }
}

pub type LexResult<T> = Result<T, CompError>;

// ============= 2. ЛЕКСЕР =============
pub struct Lexer {
    source: String,
    chars: Vec<char>,
    pos: Position,
    tokens: Vec<Token>,
    errors: Vec<CompError>,
}

impl Lexer {
    pub fn new(source: &str) -> Self {
        Self {
            source: source.to_string(),
            chars: source.chars().collect(),
            pos: Position::new(),
            tokens: Vec::new(),
            errors: Vec::new(),
        }
    }

    pub fn tokenize(mut self) -> (Vec<Token>, Vec<CompError>) {
        loop {
            match self.next_token() {
                Ok(token) => {
                    let is_EOF = matches!(token.kind, TokenKind::EOF);
                    self.tokens.push(token);
                    if is_EOF {
                        break;
                    }
                }
                Err(err) => {
                    self.errors.push(err);
                    // При ошибке сдвигаемся на 1 символ, чтобы не зациклиться
                    self.pos.index += 1;
                    if self.pos.index >= self.chars.len() {
                        break;
                    }
                }
            }
        }
        (self.tokens, self.errors)
    }

    // Вспомогательные методы для навигации
    fn peek_char(&self) -> Option<char> {
        self.chars.get(self.pos.index).copied()
    }

    fn next_char(&mut self) -> Option<char> {
        let ch = self.peek_char();
        if let Some(c) = ch {
            self.pos.index +=  1;
            if c == '\n' {
                self.pos.line += 1;
                self.pos.column += 1;
            } else {
                self.pos.column += 1;
            }
        }
        ch
    }

    fn skip_whitespace(&mut self) {
        while let Some(ch) = self.peek_char() {
            if !ch.is_whitespace() {
                break;
            }
            self.next_char();
        }
    }

    fn make_token(&self, kind: TokenKind, start_pos: Position, len: usize) -> Token {
        Token::new(kind, start_pos, len)
    }

    // ============= ГЛАВНЫЙ МЕТОД =============
    fn next_token(&mut self) -> LexResult<Token> {
        self.skip_whitespace();

        let start_pos = self.pos;
        let ch = match self.next_char() {
            Some(c) => c,
            None => {
                return Ok(self.make_token(TokenKind::EOF, start_pos, 0));
            }
        };

        match ch {
            '=' => Ok(self.make_token(TokenKind::Eq, start_pos, 1)),
            '+' => Ok(self.make_token(TokenKind::Plus, start_pos, 1)),
            '-' => self.read_potential_many_digit_operator(start_pos, ch, vec!["-", "->"], vec![TokenKind::Minus, TokenKind::Impl]),
            '*' => Ok(self.make_token(TokenKind::Star, start_pos, 1)),
            '/' => self.read_potential_many_digit_operator(start_pos, ch, vec!["/", "/\\"], vec![TokenKind::Slash, TokenKind::And]),
            ',' => Ok(self.make_token(TokenKind::Dot, start_pos, 1)),
            '.' => Ok(self.make_token(TokenKind::Point, start_pos, 1)),
            '(' => Ok(self.make_token(TokenKind::LParen, start_pos, 1)),
            ')' => Ok(self.make_token(TokenKind::RParen, start_pos, 1)),
            '~' => Ok(self.make_token(TokenKind::Not, start_pos, 1)),

            '\\' => self.read_potential_many_digit_operator(start_pos, ch, vec!["\\/"], vec![TokenKind::Or]),

            'a'..='z' | 'A'..='Z' | '_' => self.read_identifier_or_keyword(start_pos, ch),

            '0'..='9' => self.read_number(start_pos, ch),

            _ => Err(CompError::new(
                format!("unexpected character '{}'", ch),
                start_pos,
                1,
            )),
        }
    }

    // Для потенциально многозначных операторов
    fn read_potential_many_digit_operator(&mut self, start_pos: Position, first: char, 
                                          candidates: Vec<&str>, candidates_kind: Vec<TokenKind>) -> LexResult<Token> {
        let mut operator = String::from(first);
        let mut len = 1;
        let mut last_token: Option<Token> = None; // токен по умолчанию

        if candidates.iter().any(|s| *s == operator) {
            let idx = candidates.iter().position(|&item| item == operator);
            last_token = Some(self.make_token(candidates_kind[idx.unwrap()].clone(), start_pos, len));
        }

        // ищем максимальное вхождение
        while let Some(next) = self.peek_char() {
            let operator_new = format!("{}{}", operator, next);
            if candidates.iter().any(|s| s.starts_with(&operator_new)) {
                if candidates.iter().any(|s| *s == operator_new) {
                    let idx = candidates.iter().position(|&item| item == operator_new);
                    last_token = Some(self.make_token(candidates_kind[idx.unwrap()].clone(), start_pos, len));
                }

                operator.push(next);
                self.next_char();
                len += 1;
            }
            else {
                break;
            }
        }

        match last_token {
            None => {
                self.pos = start_pos;
                Err(CompError::new(
                    format!("unexpected character '{}'", first),
                    start_pos,
                    1,
                ))
            },
            Some(t) => {
                self.pos = t.pos;
                for _ in 1..t.len {
                    self.next_char();
                }
                Ok(t)
            }
        }
    }
    // Читаем идентификатор
    fn read_identifier_or_keyword(&mut self, start_pos: Position, first: char) -> LexResult<Token> {
        let mut ident = String::from(first);
        let mut len = 1;

        while let Some(next) = self.peek_char() {
            if next.is_alphanumeric() || next == '_' {
                ident.push(next);
                self.next_char();
                len += 1;
            }
            else {
                break;
            }
        }

        let kind = match ident.as_str() {
            "axiom" => TokenKind::Axiom,
            "schema" => TokenKind::Schema,
            "forall" => TokenKind::ForAll,
            "exist" => TokenKind::Exist,
            _ => TokenKind::Ident(ident)
        };

        Ok(self.make_token(kind, start_pos, len))
    }

    fn read_number(&mut self, start_pos: Position, first: char) -> LexResult<Token> {
        let mut number = String::from(first);
        let mut len = 1;

        while let Some(next) = self.peek_char() {
            if next.is_ascii_digit() {
                number.push(next);
                self.next_char();
                len += 1;
            }
            else {
                break;
            }
        }

        match number.parse::<i64>() {
            Ok(val) => Ok(self.make_token(TokenKind::Number(val), start_pos, len)),
            Err(_) => Err(CompError::new(
                format!("number '{}' is too large for 64-bit number", number),
                start_pos,
                len,
            )),
        }
    }
}