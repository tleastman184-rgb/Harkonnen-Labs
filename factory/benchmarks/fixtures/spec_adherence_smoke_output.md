# Spec Adherence Benchmark README

This smoke fixture demonstrates the kind of review-oriented artifact the spec adherence benchmark judges.

## Run

Execute the adapter locally with:

`cargo run -- benchmark run --suite harkonnen_spec_adherence`

The adapter can evaluate an explicit JSONL dataset or fall back to recent completed runs when run data is available.

## Validate

Use `cargo check` to confirm the benchmark wiring compiles, then run `cargo test -q` to verify the adapter and its fixtures remain healthy.

## Notes

The smoke fixture for this benchmark lives under `factory/benchmarks/fixtures/`, which keeps the baseline reproducible on machines without a populated run corpus.
