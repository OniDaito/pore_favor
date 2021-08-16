# pore_favor

A couple of programs for dealing with the nuclear pore data.

## Requirements

* Linux (may work on other systems - so far untested)

Use git to checkout this project *at the same level* as  this one. E.g

    /home/me/projects/pore_favor

## Building

If you have rust installed, enter the swiss_parse directory and type

    cargo build

Cargo should find the hdf5-rust package and build it, so long as it's at the same level as this project.

I think it's a good idea to make sure your rust compilier is up-to-date. I've fallen afoul of this a few times

    rustup upgrade

## Running

There are two programs: render and chooser. Render creates all the images and chooser lets you browse the images to select these that you want to use.

    cargo run --bin render --release <path to csv file> <path to output> <threads> <sigma> <pertubations> <accepted - OPTIONAL>

Example:

    cargo run --release --bin render /tmp/pore.csv /output/1.8 24 1.8 100 accepted.txt