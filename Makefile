run:
	RUST_LOG=debug cargo run --bin qust

run-release:
	cargo run --release --bin qust

benchmark:
	cargo bench --bench server

test:
	cargo test --test server
