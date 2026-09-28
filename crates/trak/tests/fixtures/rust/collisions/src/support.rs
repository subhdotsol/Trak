use std::io::{IsTerminal, Write};

pub fn read_input() -> String {
    let mut input = String::new();
    let _ = std::io::stdin().read_line(&mut input);
    input.trim().to_string()
}

pub fn record(input: &str) {
    println!("{input}");
}

/// A method the caller cannot tell from a std one by name alone.
pub fn yellow(&self) -> String {
    self.0.clone()
}

/// Collides with the std error combinator.
pub fn map_err<T, E>(value: T) -> Result<T, E> {
    Ok(value)
}
