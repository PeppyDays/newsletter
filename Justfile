default:
	just --list

test:
	cargo test

build:
	cargo build

run: migrate
	cargo run

container-up:
	docker compose up -d

container-down:
	docker compose down

migrate:
	sqlx database create
	sqlx migrate run
