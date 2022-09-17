use std::borrow::{Borrow, Cow};
use std::process::Child;
use nix::unistd::{fork, ForkResult, pipe};

pub struct Command<'a> {
    args: Vec<Cow<'a, str>>,
}

impl Command<'_> {
    pub fn new(args: Vec<Cow<str>>) -> Self {
        Self { args }
    }


    pub fn run(&self) -> Result<Child, String> {
        std::process::Command::new(&self.args[0])
            .args(&self.args[1..])
            .spawn()
            .map_err(|e| e.to_string())
    }

}