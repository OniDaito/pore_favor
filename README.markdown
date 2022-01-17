# pore_favor

A couple of programs for dealing with the nuclear pore data.

## Requirements

* Linux (may work on other systems - so far untested)

## Building

If you have rust installed, enter the swiss_parse directory and type

    cargo build

I think it's a good idea to make sure your rust compilier is up-to-date. I've fallen afoul of this a few times

    rustup upgrade

## Running

Rendering
    cargo run --release -bin render -- <path to csv file> <path to output> <threads> <sigma>

Cutting out the ilastik files
    cargo run --release --bin ilastik -- /media/proto_backup/npore/pores.tif /media/proto_backup/npore/pores_Object\ Identities_.tif 1