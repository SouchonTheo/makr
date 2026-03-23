RED = \033[0;31m
GREEN = \033[0;32m
YELLOW = \033[0;33m
BLUE = \033[0;34m
RESET = \033[0m

print-ok:
	@echo "$(GREEN)OK$(RESET)"

print-error:
	@echo "$(RED)ERROR$(RESET)"

print-warning:
	@echo "$(YELLOW)WARNING$(RESET)"

color-test:
	@echo "$(RED)Red$(RESET) $(GREEN)Green$(RESET) $(YELLOW)Yellow$(RESET) $(BLUE)Blue$(RESET)"
