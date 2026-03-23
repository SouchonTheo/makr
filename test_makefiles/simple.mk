# Simple Makefile — no variables, no includes
all: hello goodbye

hello:
	echo "Hello, world!"

goodbye:
	echo "Goodbye!"

clean:
	rm -f *.o
