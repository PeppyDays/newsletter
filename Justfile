default:
	just --list

test:
	cargo test

build:
	cargo build

run: migrate
	cargo run | bunyan

watch-run: migrate
	cargo watch -x run | bunyan

container-up:
	docker compose up -d

container-down:
	docker compose down

migrate:
	sqlx database create
	sqlx migrate run
