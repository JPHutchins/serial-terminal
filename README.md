# serial-terminal

A simple serial CLI for Windows, Linux, and MacOS.

# Usage

```
Press Ctrl-t to bring up the menu and exit.

Usage: serial-terminal [OPTIONS] <PORT>

Arguments:
  <PORT>  Path to the serial port, e.g. 'COM1' or '/dev/ttyUSB0'

Options:
  -b, --baud <BAUD>                  [default: 115200]
  -d, --data-bits <DATA_BITS>        5, 6, 7, or 8 [default: 8]
  -f, --flow-control <FLOW_CONTROL>  none, sw, or hw [default: none]
  -p, --parity <PARITY>              none, odd, or even [default: none]
  -s, --stop-bits <STOP_BITS>        1 or 2 [default: 1]
  -h, --help                         Print help information (use `--help` for more detail)
  -V, --version                      Print version information
  ```

# Development

## Requirements

* Rust and Cargo: https://www.rust-lang.org/tools/install
  * Test the Rust and Cargo install: https://doc.rust-lang.org/book/ch01-00-getting-started.html

## Setup

* Create a fork of this repository: https://github.com/JPHutchins/serial-terminal/fork
* Clone your fork: `git clone git@github.com:<YOUR_USERNAME>/serial-terminal.git`

## Build

* From repository root use `cargo build`
  * For release build use `cargo build --release`
* To build and then run the target use `cargo run`
  * To run with arguments, separate the build arguments from the target arguments with `--`; examples:
    * `cargo run -- COM20 -b 9600`
    * `cargo run --release -- COM8 -b 9600`
