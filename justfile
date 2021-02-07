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
	cargo test -- --test-threads 1 {{args}}

alias t := test

doc *args:
	cargo doc {{args}}

alias d := doc
