use std::iter::Peekable;
use std::result;
use std::str;
use thiserror::Error;

use crate::lexer::{Token, TokenError, Tokenizer};

#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    Atom(Token),
    ListVal(Vec<Expr>),
}

#[derive(Clone, Debug, PartialEq, Error)]
pub enum ParseError {
    #[error("Error reading tokens")]
    TokenError(#[from] TokenError),
    #[error("Unexpected token, {0:?}")]
    Unexpected(Token),
    #[error("Unexpected EOF")]
    UnexpectedEOF,
}

#[derive(Debug)]
pub struct Parser<'a> {
    tokenizer: Peekable<Tokenizer<'a>>,
}

pub type Result<T> = result::Result<T, ParseError>;

impl<'a> Parser<'a> {
    pub fn new(input: &'a str) -> Self {
        Parser {
            tokenizer: Tokenizer::new(input).peekable(),
        }
    }

    // Jason's attempt
    fn read_from_tokens(&mut self) -> Result<Expr> {
        let mut stack: Vec<Vec<Expr>> = Vec::new();
        let mut current_frame: Vec<Expr> = Vec::new();

        loop {
            match self.tokenizer.next() {
                Some(Ok(t)) => match t {
                    Token::OpenParen => {
                        stack.push(current_frame);
                        current_frame = Vec::new();
                    }
                    Token::CloseParen => {
                        if let Some(mut prev_frame) = stack.pop() {
                            prev_frame.push(Expr::ListVal(current_frame));
                            current_frame = prev_frame;
                        } else {
                            return Ok(Expr::ListVal(current_frame));
                        }
                    }
                    tok => {
                        current_frame.push(Expr::Atom(tok));
                    }
                },
                Some(Err(e)) => return Err(ParseError::TokenError(e)),
                None => return Err(ParseError::UnexpectedEOF),
            }
        }
    }
}

impl<'a> Iterator for Parser<'a> {
    type Item = Result<Expr>;

    fn next(&mut self) -> Option<Self::Item> {
        self.tokenizer.next().map(|res| match res {
            Err(e) => Err(ParseError::TokenError(e)),
            Ok(tok) => match tok {
                Token::OpenParen => self.read_from_tokens(),
                tok if tok.is_reserved_keyword() => Err(ParseError::Unexpected(tok)),
                tok => Ok(Expr::Atom(tok)),
            },
        })
    }
}

#[cfg(test)]
mod parser_tests {
    use super::Expr::*;
    use super::Token::*;
    use super::*;

    #[test]
    fn test_empty() {
        assert_parse("", &[]);
        assert_parse("()", &[ListVal(vec![])]);
    }

    #[test]
    fn test_multi_parse() {
        assert_parse(
            "a b +",
            &[
                Atom(Identifier("a".to_string())),
                Atom(Identifier("b".to_string())),
                Atom(Identifier("+".to_string())),
            ],
        );
        assert_parse(
            "a b (lambda  1 (+ 2 3.5))",
            &[
                Atom(Identifier("a".to_string())),
                Atom(Identifier("b".to_string())),
                ListVal(vec![
                    Atom(Lambda),
                    Atom(NumberLiteral(1.0)),
                    ListVal(vec![
                        Atom(Identifier("+".to_string())),
                        Atom(NumberLiteral(2.0)),
                        Atom(NumberLiteral(3.5)),
                    ]),
                ]),
            ],
        )
    }
    #[test]
    fn test_parse_simple() {
        assert_parse(
            "(+ 1 2 3) (- 4 3)",
            &[
                ListVal(vec![
                    Atom(Identifier("+".to_string())),
                    Atom(NumberLiteral(1.0)),
                    Atom(NumberLiteral(2.0)),
                    Atom(NumberLiteral(3.0)),
                ]),
                ListVal(vec![
                    Atom(Identifier("-".to_string())),
                    Atom(NumberLiteral(4.0)),
                    Atom(NumberLiteral(3.0)),
                ]),
            ],
        );
    }
    #[test]
    fn test_parse_nested() {
        assert_parse(
            "(+ 1 (foo (bar 2 3)))",
            &[ListVal(vec![
                Atom(Identifier("+".to_string())),
                Atom(NumberLiteral(1.0)),
                ListVal(vec![
                    Atom(Identifier("foo".to_string())),
                    ListVal(vec![
                        Atom(Identifier("bar".to_owned())),
                        Atom(NumberLiteral(2.0)),
                        Atom(NumberLiteral(3.0)),
                    ]),
                ]),
            ])],
        );
        assert_parse(
            "(+ 1 (+ 2 3) (foo (bar 2 3)))",
            &[ListVal(vec![
                Atom(Identifier("+".to_string())),
                Atom(NumberLiteral(1.0)),
                ListVal(vec![
                    Atom(Identifier("+".to_string())),
                    Atom(NumberLiteral(2.0)),
                    Atom(NumberLiteral(3.0)),
                ]),
                ListVal(vec![
                    Atom(Identifier("foo".to_string())),
                    ListVal(vec![
                        Atom(Identifier("bar".to_owned())),
                        Atom(NumberLiteral(2.0)),
                        Atom(NumberLiteral(3.0)),
                    ]),
                ]),
            ])],
        );
        assert_parse(
            "(+ 1 (+ 2 3) (foo (+ (bar 1 1) 3) 5))",
            &[ListVal(vec![
                Atom(Identifier("+".to_string())),
                Atom(NumberLiteral(1.0)),
                ListVal(vec![
                    Atom(Identifier("+".to_string())),
                    Atom(NumberLiteral(2.0)),
                    Atom(NumberLiteral(3.0)),
                ]),
                ListVal(vec![
                    Atom(Identifier("foo".to_string())),
                    ListVal(vec![
                        Atom(Identifier("+".to_string())),
                        ListVal(vec![
                            Atom(Identifier("bar".to_string())),
                            Atom(NumberLiteral(1.0)),
                            Atom(NumberLiteral(1.0)),
                        ]),
                        Atom(NumberLiteral(3.0)),
                    ]),
                    Atom(NumberLiteral(5.0)),
                ]),
            ])],
        );
    }
    #[test]
    fn test_parse_specials() {
        assert_parse(
            "(define (foo a b) (+ (- a 1) b))",
            &[ListVal(vec![
                Atom(Define),
                ListVal(vec![
                    Atom(Identifier("foo".to_string())),
                    Atom(Identifier("a".to_string())),
                    Atom(Identifier("b".to_string())),
                ]),
                ListVal(vec![
                    Atom(Identifier("+".to_string())),
                    ListVal(vec![
                        Atom(Identifier("-".to_string())),
                        Atom(Identifier("a".to_string())),
                        Atom(NumberLiteral(1.0)),
                    ]),
                    Atom(Identifier("b".to_string())),
                ]),
            ])],
        );

        assert_parse(
            "(if   #t     1 2)",
            &[ListVal(vec![
                Atom(If),
                Atom(BooleanLiteral(true)),
                Atom(NumberLiteral(1.0)),
                Atom(NumberLiteral(2.0)),
            ])],
        );
        assert_parse(
            "(lambda (a b) (+ a b)) (- 1 2) (\"dumpsterfire\")",
            &[
                ListVal(vec![
                    Atom(Lambda),
                    ListVal(vec![
                        Atom(Identifier("a".to_string())),
                        Atom(Identifier("b".to_string())),
                    ]),
                    ListVal(vec![
                        Atom(Identifier("+".to_string())),
                        Atom(Identifier("a".to_string())),
                        Atom(Identifier("b".to_string())),
                    ]),
                ]),
                ListVal(vec![
                    Atom(Identifier("-".to_string())),
                    Atom(NumberLiteral(1.0)),
                    Atom(NumberLiteral(2.0)),
                ]),
                ListVal(vec![Atom(StringLiteral("dumpsterfire".to_string()))]),
            ],
        );
    }

    #[test]
    fn test_error() {
        assert_parse_err("(", ParseError::UnexpectedEOF);
        assert_parse_err("(abc", ParseError::UnexpectedEOF);
        assert_parse_err("(ab 1 2", ParseError::UnexpectedEOF);
        assert_parse_err("((((ab 1 2) (", ParseError::UnexpectedEOF);
        assert_parse_err("())", ParseError::Unexpected(Token::CloseParen));
        assert_parse_err("() ((((", ParseError::UnexpectedEOF);
    }

    fn assert_parse_err(s: &str, err: ParseError) {
        let a: Result<Vec<Expr>> = Parser::new(s).collect();
        assert_eq!(a, Err(err));
    }

    fn assert_parse(s: &str, result: &[Expr]) {
        let a: Result<Vec<Expr>> = Parser::new(s).collect();
        let a = a.unwrap();
        assert_eq!(a, result);
    }
}