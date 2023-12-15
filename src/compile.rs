use std::result::Result;
use anstream::println;


use crate::errors::LocalizedError;
use crate::frontend::tokenizer::tokenize;
use crate::frontend::ast;

pub fn compile_lines<I, S>(lines: I) -> Result<(), LocalizedError> 
where I: Iterator<Item = S>, S: AsRef<str>
{
    let mut tokenizer = tokenize(lines);

    let ast = ast::parse(&mut tokenizer);

    if let Some(error) = tokenizer.error() {
        return Err(error);
    } else if let Err(error) = ast {
        return Err(error);
    } 

    println!("{:#?}", ast.unwrap());

    Ok(())
}
