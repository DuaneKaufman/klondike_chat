20251214 - Duane Kaufman

Rust-based klondike Draw-3 card player to investigate fraction of winnable hands

Invocation:

cargo run --release -- \
  --pysol-seed-range=1:1000000 \
  --out-csv=results.csv \
  --max-nodes=5000000 \
  --max-depth=700

or to resume:

cargo run --release -- \
  --pysol-seed-range=1:1000000 \
  --out-csv=results.csv \
  --resume \
  --max-nodes=5000000 \
  --max-depth=700
