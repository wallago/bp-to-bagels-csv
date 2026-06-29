# List all commands
default:
    @{{ just_executable() }} --list

# ── Dev ───────────────────────────────────────────────────────

# Run the app. Usage: just run -- statement.csv bagels.db
[group('dev')]
run *ARGS:
    cargo run -- {{ ARGS }}

# Build the optimized release binary
[group('dev')]
build:
    cargo build --release

# Type-check the whole workspace (fast feedback, no binary)
[group('dev')]
check:
    cargo check --all-targets

# Run the test suite. Usage: just test [pattern]
[group('dev')]
test *ARGS:
    cargo nextest run {{ ARGS }}

# Format Rust + Nix sources in place
[group('dev')]
fmt:
    cargo fmt
    nixfmt flake.nix package.nix module.nix

# ── Quality ───────────────────────────────────────────────────

# Lint with clippy, treating warnings as errors
[group('quality')]
lint:
    cargo clippy --all-targets -- -D warnings

# Spell-check the codebase
[group('quality')]
typos:
    typos

# Report unused dependencies
[group('quality')]
udeps:
    cargo machete

# Lint Nix (statix) and report dead Nix code (deadnix)
[group('quality')]
nix-lint:
    statix check .
    deadnix --fail .

# ── Security ──────────────────────────────────────────────────

# Audit dependencies against the RustSec advisory DB
[group('security')]
audit:
    cargo audit

# Check licenses, banned crates, sources and advisories
[group('security')]
deny:
    cargo deny check

# ── Nix ───────────────────────────────────────────────────────

# Build & check every flake output on all systems
[group('nix')]
flake-check:
    nix flake check --print-build-logs --all-systems

# Update all flake inputs to their latest revisions
[group('nix')]
update:
    nix flake update

# ── Release ───────────────────────────────────────────────────

# Verify commit messages follow Conventional Commits
[group('release')]
commits:
    committed -vv HEAD

# Regenerate CHANGELOG.md from git history
[group('release')]
changelog:
    git-cliff -o CHANGELOG.md

# Cut & publish a release: bump version, changelog, commit, tag, push. Usage: just publish 0.2.0
[confirm("This will tag and push a release to origin. Continue?")]
[group('release')]
publish VERSION:
    @test -z "$(jj diff --name-only)" || (echo "✗ working copy has uncommitted changes — commit or abandon them first" && exit 1)
    @echo "▶ bump version → {{ VERSION }}"
    cargo set-version {{ VERSION }}
    @echo "▶ regenerate changelog"
    git-cliff --tag v{{ VERSION }} -o CHANGELOG.md
    @echo "▶ record release commit"
    jj commit -m "chore(release): v{{ VERSION }}"
    @echo "▶ advance main bookmark"
    jj bookmark set main -r @-
    @echo "▶ tag release commit"
    git tag -a v{{ VERSION }} -m "v{{ VERSION }}" "$(jj log -r @- --no-graph -T commit_id)"
    @echo "▶ push bookmark + tag"
    jj git push --bookmark main
    git push origin v{{ VERSION }}
    @echo "✅ published v{{ VERSION }}"

# ── CI ────────────────────────────────────────────────────────

# Mirror the full CI pipeline locally. Run before pushing.
[group('ci')]
ci:
    @echo "▶ format"
    cargo fmt --check
    @echo "▶ clippy"
    cargo clippy --all-targets -- -D warnings
    # @echo "▶ test"
    # cargo nextest run
    @echo "▶ typos"
    typos
    @echo "▶ unused deps"
    cargo machete
    @echo "▶ deny"
    cargo deny check
    @echo "▶ audit"
    cargo audit
    @echo "▶ commits"
    committed -vv HEAD
    @echo "▶ nix lint"
    statix check .
    deadnix --fail .
    @echo "▶ flake check"
    nix flake check --print-build-logs --all-systems
    @echo "✅ CI mirror passed"
