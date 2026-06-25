#[cfg(not(coverage))]
use seatd::cli::{RealCommands, dispatch, print_outcome};
#[cfg(not(coverage))]
use std::env;

#[cfg(not(coverage))]
fn main() {
    let args: Vec<String> = env::args().collect();
    print_outcome(dispatch(&args, &RealCommands));
}

#[cfg(coverage)]
fn main() {}
