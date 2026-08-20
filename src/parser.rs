use std::collections::HashMap;
use std::fmt;

use log::debug;

use crate::error::{CompError, Position};
use crate::lexer::{Lexer, Token, TokenKind};

// =============== 1. AST ===============
#[derive(Debug, Clone, PartialEq)]
pub enum AST {
    Const(String),
    VariableForAll(String),
    VariableExist(String),
    Function(String, Vec<AST>),
    Predicate(String, Vec<AST>),
    Not(Box<AST>),
    Or(Box<AST>, Box<AST>),
    And(Box<AST>, Box<AST>),
    Impl(Box<AST>, Box<AST>),
    Axiom(String, Box<AST>),
    Schema(String, Box<AST>),
    CriticalError,
}

// =============== 2. ПАРСЕР ===============
pub struct Parser {
    tokens: Vec<Token>,
    pos: usize,
    errors: Vec<CompError>,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Self {
            tokens,
            pos: 0,
            errors: Vec::new(),
        }
    }

    pub fn parse(&mut self) -> (AST, Vec<CompError>) {
        let ast = self.parse_expression();

        // Проверяем, что все токены разобраны
        let new_peek = self.peek();
        if !matches!(new_peek.kind, TokenKind::EOF) {
            self.errors.push(CompError::new(
                format!("Unexcepted token after expression: {}", self.peek()),
                new_peek.pos,
                new_peek.len,
            ))
        }
        (ast, self.errors.clone())
    }

    fn peek(&self) -> Token {
        self.tokens
            .get(self.pos)
            .cloned()
            .unwrap_or(Token::new(TokenKind::EOF, Position::new(), 0))
    }

    fn next(&mut self) -> Token {
        let token = self.peek();
        self.pos += 1;
        token
    }

    // Проверяет что следующий токен соотвествует ожидаемому
    fn expect(&mut self, expected: TokenKind, get_error: bool) -> bool {
        if self.peek().kind == expected {
            if get_error {
                self.next();
            }
            true
        } else {
            if get_error {
                self.errors.push(CompError::new(
                    format!("Expected '{}', got '{}'", expected, self.peek().kind),
                    self.peek().pos,
                    self.peek().len,
                ));
                self.next();
            }
            false
        }
    }

    // ================== ОСНОВНАЯ ЧАСТЬ ====================
    // expression -> axiom IDENT: fol | schema IDENT: fol
    fn parse_expression(&mut self) -> AST {
        // Проверяем первое слово
        let mut axiom_or_schema: Option<TokenKind> = None;
        if !self.expect(TokenKind::Axiom, false) && !self.expect(TokenKind::Schema, false) {
            self.errors.push(CompError::new(
                format!(
                    "Critical Error: expected 'axiom' or 'schema', got '{}'",
                    self.peek().kind
                ),
                self.peek().pos,
                self.peek().len,
            ));
            self.next();
            return AST::CriticalError;
        } else {
            axiom_or_schema = Some(self.peek().kind);
        }
        self.next();

        let mut ident: Option<String> = None;
        if let TokenKind::Ident(s) = self.peek().kind {
            ident = Some(s.clone());
            self.next();
        } else {
            self.errors.push(CompError::new(
                format!(
                    "Critical Error: Expected identificator, got '{}'",
                    self.peek().kind
                ),
                self.peek().pos,
                self.peek().len,
            ));
            self.next();
            return AST::CriticalError;
        }

        self.expect(TokenKind::Colon, true); // Проверяем на двоеточие

        let mut vars: HashMap<String, bool> = HashMap::new();

        match axiom_or_schema.unwrap() {
            TokenKind::Axiom => AST::Axiom(ident.unwrap(), Box::new(self.parse_fol(&mut vars))),
            TokenKind::Schema => AST::Schema(ident.unwrap(), Box::new(self.parse_fol(&mut vars))),
            _ => unreachable!(),
        }
    }

    // QUANTORS -> (((forall | exist) IDENT(, IDENT)*:)* term | ((forall | exist) IDENT(, IDENT)*:)* (fol))
    fn parse_quantors<F>(&mut self, vars: &mut HashMap<String, bool>, mut f: F) -> AST
    where
        F: FnMut(&mut Parser, &mut HashMap<String, bool>) -> AST,
    {
        if !self.expect(TokenKind::ForAll, false) && !self.expect(TokenKind::Exist, false) {
            return f(self, vars);
        }

        while let kind @ (TokenKind::ForAll | TokenKind::Exist) = self.peek().kind {
            self.next(); // Пожираем

            // Проверяем на первый индифекатор
            if let TokenKind::Ident(ident) = self.peek().kind {
                match kind {
                    TokenKind::ForAll => vars.insert(ident, true),
                    TokenKind::Exist => vars.insert(ident, false),
                    _ => unreachable!(),
                };
                self.next();
            } else {
                self.errors.push(CompError::new(
                    format!(
                        "Critical Error: Expected identificator, got '{}'",
                        self.peek().kind
                    ),
                    self.peek().pos,
                    self.peek().len,
                ));
                self.next();
                return AST::CriticalError;
            }

            // Проверяем на последующие
            while let TokenKind::Dot = self.peek().kind {
                self.next();
                if let TokenKind::Ident(ident) = self.peek().kind {
                    match kind {
                        TokenKind::ForAll => vars.insert(ident, true),
                        TokenKind::Exist => vars.insert(ident, false),
                        _ => unreachable!(),
                    };
                    self.next();
                } else {
                    self.expect(TokenKind::Ident(String::from("identificator")), true);
                }
            }

            self.expect(TokenKind::Colon, true);

            if self.peek().kind == TokenKind::LParen {
                self.expect(TokenKind::LParen, true);
                let result = self.parse_fol(vars);
                self.expect(TokenKind::RParen, true);
                return result;
            }
        }
        return f(self, vars);
    }

    // fol -> QUANTORS term ('->' QUANTORS term)*
    fn parse_fol(&mut self, vars: &mut HashMap<String, bool>) -> AST {
        self.parse_quantors(vars, Parser::parse_fol);
        let mut left = self.parse_term(vars);

        while let TokenKind::Impl = self.peek().kind {
            self.next(); // Пожираем оператор
            self.parse_quantors(vars, Parser::parse_fol);
            left = AST::Impl(Box::new(left), Box::new(self.parse_term(vars)));
        }

        left
    }

    // term -> QUANTORS term_or (/\ QUANTORS term_or)*
    fn parse_term(&mut self, vars: &mut HashMap<String, bool>) -> AST {
        self.parse_quantors(vars, Parser::parse_term);
        let mut left = self.parse_term_or(vars);

        while let TokenKind::And = self.peek().kind {
            self.next(); // Пожираем оператор
            self.parse_quantors(vars, Parser::parse_term);
            left = AST::And(Box::new(left), Box::new(self.parse_term_or(vars)));
        }

        left
    }

    // term_or -> QUANTORS literal (\/ QUANTORS literal)*
    fn parse_term_or(&mut self, vars: &mut HashMap<String, bool>) -> AST {
        self.parse_quantors(vars, Parser::parse_term_or);
        let mut left = self.parse_literal(vars);

        while let TokenKind::Or = self.peek().kind {
            self.next(); // Пожираем оператор
            self.parse_quantors(vars, Parser::parse_term_or);
            left = AST::Or(Box::new(left), Box::new(self.parse_literal(vars)));
        }

        left
    }

    // literal -> QUANTORS (NOT)? IDENT '(' arg (',' arg)* ')'
    fn parse_literal(&mut self, vars: &mut HashMap<String, bool>) -> AST {
        self.parse_quantors(vars, Parser::parse_literal);

        let mut is_not = false;
        if self.peek().kind == TokenKind::Not {
            is_not = true;
            self.next();
        }

        let mut ident: Option<String> = None;
        if let TokenKind::Ident(s) = self.peek().kind {
            ident = Some(s.clone());
            self.next();
        } else {
            self.errors.push(CompError::new(
                format!(
                    "Critical Error: Expected identificator, got '{}'",
                    self.peek().kind
                ),
                self.peek().pos,
                self.peek().len,
            ));
            self.next();
            return AST::CriticalError;
        }

        self.expect(TokenKind::LParen, true);
        let mut args: Vec<AST> = Vec::new();
        args.push(self.parse_arg(vars));
        while let TokenKind::Dot = self.peek().kind {
            self.next();
            args.push(self.parse_arg(vars));
        }
        self.expect(TokenKind::RParen, true);

        if is_not {
            return AST::Not(Box::new(AST::Predicate(ident.unwrap(), args)));
        }
        AST::Predicate(ident.unwrap(), args)
    }

    // arg -> arith | IDENT '(' arg (',' arg)* ')'
    fn parse_arg(&mut self, vars: &mut HashMap<String, bool>) -> AST {
        let mut is_f = false;
        if let TokenKind::Ident(s) = self.peek().kind {
            self.next();
            if self.peek().kind == TokenKind::LParen {
                is_f = true;
            }
            self.pos -= 1;
        }

        if is_f {
            if let TokenKind::Ident(ident) = self.peek().kind {
                self.next();
                self.next();

                let mut args: Vec<AST> = Vec::new();
                args.push(self.parse_arg(vars));
                while let TokenKind::Dot = self.peek().kind {
                    self.next();
                    args.push(self.parse_arg(vars));
                }
                self.expect(TokenKind::RParen, true);

                return AST::Function(ident, args);
            } else {
                unreachable!()
            }
        }

        self.parse_arith(vars)
    }

    // arith -> arith_term ((+ | -) term)*
    fn parse_arith(&mut self, vars: &mut HashMap<String, bool>) -> AST {
        let mut left = self.parse_arith_term(vars);

        while let kind @ (TokenKind::Plus | TokenKind::Minus) = self.peek().kind {
            self.next();
            let right = self.parse_arith_term(vars);
            left = match kind {
                TokenKind::Plus => AST::Function(String::from("+"), vec![left, right]),
                TokenKind::Minus => AST::Function(String::from("-"), vec![left, right]),
                _ => unreachable!(),
            };
        }

        left
    }

    // arith_term -> factor ((* | /) factor)*
    fn parse_arith_term(&mut self, vars: &mut HashMap<String, bool>) -> AST {
        let mut left = self.parse_factor(vars);

        while let kind @ (TokenKind::Star | TokenKind::Slash) = self.peek().kind {
            self.next();
            let right = self.parse_factor(vars);
            left = match kind {
                TokenKind::Star => AST::Function(String::from("*"), vec![left, right]),
                TokenKind::Slash => AST::Function(String::from("/"), vec![left, right]),
                _ => unreachable!(),
            };
        }

        left
    }

    // factor -> NUMBER | IDENT | '(' fol ')'
    fn parse_factor(&mut self, vars: &mut HashMap<String, bool>) -> AST {
        match self.peek().kind {
            TokenKind::Number(i) => {
                self.next();
                AST::Const(i.to_string())
            }
            TokenKind::Ident(s) => {
                self.next();
                match vars.get(&s) {
                    Some(value) => match value {
                        true => AST::VariableForAll(s),
                        false => AST::VariableForAll(s),
                    },
                    None => AST::Const(s),
                }
            }
            TokenKind::LParen => {
                self.next();
                let inner = self.parse_fol(vars);
                self.expect(TokenKind::RParen, true);
                inner
            }
            _ => {
                self.errors.push(CompError::new(
                    format!("Critical Error: expected number or identificator or expression in parens, got '{}'", self.peek().kind),
                    self.peek().pos,
                    self.peek().len
                ));
                self.next();
                return AST::CriticalError;
            }
        }
    }
}

// ТЕСТЫ ПАРСЕРА
#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn test_parser_without_errors() {
        let code = "schema IND: P(0) /\\ forall n: (P(n) -> P(S(n))) -> forall n: P(n)";
        let lex = Lexer::new(&code);
        let (tokens, _) = lex.tokenize();

        println!("{:?}", tokens);

        let mut parser = Parser::new(tokens);
        let (ast, errors) = parser.parse();

        println!("{:?}", errors);

        assert_eq!(
            ast,
            AST::Schema(
                String::from("IND"),
                Box::new(AST::Impl(
                    Box::new(AST::And(
                        Box::new(AST::Predicate(
                            String::from("P"),
                            vec![AST::Const(String::from("0"))]
                        )),
                        Box::new(AST::Impl(
                            Box::new(AST::Predicate(
                                String::from("P"),
                                vec![AST::VariableForAll(String::from("n"))]
                            )),
                            Box::new(AST::Predicate(
                                String::from("P"),
                                vec![AST::Function(
                                    String::from("S"),
                                    vec![AST::VariableForAll(String::from("n"))]
                                )]
                            ))
                        ))
                    )),
                    Box::new(AST::Predicate(
                        String::from("P"),
                        vec![AST::VariableForAll(String::from("n"))]
                    ))
                ))
            )
        );
    }
}
