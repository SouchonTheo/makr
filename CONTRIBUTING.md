# Contributing to makr

Thanks for your interest in contributing! Here's how you can help.

## Reporting issues

- Search [existing issues](https://github.com/SouchonTheo/makr/issues) before opening a new one.
- Include steps to reproduce the problem, the Makefile that triggered it (if applicable), and your OS / Rust version.

## Submitting changes

1. Fork the repo and create a branch from `main`.
2. Make your changes.
3. Make sure the checks pass:

   ```sh
   cargo fmt --all -- --check
   cargo clippy -- -D warnings
   cargo test
   ```

4. Open a pull request against `main`.

## Code style

- Run `cargo fmt` before committing.
- Keep warnings clean (`cargo clippy -- -D warnings`).
- Add tests for new parser features or fuzzy matching logic.

## License

By contributing, you agree that your contributions will be licensed under the MIT License.
