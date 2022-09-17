use std::{process::Command, ffi::OsStr};
use paris::error;

pub fn spawn_child<I, S>(command: &str, args: I) -> bool
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    match Command::new(command).args(args).spawn() {
        Ok(mut child) => match child.try_wait() {
            Ok(Some(status)) => status.success(),
            Ok(None) => {
                let res = child.wait();

                match res {
                    Ok(status) => status.success(),
                    Err(error) => {
                        error!("Failed to unwrap exit code: {}", error);
                        false
                    }
                }
            }
            Err(error) => {
                error!("Cannot wait for child: {}", error);
                false
            }
        },
        Err(error) => {
            error!("Command \"{:?}\" failed to execute: {}", command, error);
            false
        }
    }
}