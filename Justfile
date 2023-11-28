default:
	just --list

test: migrate
	cargo test

test_debug: migrate
	TEST_LOG=true cargo test | bunyan

build:
	cargo build

run: migrate
	cargo run | bunyan

sqlx-prepare:
	cargo sqlx prepare --workspace

watch-run: migrate
	cargo watch -x run | bunyan

container-up:
	docker compose up -d

container-down:
	docker compose down

migrate:
	sqlx database create
	sqlx migrate run
