set windows-shell := ["pwsh.exe", "-NoLogo", "-NoProfile", "-Command"]


default:
    @vx just --list

fmt:
    vx cargo fmt --all

fmt-check:
    vx cargo fmt --all -- --check

check:
    vx cargo check --workspace

lint:
    vx cargo clippy --workspace --all-targets -- -D warnings

test:
    vx cargo test --workspace

coverage:
    vx cargo llvm-cov --workspace --lcov --output-path lcov.info

coverage-html:
    vx cargo llvm-cov --workspace --html

ci:
    vx just fmt-check
    vx just check
    vx just lint
    vx just test

build-release *args:
    vx cargo build --release -p noti-cli {{args}}

build-release-target target:
    vx cargo build --release -p noti-cli --target {{target}}

run *args:
    vx cargo run -p noti-cli -- {{args}}

package-skills:
    vx python scripts/package_openclaw_skill.py
