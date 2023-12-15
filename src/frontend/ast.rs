use core::fmt;
use std::{error::Error, iter::Peekable, fmt::Debug};

use owo_colors::OwoColorize;

use crate::frontend::tokenizer::{Operator, Token, Location, Type as TokenT, Tokenizer};
use crate::errors::{LocalizableError, LocalizedError};

pub struct  AST {
    type_: Type,
    location: Location,
}

impl Debug for AST {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{}:{}]", self.location.line.blue(), self.location.column.blue())?;
        write!(f, "{:#?}", self.type_)
    }
}

#[derive(Debug)]
pub enum Type {
    Literal(String),
    // name, type
    TypedLiteral(String, String),
    // operator, lhs, rhs - arithmetic expression
    Expression(Operator, Box<AST>, Box<AST>),
    // return type, arguments, body
    Lambda(String, Vec<AST>, Box<AST>),
    Block(Vec<AST>),
    Module(Vec<AST>),
}

impl Type {
    pub fn wrap(self, location: Location) -> AST {
        AST {
            type_: self,
            location,
        }
    }
}

#[derive(Debug)]
pub struct ParseError {
    message: String,
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ParseError: {}", self.message)
    }
}

impl Error for ParseError {}


pub fn parse<I, S>(tokenizer: &mut Tokenizer<I>) -> Result<AST, LocalizedError>
where I: Iterator<Item = S>, S: AsRef<str>
{
    let mut tokens = tokenizer.peekable();
    parse_module(&mut tokens)
        .map_err(|err| err.with_location(Location::default())) // FIXME: this is a hack
        // .map_err(|err| err.with_location(Location { 
        //     line: tokenizer.location().line, 
        //     column: tokenizer.location().column-1 
        // }))
}

/// Parses a module
/// * `tokens` - the tokens to parse
pub fn parse_module(tokens: &mut Peekable<impl Iterator<Item = Token>>) -> Result<AST, ParseError> {
    let location = locate(tokens);
    let mut asts = Vec::new();
    while let Some(token) = tokens.peek() {
        match token.type_ {
            _ => asts.push(parse_statement(tokens)?),
        }
    }
    Ok(Type::Module(asts).wrap(location))
}

/// parse an _arithmetic_ expression, e.g. `1 + 2 * 3`
/// * `tokens` - the tokens to parse
pub fn parse_expression(tokens: &mut Peekable<impl Iterator<Item = Token>>) -> Result<AST, ParseError> {
    match tokens.peek().map(|x| x.type_.clone()) {
        Some(TokenT::Operator(Operator::LCurl)) => parse_block(tokens),
        Some(TokenT::Operator(Operator::Fn)) => parse_function(tokens),
        Some(_) => parse_airthmetic_expression(tokens),
        None => Err(ParseError {
            message: format!("Expected expression, found end of input"),
        }),
    }
}


/// Parses an arithmetic expression, e.g. `1 + 2 * 3`

pub fn parse_airthmetic_expression(tokens: &mut Peekable<impl Iterator<Item = Token>>) -> Result<AST, ParseError> {
    let location = locate(tokens);
    let mut ast = parse_term(tokens)?;
    while let Some(token) = tokens.peek() {
        match token.type_ {
            TokenT::Operator(Operator::Add) => {
                tokens.next();
                ast = Type::Expression(Operator::Add, Box::new(ast), Box::new(parse_term(tokens)?)).wrap(location);
            }
            TokenT::Operator(Operator::Sub) => {
                tokens.next();
                ast = Type::Expression(Operator::Sub, Box::new(ast), Box::new(parse_term(tokens)?)).wrap(location);
            }
            _ => break,
        }
    }
    Ok(ast)
}

pub fn parse_term(tokens: &mut Peekable<impl Iterator<Item = Token>>) -> Result<AST, ParseError> {
    let location = locate(tokens);
    let mut ast = parse_factor(tokens)?;
    while let Some(token) = tokens.peek() {
        match token.type_ {
            TokenT::Operator(Operator::Mul) => {
                tokens.next();
                ast = Type::Expression(Operator::Mul, Box::new(ast), Box::new(parse_factor(tokens)?)).wrap(location);
            }
            TokenT::Operator(Operator::Div) => {
                tokens.next();
                ast = Type::Expression(Operator::Div, Box::new(ast), Box::new(parse_factor(tokens)?)).wrap(location);
            }
            TokenT::Operator(Operator::Mod) => {
                tokens.next();
                ast = Type::Expression(Operator::Mod, Box::new(ast), Box::new(parse_factor(tokens)?)).wrap(location);
            }
            _ => break,
        }
    }
    Ok(ast)
}

pub fn parse_factor(tokens: &mut Peekable<impl Iterator<Item = Token>>) -> Result<AST, ParseError> {
    let location = locate(tokens);
    let mut ast = parse_atom(tokens)?;
    while let Some(token) = tokens.peek() {
        match token.type_ {
            TokenT::Operator(Operator::Pow) => {
                tokens.next();
                ast = Type::Expression(Operator::Pow, Box::new(ast), Box::new(parse_atom(tokens)?)).wrap(location);
            }
            _ => break,
        }
    }
    Ok(ast)
}

/// Parses an atom of an arithmetic expression, e.g. `1`, `2`, `3`, `1 + 2`, `(1 + 2) * 3`, etc.
/// * `tokens` - the tokens to parse
pub fn parse_atom(tokens: &mut Peekable<impl Iterator<Item = Token>>) -> Result<AST, ParseError> {
    let location = locate(tokens);
    match tokens.next().map(|x| x.type_) {
        Some(TokenT::Literal(s)) => Ok(Type::Literal(s).wrap(location)),
        Some(TokenT::Operator(Operator::Sub)) => Ok(Type::Expression(Operator::Sub, 
            Box::new(Type::Literal("0".to_owned()).wrap(location)), 
            Box::new(parse_atom(tokens)?)).wrap(location)),
        Some(TokenT::Operator(Operator::Add)) => Ok(parse_atom(tokens)?),
        Some(TokenT::Operator(Operator::LParen)) => {
            let ast = parse_airthmetic_expression(tokens)?;
            match tokens.next().map(|x| x.type_) {
                Some(TokenT::Operator(Operator::RParen)) => Ok(ast),
                x => Err(expected_found("closing parenthesis", x)),
            }
        }
        x => Err(expected_found("literal, unary operator or opening parenthesis", x)),
    }
}

///////////////////////////////

/// Parses a typed literal, e.g. `1: int`
/// * `tokens` - the tokens to parse
/// * `strict` - whether to require a type annotation (type information can still be provided by the user)
pub fn parse_typed_literal(tokens: &mut Peekable<impl Iterator<Item = Token>>, strict: bool) -> Result<AST, ParseError> {
    let location = locate(tokens);
    match tokens.next().map(|x| x.type_) {
        Some(TokenT::Literal(s)) => {
            match tokens.peek().map(|x| x.type_.clone()) {
                Some(TokenT::Operator(Operator::Colon)) => {
                    tokens.next();
                    match tokens.next().map(|x| x.type_) {
                        Some(TokenT::Literal(t)) => Ok(Type::TypedLiteral(s, t).wrap(location)),
                        x => Err(expected_found("literal [type information]", x)),
                    }
                }
                x => if strict { 
                    Err(expected_found("colon [type information]", x))
                } else {
                    Ok(Type::Literal(s).wrap(location))
                },
            }
        }
        x => Err(expected_found("literal [name information]", x)),
    }
}



/// parse an assignment expression, e.g. `let x = 1`
/// * `tokens` - the tokens to parse
pub fn parse_assignment(tokens: &mut Peekable<impl Iterator<Item = Token>>) -> Result<AST, ParseError> {
    let location = locate(tokens);
    match tokens.next().map(|x| x.type_) {
        Some(TokenT::Operator(Operator::Let)) => (),
        x => return Err(expected_found("let keyword", x)),
    }
    let name = parse_typed_literal(tokens, false)?;
    match tokens.next().map(|x| x.type_) {
        Some(TokenT::Operator(Operator::Assign)) => {
            let ast = parse_expression(tokens)?;
            Ok(Type::Expression(Operator::Let, Box::new(name), Box::new(ast)).wrap(location))
        }
        x => Err(expected_found("assignment operator", x)),
    }
}

/// parse a top level module statement
/// * `tokens` - the tokens to parse
pub fn parse_statement(tokens: &mut Peekable<impl Iterator<Item = Token>>) -> Result<AST, ParseError> {
    let ast = match tokens.peek().map(|x| x.type_.clone()) {
        Some(TokenT::Operator(Operator::Let)) => parse_assignment(tokens)?,
        _ => parse_expression(tokens)?,
    };
    match tokens.next().map(|x| x.type_) {
        Some(TokenT::Operator(Operator::Semicolon)) => Ok(ast),
        x => Err(expected_found("semicolon", x)),
    }
}

/// parse a curly brace delimited block
/// * `tokens` - the tokens to parse
pub fn parse_block(tokens: &mut Peekable<impl Iterator<Item = Token>>) -> Result<AST, ParseError> {
    let location = locate(tokens);
    match tokens.next().map(|x| x.type_) {
        Some(TokenT::Operator(Operator::LCurl)) => (),
        x => return Err(expected_found("opening curly brace", x)),
    }
    let mut asts = Vec::new();
    while let Some(token) = tokens.peek() {
        match token.type_ {
            TokenT::Operator(Operator::RCurl) => {
                tokens.next();
                break;
            }
            _ => asts.push(parse_statement(tokens)?),
        }
    }
    Ok(Type::Block(asts).wrap(location))
}


/////////////////////////////

/// parse a lambda expression
pub fn parse_function(tokens: &mut Peekable<impl Iterator<Item = Token>>) -> Result<AST, ParseError> {
    let location = locate(tokens);
    match tokens.next().map(|x| x.type_) {
        Some(TokenT::Operator(Operator::Fn)) => (),
        x => return Err(expected_found("fn keyword", x)),
    }
    match tokens.next().map(|x| x.type_) {
        Some(TokenT::Operator(Operator::LParen)) => (),
        x => return Err(expected_found("opening parenthesis", x)),
    }
    let mut args = Vec::new();
    while let Some(token) = tokens.peek() {
        match token.type_.clone() {
            TokenT::Operator(Operator::RParen) => {
                tokens.next();
                break;
            }
            TokenT::Literal(_) => {
                args.push(parse_typed_literal(tokens, true)?);
                match tokens.peek().map(|x| x.type_.clone()) {
                    Some(TokenT::Operator(Operator::Comma)) => {
                        tokens.next();
                    }
                    Some(TokenT::Operator(Operator::RParen)) => (),
                    x => return Err(expected_found("comma or closing parenthesis", x)),
                }
            }
            x => return Err(expected_found("literal or closing parenthesis", Some(x))),
        }
    }
    match tokens.next().map(|x| x.type_) {
        Some(TokenT::Operator(Operator::Colon)) => (),
        x => return Err(expected_found("colon [type information] ", x)),
    }
    let typ = match tokens.next().map(|x| x.type_) {
        Some(TokenT::Literal(t)) => Ok(t),
        x => return Err(expected_found("literal [type information]", x)),
    }?;
    let block = parse_block(tokens)?;
    Ok(Type::Lambda(typ, args, Box::new(block)).wrap(location))
}



// PRIVATE HELPER FUNCTIONS

fn expected_found<T>(expected: &str, found: Option<T>) -> ParseError
where T: fmt::Debug,
{
    match found {
        Some(found) => ParseError {
            message: format!("Expected {}, found {:?}", expected, found),
        },
        None => ParseError {
            message: format!("Expected {}, found end of input", expected),
        },
    }
}

fn locate(tokens: &mut Peekable<impl Iterator<Item = Token>>) -> Location {
    tokens.peek().map(|x| x.location).unwrap_or_default()
}