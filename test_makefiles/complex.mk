# Complex Makefile with many variable types and targets

CC := clang
CXX := clang++
CFLAGS ?= -Wall -Wextra -Werror
LDFLAGS += -lm -lpthread

PREFIX = /usr/local
BINDIR = $(PREFIX)/bin
LIBDIR = $(PREFIX)/lib

SRC_DIR = src
BUILD_DIR = build
TEST_DIR = tests

SRCS = $(wildcard $(SRC_DIR)/*.c)
OBJS = $(SRCS:$(SRC_DIR)/%.c=$(BUILD_DIR)/%.o)

NAME = megaproject
VERSION = 1.2.3

.PHONY: all clean test install uninstall dist docker

all: $(NAME)

$(NAME): $(OBJS)
	$(CC) $(CFLAGS) $(LDFLAGS) -o $(NAME) $(OBJS)

$(BUILD_DIR)/%.o: $(SRC_DIR)/%.c
	@mkdir -p $(BUILD_DIR)
	$(CC) $(CFLAGS) -c $< -o $@

clean:
	rm -rf $(BUILD_DIR)
	rm -f $(NAME)

test: $(NAME)
	@echo "Running tests..."
	$(CC) $(CFLAGS) -o test_runner $(TEST_DIR)/*.c
	./test_runner
	@echo "All tests passed!"

install: $(NAME)
	install -d $(BINDIR)
	install -m 755 $(NAME) $(BINDIR)/$(NAME)

uninstall:
	rm -f $(BINDIR)/$(NAME)

dist: clean
	tar czf $(NAME)-$(VERSION).tar.gz --exclude='.git' .

lint:
	cppcheck --enable=all $(SRC_DIR)
	clang-tidy $(SRCS) -- $(CFLAGS)

format:
	clang-format -i $(SRCS)

docker:
	docker build -t $(NAME):$(VERSION) .
	docker tag $(NAME):$(VERSION) $(NAME):latest

docker-push: docker
	docker push $(NAME):$(VERSION)
	docker push $(NAME):latest

benchmark: $(NAME)
	hyperfine './$(NAME) --bench'

docs:
	doxygen Doxyfile
	@echo "Documentation generated"
