use std::borrow::{Borrow, Cow};
use std::ops::Deref;
use std::process::Child;
use nix::unistd::{fork, ForkResult, pipe};

pub struct Command<'a> {
    args: Vec<Cow<'a, str>>,
}

impl <'a>Command<'a> {
    pub fn new(args: Vec<Cow<'a, str>>) -> Self {
        Self { args }
    }


    pub fn run(&self) -> Result<Child, String> {
        let mut cmd = std::process::Command::new(self.args[0].deref());
        for arg in self.args.iter().skip(1) {
            cmd.arg(arg.deref());
        }

        cmd.spawn().map_err(|e| e.to_string())
    }

}