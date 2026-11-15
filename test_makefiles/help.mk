CC = gcc # the compiler
HASH = a\#b
DATE = $(shell date +%s#)

### Build ###################################

build: main.o ## Compile the project
	$(CC) -o app main.o

## Run the test suite
## (slow)
test: build
	./run-tests

# Plain comment, not a description
clean:
	rm -f app

## Orphaned: a blank line separates it from the rule

lint: ## Inline wins
lint: build ## Ignored, first description is kept
