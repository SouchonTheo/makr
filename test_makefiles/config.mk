DEBUG = 1
OPTFLAGS = -O0 -g
LOG_LEVEL = verbose

check-config:
	@echo "DEBUG=$(DEBUG)"
	@echo "OPTFLAGS=$(OPTFLAGS)"
	@echo "LOG_LEVEL=$(LOG_LEVEL)"

set-release:
	$(eval DEBUG = 0)
	$(eval OPTFLAGS = -O2)
	@echo "Switched to release config"
