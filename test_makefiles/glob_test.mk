include *.mk

# This should include all .mk files in the same directory
# (config.mk, colors.mk, etc.) but not itself (cycle protection)

all:
	@echo "Glob include test"
