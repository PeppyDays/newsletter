container-up:
	docker compose up -d

container-down:
	docker compose down

db-migrate:
	sqlx migrate run --source ./resources/db/migrations
