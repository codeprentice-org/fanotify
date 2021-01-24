set shell := ["bash", "-c"]

_default:
   just --list

build *args:
	cargo build {{args}}

alias b := build

check *args:
	@# actually run clippy for more warnings
	@# clippy also runs check
	cargo clippy {{args}}

alias c := check

test *args:
	cargo test {{args}}

alias t := test

sudo-test *args:
	sudo -E env "PATH=${PATH}" cargo test {{args}}

alias st := sudo-test

doc *args:
	cargo doc --document-private-items {{args}}

alias d := doc
