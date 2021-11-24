# pore_favor

A couple of programs for dealing with the nuclear pore data.

## Requirements

* Linux (may work on other systems - so far untested)

Use git to checkout this project *at the same level* as  this one. E.g

    /home/me/projects/pore_favor

## Building

If you have rust installed, enter the swiss_parse directory and type

    cargo build

I think it's a good idea to make sure your rust compilier is up-to-date. I've fallen afoul of this a few times

    rustup upgrade

## Running

     cargo run --release --bin ilastik -- /media/proto_backup/npore/pores.tif /media/proto_backup/npore/pores_Object\ Identities_.tif 