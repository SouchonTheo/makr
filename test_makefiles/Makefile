include config.mk
include colors.mk
-include does_not_exist.mk

CC = gcc
CFLAGS = -Wall $(OPTFLAGS)
NAME = my_app

.PHONY: all clean run

all: $(NAME)

$(NAME): main.o utils.o
	$(CC) $(CFLAGS) -o $(NAME) main.o utils.o

main.o: main.c
	$(CC) $(CFLAGS) -c main.c

utils.o: utils.c
	$(CC) $(CFLAGS) -c utils.c

clean:
	rm -f *.o $(NAME)

run: $(NAME)
	@echo "$(GREEN)Running $(NAME)...$(RESET)"
	./$(NAME)

debug: $(NAME)
	@echo "$(RED)Debug mode (LOG_LEVEL=$(LOG_LEVEL))$(RESET)"
	valgrind ./$(NAME)
