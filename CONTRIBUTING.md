# Contributing to makr

Thanks for your interest in contributing! Here's how you can help.

## Reporting issues

- Search [existing issues](https://github.com/SouchonTheo/makr/issues) before opening a new one.
- Include steps to reproduce the problem, the Makefile that triggered it (if applicable), and your OS / Rust version.

## Submitting changes

`main` is protected: every change goes through a pull request that is squash-merged once CI is green.

1. Create a branch from `main` (`feat/…`, `fix/…`, `docs/…`).
2. Make your changes and run the checks:

   ```sh
   make check   # cargo fmt --check, clippy --all-targets -D warnings, cargo test
   ```

3. Open a pull request against `main`. **Its title becomes the commit message on `main`**
   and feeds the release notes, so use a [conventional commit](https://www.conventionalcommits.org/)
   title: `feat: …`, `fix: …`, `docs: …`, `ci: …`.

## Code style

- Run `cargo fmt` before committing.
- Keep warnings clean (`cargo clippy --all-targets -- -D warnings`).
- Add tests for new parser features or fuzzy matching logic.

## Releasing (maintainers)

```sh
make release VERSION=0.2.0   # bump on branch release/v0.2.0, run checks, push, print the PR link
# → open the PR "chore(release): v0.2.0", wait for CI, squash-merge it
make tag                     # tag the merged commit on main and push the tag
```

Pushing the tag runs the release workflow: it verifies the tag matches `Cargo.toml`, creates the
GitHub release with notes generated from commit titles, uploads the binaries and publishes to crates.io.

## License

By contributing, you agree that your contributions will be licensed under the MIT License.
