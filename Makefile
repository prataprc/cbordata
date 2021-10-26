build:
	# ... build ...
	cargo +stable build
	cargo +nightly build
	cd cbordata-derive; cargo +nightly build
	#
	# ... test ...
	cargo +stable test --no-run
	cd cbordata-derive; cargo +nightly test --no-run
	cargo +nightly test --no-run
	cd cbordata-derive; cargo +nightly test --no-run
	#
	# ... bench ...
	cargo +nightly bench --no-run
	cd cbordata-derive; cargo +nightly bench --no-run
	#
	# ... doc ...
	cargo +stable doc
	cd cbordata-derive; cargo +nightly bench --no-run
	cargo +nightly doc
	cd cbordata-derive; cargo +nightly bench --no-run
	#
	# ... meta commands ...
	cargo +nightly clippy --all-targets --all-features
	cd cbordata-derive; cargo +nightly clippy --all-targets --all-features

test:
	# ... test ...
	cargo +stable test --no-run
	cd cbordata-derive; cargo +stable test
	cargo +stable run --example macro
	cargo +stable run --example arc
	cargo +nightly test
	cd cbordata-derive; cargo +nightly test
	cargo +nightly run --example macro
	cargo +nightly run --example arc

bench:
	# ... bench ...
	cargo +stable test --no-run
	cd cbordata-derive; cargo +stable test
	cargo +nightly bench
	cd cbordata-derive; cargo +nightly bench

flamegraph:
	echo "not an executable"

prepare: build test bench
	check.sh check.out
	perf.sh perf.out

clean:
	cargo clean
	rm -f check.out perf.out flamegraph.svg perf.data perf.data.old
