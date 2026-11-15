.PHONY: build build-release release tag fmt fmt-check clippy lint test clean check

build:
	cargo build

build-release:
	cargo build --release

fmt:
	cargo fmt --all

clippy:
	cargo clippy --all-targets -- -D warnings

fmt-check:
	cargo fmt --all -- --check

lint: fmt clippy

check: fmt-check clippy test

test:
	cargo test

clean:
	cargo clean

# ── Releasing ────────────────────────────────────────────────────────────
# `main` only accepts squash-merged PRs, so a release takes two steps:
#
#   1. make release VERSION=0.2.0   bump on a release/v0.2.0 branch, check, push, open the PR
#   2. (merge the PR on GitHub)
#   3. make tag                     tag the merged commit on main and push the tag
#
# Pushing the tag triggers .github/workflows/release.yml.

REPO_URL := https://github.com/SouchonTheo/makr
CARGO_VERSION = $$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -n1)

release:
	@echo "$(VERSION)" | grep -Eq '^[0-9]+\.[0-9]+\.[0-9]+(-[0-9A-Za-z.]+)?$$' \
		|| { echo "usage: make release VERSION=x.y.z"; exit 1; }
	@git diff --quiet HEAD || { echo "error: commit or stash your changes first"; exit 1; }
	git fetch --quiet --tags origin
	@! git rev-parse -q --verify "refs/tags/v$(VERSION)" >/dev/null \
		|| { echo "error: tag v$(VERSION) already exists"; exit 1; }
	git switch --quiet -c "release/v$(VERSION)" origin/main
	sed -i.bak 's/^version = ".*"/version = "$(VERSION)"/' Cargo.toml && rm Cargo.toml.bak
	cargo update --workspace --offline
	$(MAKE) check
	git commit --quiet -am "chore(release): v$(VERSION)"
	git push --quiet -u origin "release/v$(VERSION)"
	@echo
	@echo "Branch release/v$(VERSION) pushed. Open the PR (title: chore(release): v$(VERSION)):"
	@echo "  $(REPO_URL)/compare/main...release/v$(VERSION)?expand=1"
	@echo "Once it is squash-merged, run: make tag"

tag:
	@git diff --quiet HEAD || { echo "error: commit or stash your changes first"; exit 1; }
	git switch --quiet main
	git pull --quiet --ff-only origin main
	git fetch --quiet --tags origin
	@v=$(CARGO_VERSION); \
	if git rev-parse -q --verify "refs/tags/v$$v" >/dev/null; then \
		echo "error: v$$v is already tagged. Did the release PR get merged?"; exit 1; \
	fi; \
	git tag -a "v$$v" -m "v$$v" && \
	git push --quiet origin "v$$v" && \
	git branch -D "release/v$$v" >/dev/null 2>&1; \
	echo; echo "Tag v$$v pushed: the release workflow is running."; \
	echo "  $(REPO_URL)/actions/workflows/release.yml"
