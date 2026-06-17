.PHONY: help dev test coverage lint check infra-up infra-stop infra-down infra-clean build run sonar clean

help:
	@echo "Job Service Rust — available targets:"
	@echo "  make dev             Run in dev mode (cargo run)"
	@echo "  make test            Run unit tests"
	@echo "  make coverage        Run tests with coverage (cargo tarpaulin)"
	@echo "  make lint            Run clippy"
	@echo "  make check           Run lint + test"
	@echo "  make infra-up        Start PG + Redis + Rabbit via Docker Compose"
	@echo "  make infra-stop      Stop the dev infrastructure"
	@echo "  make infra-down      Stop and remove infra containers"
	@echo "  make infra-clean     Stop infra and remove volumes"
	@echo "  make build           Build Docker image"
	@echo "  make run             Run the application (Docker)"
	@echo "  make sonar           Run SonarQube scan"
	@echo "  make clean           Remove target/"

dev:
	@echo "Running job runner..."
	cargo run

test:
	@echo "Running tests..."
	cargo test

coverage:
	@echo "Running tests with coverage..."
	cargo tarpaulin --workspace --timeout 300 --out Html --out Xml --output-dir coverage

lint:
	@echo "Running clippy..."
	cargo clippy --all-targets --all-features -- -D warnings

check: lint test
	@echo "All checks passed"

infra-up:
	@echo "Starting infrastructure (PostgreSQL + Redis + RabbitMQ)..."
	docker compose -f docker-compose.infra.yml up -d

infra-stop:
	@echo "Stopping infrastructure..."
	docker compose -f docker-compose.infra.yml stop

infra-down:
	@echo "Removing infrastructure containers..."
	docker compose -f docker-compose.infra.yml down

infra-clean:
	@echo "Cleaning infrastructure..."
	docker compose -f docker-compose.infra.yml down -v --rmi all

build:
	@echo "Building Docker image..."
	docker build -t job-service-rust .

run:
	@echo "Running application container..."
	docker run --rm --network host job-service-rust

sonar:
	@echo "Running SonarQube scan..."
	./scripts/sonar-scan.sh "job-service-rust" "Job Service Rust"

clean:
	@echo "Cleaning artifacts..."
	cargo clean
	rm -rf coverage/
