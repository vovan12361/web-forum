run:
	go run cmd/main.go

up:
	docker compose --env-file .env up -d

build:
	docker compose build

up_build:
	docker compose --env-file .env up -d --build

stop:
	docker compose stop

down:
	docker compose down

clean:
	docker volume rm posts_postgres_data