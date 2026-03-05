.PHONY: all
all:
	@cargo build
	@cargo build --release

.PHONY: test
test:
	cargo nextest run --release
	cargo nextest run
# cargo test --release
# cargo test

.PHONY: clean
clean:
	@rm -rf release

.PHONY: rsb_clean_build
rsb_clean_build:
	@target/release/rsb clean -v
	@target/release/rsb build -j 4 -v

.PHONY: rsb_clean_build_hard
rsb_clean_build_hard:
	@target/release/rsb clean -v
	@target/release/rsb cache clear -v
	@time target/release/rsb build -v

.PHONY: rsb_graph
rsb_graph:
	@target/release/rsb graph --view mermaid

.PHONY: rsb_build
rsb_build:
	@target/release/rsb build -v

.PHONY: rsb_clean
rsb_clean:
	@target/release/rsb clean -v
