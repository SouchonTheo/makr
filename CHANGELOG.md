# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0] - 2026-04-19

### Added

- Interactive TUI for browsing Makefile targets with `.PHONY` indicators
- Target detail panel showing dependencies, commands, and used variables (scrollable)
- Fuzzy search (`/`) with ranked results and match highlighting
- Variable editor popup with live command preview before running a target
- Dry-run mode (`--dry-run` flag or `Ctrl+n` toggle in the run popup)
- Support for `include`, `-include`, `sinclude` directives with glob expansion
- Robust parser: continuation lines, `export`/`override` prefixes, `define` blocks, all assignment types (`=`, `:=`, `?=`, `+=`)

[Unreleased]: https://github.com/SouchonTheo/makr/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/SouchonTheo/makr/releases/tag/v0.1.0
