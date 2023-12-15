
use std::error::Error;
use std::str::FromStr;
use std::fmt;
use itertools::Itertools;

use crate::errors::{LocalizableError, LocalizedError};


#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Operator{
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Pow,
    Let,
    Fn,
    Comma,
    Colon,
    Semicolon,
    Assign,
    LParen,
    RParen,
    LCurl,
    RCurl,
}


#[derive(Debug, Clone, PartialEq)]
pub enum Type{
    Operator(Operator),
    Literal(String),
}

#[derive(Debug)]
pub struct Token{
    pub type_: Type,
    pub location: Location,
}

#[derive(Debug)]
pub struct TokenError {
    pub message: String,
}

impl Error for TokenError {}

impl fmt::Display for TokenError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "TokenizerError: {}", self.message)
    }
}



impl FromStr for Type {
    type Err = TokenError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        use Type::Operator as Op;
        match s {
            "+" => Ok(Op(Operator::Add)),
            "-" => Ok(Op(Operator::Sub)),
            "*" => Ok(Op(Operator::Mul)),
            "/" => Ok(Op(Operator::Div)),
            "%" => Ok(Op(Operator::Mod)),
            "**" => Ok(Op(Operator::Pow)),
            "," => Ok(Op(Operator::Comma)),
            ":" => Ok(Op(Operator::Colon)),
            ";" => Ok(Op(Operator::Semicolon)),
            "=" => Ok(Op(Operator::Assign)),
            "(" => Ok(Op(Operator::LParen)),
            ")" => Ok(Op(Operator::RParen)),
            "{" => Ok(Op(Operator::LCurl)),
            "}" => Ok(Op(Operator::RCurl)),
            "let" => Ok(Op(Operator::Let)),
            "fn" => Ok(Op(Operator::Fn)), 
            _ if s.chars().all(char::is_alphanumeric) => Ok(Type::Literal(s.to_owned())),
            _ => Err(TokenError {
                message: format!("Invalid token: {}", s),
            }),
        }
    }
}


#[derive(Debug, Clone, Copy)]
pub struct Location {
    pub line: usize,
    pub column: usize,
}

impl Default for Location {
    fn default() -> Self {
        Self {
            line: 0,
            column: 0,
        }
    }
}

pub struct Tokenizer<I> {
    lines: I,
    tokens: Vec<Token>,
    location: Location,
    error: Option<LocalizedError>,
}

impl <I, S> Tokenizer<I> 
where I: Iterator<Item = S>, S: AsRef<str>
{
    fn new(lines: I) -> Self {
        Self {
            lines,
            location: Location { line: 0, column: 0 },
            tokens: Vec::new(),
            error: None,
        }
    }

    pub fn error(self) -> Option<LocalizedError> {
        self.error
    }
}

// TODO
// pub trait TokenizerExt {
//     fn peek(&mut self) -> Option<&Token>;
//     fn peek_type(&mut self) -> Option<&Type>;
//     fn peek_operator(&mut self) -> Option<&Operator>;
//     fn peek_literal(&mut self) -> Option<&String>;
//     fn next_if(&mut self, type_: Type) -> bool;
//     fn next_if_operator(&mut self, operator: Operator) -> bool;
//     fn next_if_literal(&mut self, literal: &str) -> bool;
//     fn next(&mut self) -> Option<Token>;
//     fn expect(&mut self, type_: Type) -> Result<Token, CompileError>;
//     fn expect_operator(&mut self, operator: Operator) -> Result<Token, CompileError>;
//     fn expect_literal(&mut self, literal: &str) -> Result<Token, CompileError>;
// }



pub fn slice_into_snippets<'a>(line: &'a str) -> impl Iterator<Item = &'a str> {
    let category = |c: char| -> u8 {
        if c.is_whitespace() { 0 }
        else if c.is_alphanumeric() { 1 }
        else if c.is_ascii_punctuation() { 
            match c {
                '(' => 2,
                ')' => 3,
                '{' => 4,
                '}' => 5,
                ';' => 6,
                ':' => 7,
                '=' => 8,
                '+' => 9,
                '-' => 10,
                '*' => 11,
                '/' => 12,
                '%' => 13,
                ',' => 14,
                _ => 99,
            }
        }
        else { 99 }
    };

    line
        .char_indices()
        .inspect(|(_, c)| assert!(c.is_ascii()))
        .group_by(move |(_, c)| category(*c))
        .into_iter()
        .filter(|(category, _)| *category != 0)
        .map(|(_, mut group)| -> &str {
            match (group.next(), group.last()) {
                (Some((i, _)), Some((j, _))) => &line[i..j+1],
                (Some((i, _)), None) => &line[i..i+1],
                _ => panic!("Empty group"),
            }
        })
        .take_while(|s| !s.contains("//"))
        .collect::<Vec<_>>()
        .into_iter()
}

impl<I, S> Iterator for Tokenizer<I>
where I: Iterator<Item = S>, S: AsRef<str>
{
    type Item = Token;

    fn next(&mut self) -> Option<Self::Item> {
        if self.error.is_some() { return None; }
        else if let Some(val) = self.tokens.pop() { 
            self.location.column += 1;
            return Some(val); 
        } else {
            let line = self.lines.next()?;
            self.location.line += 1;
            let snippets = slice_into_snippets(line.as_ref());

            self.tokens = vec![];

            for (i, snippet) in snippets.enumerate() {
                let type_ = Type::from_str(snippet);
                self.location.column = i;
                match type_ {
                    Ok(type_) => self.tokens.push(Token { type_, location: self.location }),
                    Err(error) => {
                        self.error = Some(error.with_location(self.location));
                        break;
                    }
                }
            }
    
            self.tokens.reverse();

            self.location.column = 0;
            self.next()
        }
    }
}


pub fn tokenize<I, S>(lines: I) -> Tokenizer<I>
where I: Iterator<Item = S>, S: AsRef<str>
{
    Tokenizer::new(lines)
}