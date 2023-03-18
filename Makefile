image:
	cargo build --release
	strip target/release/cookie-clicker-afk
	docker build -t cookie-clicker-afk-worker .
	docker save cookie-clicker-afk-worker -o worker.tar

load:
	docker load -i worker.tar
