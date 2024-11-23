default: build

build:
	cargo test --color always 2>&1 | less -R

install:
	brew install poppler pdftk-java
