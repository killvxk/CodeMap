use std::io::{self, Read, Write};
use std::collections::HashMap;
use crate::config::Settings;

pub struct Engine {
    settings: Settings,
    cache: HashMap<String, Vec<u8>>,
}

pub enum Status {
    Running,
    Stopped,
    Error(String),
}

pub trait Processable {
    fn process(&self, input: &[u8]) -> io::Result<Vec<u8>>;
}

impl Engine {
    pub fn new(settings: Settings) -> Self {
        Engine {
            settings,
            cache: HashMap::new(),
        }
    }

    pub fn run(&mut self, data: &[u8]) -> io::Result<Vec<u8>> {
        Ok(data.to_vec())
    }

    fn internal_method(&self) -> bool {
        true
    }
}

impl Processable for Engine {
    fn process(&self, input: &[u8]) -> io::Result<Vec<u8>> {
        self.run(input)
    }
}

fn helper_function() -> bool {
    false
}

pub fn public_function(input: &str) -> String {
    input.to_uppercase()
}
