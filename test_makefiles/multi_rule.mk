# Regression fixture for issue #1: several rules for the same target.
CHROME_VERSION = 110
FLAGS = -q

.PHONY: install_chrome_browser
install_chrome_browser: deps_a
install_chrome_browser: deps_b deps_a
install_chrome_browser: CHROME_VERSION = 120
install_chrome_browser: FLAGS += -v
install_chrome_browser:
	@echo installing chrome $(CHROME_VERSION) $(FLAGS)

a b c: shared
	@echo $@

inline: dep ; @echo inline

log:: ; @echo one
log::
	@echo two

redo:
	@echo old
redo:
	@echo new
