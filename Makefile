image:
	cargo build --target x86_64-unknown-linux-musl --release
	strip target/x86_64-unknown-linux-musl/release/cookie-clicker-afk
	docker build -t cookie-clicker-afk-worker .
	docker save cookie-clicker-afk-worker -o worker.tar

load:
	docker load -i worker.tar
