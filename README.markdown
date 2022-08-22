# pore_favor

A couple of programs for dealing with the nuclear pore data available at [https://www.ebi.ac.uk/biostudies/BioImages/studies/S-BIAD8](https://www.ebi.ac.uk/biostudies/BioImages/studies/S-BIAD8). The related paper can be found at [https://www.nature.com/articles/s41592-019-0574-9](https://www.nature.com/articles/s41592-019-0574-9).

## Requirements

* Linux (may work on other systems - so far untested)

## Building

If you have rust installed, enter the swiss_parse directory and type

    cargo build

I think it's a good idea to make sure your rust compilier is up-to-date. I've fallen afoul of this a few times

    rustup upgrade

## Running

Rendering the initial fits file from the csv pore data.

    cargo run --release --bin render -- /phd/npore/GFP_AB-AF647_190517_2_sml.csv /phd/npore 10 1.8

Once an image has been created, use [Ilastik](https://www.ilastik.org/) to segment the images. The following program will then cut-out the individual images.

    cargo run --release --bin ilastik -- /media/proto_backup/npore/pores.tiff /media/proto_backup/npore/pores_Object\ Identities.tiff 1 <optional sigma>