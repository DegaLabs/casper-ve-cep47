prepare:
	rustup target add wasm32-unknown-unknown

build-contract:
	cd ve && cargo build --release --target wasm32-unknown-unknown
	wasm-strip target/wasm32-unknown-unknown/release/ve.wasm 2>/dev/null | true

test-only:
	cargo test -p cep47-tests

copy-wasm-file-to-test:
	cp ve/target/wasm32-unknown-unknown/release/*.wasm tests/wasm

test: build-contract copy-wasm-file-to-test
	mkdir -p tests/wasm
	cd tests/test-session && cargo build --release --target wasm32-unknown-unknown
	wasm-strip tests/test-session/target/wasm32-unknown-unknown/release/test-session.wasm 2>/dev/null | true
	cp tests/test-session/target/wasm32-unknown-unknown/release/test-session.wasm tests/wasm
	cd tests && cargo test -- --nocapture

clippy:
	cargo clippy --all-targets --all -- -D warnings

check-lint: clippy
	cargo fmt --all -- --check

lint: clippy
	cargo fmt --all

clean:
	cargo clean
	rm -rf cep47-tests/wasm/*.wasm
