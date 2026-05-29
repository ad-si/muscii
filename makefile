.PHONY: help
help: makefile
	@tail -n +4 makefile | grep ".PHONY"


.PHONY: format
format:
	cargo clippy --fix --allow-dirty
	cargo fmt
	# nix fmt  # TODO: Reactivate when it's faster


.PHONY: build
build:
	cargo build


.PHONY: test
test:
	cargo test


.PHONY: install
install:
	cargo install --path .
