.PHONY: help \
	upgrade \
	lint \
	lint-audit \
	audit-fix \
	test \
	coverage \
	check \
	find-msrv \
	verify-msrv \
	clean \
	prepare \
	build \
	build-no-audit \
	doc \
	doc-public \
	watch-doc \
	doc-deps

.DEFAULT_GOAL=help

# Parameters
APP_NAME="API tools"
CARGO=cargo

help: Makefile
	@echo
	@echo "Choose a command run in "$(APP_NAME)":"
	@echo
	@sed -n 's/^##//p' $< | column -t -s ':' | sed -e 's/^/ /'
	@echo

## upgrade: Upgrade crates
upgrade:
	$(CARGO) upgrade
	$(CARGO) update

## upgrade-force: Upgrade crates with incompatible changes
upgrade-force:
	$(CARGO) upgrade --incompatible
	$(CARGO) update

## lint: Run clippy and rustfmt
lint:
	$(CARGO) fmt
	$(CARGO) clippy --all-features -- -D warnings

## lint-audit: Run clippy, rustfmt and audit
lint-audit: lint
	$(CARGO) audit

## audit-fix: Fix audit
audit-fix:
	$(CARGO) audit fix

## test: Launch unit tests in a single thread
test:
	$(CARGO) test --all-features -- --nocapture

## coverage: Launch coverage tests
coverage:
	$(CARGO) tarpaulin --all-features

## check: Clippy, audit and test
check: lint-audit test

## find-msrv: Find minimum supported Rust version
find-msrv:
	$(CARGO) msrv find

## verify-msrv: Verify minimum supported Rust version
verify-msrv:
	$(CARGO) msrv verify

## clean: Remove target directory
clean:
	$(CARGO) clean

## prepare: Run lint, test and verify-msrv
prepare: lint test verify-msrv

## build: Build application in release mode
build: lint-audit test
	$(CARGO) build --release

## build-no-audit: Build application in release mode
build-no-audit: lint test
	$(CARGO) build --release

## doc: Open Rust documentation without dependencies
doc:
	$(CARGO) doc --open --no-deps --all-features

## doc-public: Open Rust documentation without dependencies
doc-public:
	$(CARGO) doc --open --document-private-items --no-deps --all-features

## watch-doc: Watch Rust documentation without dependencies
watch-doc: doc
	$(CARGO) watch -x 'doc --no-deps --all-features'

## doc: Open Rust documentation with dependencies
doc-deps:
	$(CARGO) doc --open --document-private-items --all-features
