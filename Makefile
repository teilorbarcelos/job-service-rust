.PHONY: dev build test coverage check infra-up infra-stop infra-down infra-clean metrics-up metrics-stop metrics-down generate init-hooks

# Variables
ENVIRONMENT ?= development
PORT ?= 8888

# Starts the development server.
# Uses `cargo watch` for hot reloading, falling back to simple `cargo run` if not installed.
dev:
	@echo "🚀 Iniciando servidor de desenvolvimento Rust..."
	@if command -v cargo-watch >/dev/null 2>&1; then \
		cargo watch -c -q -x build -s "./target/debug/backend-rust"; \
	elif [ -f $(HOME)/.cargo/bin/cargo-watch ]; then \
		$(HOME)/.cargo/bin/cargo-watch -c -q -x build -s "./target/debug/backend-rust"; \
	else \
		echo "⚠️  cargo-watch não instalado no PATH. Executando diretamente..."; \
		cargo run --bin backend-rust; \
	fi

# Builds a release-optimized production binary.
build:
	@echo "📦 Compilando binário de produção otimizado..."
	cargo build --release

# Runs native Rust unit/integration tests.
test:
	@echo "🧪 Executando testes unitários..."
	cargo test

coverage:
	@echo "📊 Gerando relatório de cobertura de código..."
	@if [ -f ./bin/cargo-tarpaulin ]; then \
		./bin/cargo-tarpaulin; \
	else \
		cargo tarpaulin; \
	fi
	@echo "\n--- Resumo de Cobertura ---"
	@echo "Verifique os detalhes acima. Se houver linhas não cobertas, elas estarão listadas na tabela."

# Performs a static analysis check on the codebase.
check:
	@echo "🔍 Executando verificação estática do código..."
	cargo check

# Runs the CRUD generator. Example: make generate name=Product
generate:
	@echo "⚙️  Executando gerador de CRUD Rust para $(name)..."
	cargo run --bin generator $(name)

# Runs the Storage provider generator.
generate-storage:
	@echo "⚙️  Executando gerador de provedor de storage Rust..."
	cargo run --bin storage_generator


# Docker Infrastructure Management (Standard Prefix: infra-)
infra-up:
	@echo "🐳 Subindo infraestrutura local Rust (Postgres & Redis)..."
	docker compose -f docker-compose.infra.yml up -d

infra-stop:
	@echo "🛑 Parando serviços da infraestrutura..."
	docker compose -f docker-compose.infra.yml stop

infra-down:
	@echo "🗑️  Removendo containers da infraestrutura..."
	docker compose -f docker-compose.infra.yml down

infra-clean:
	@echo "🧹 Limpeza completa da infraestrutura (Volumes & Imagens)..."
	docker compose -f docker-compose.infra.yml down -v --rmi all

# Métricas (Prometheus & Grafana)
metrics-up:
	@echo "📈 Subindo stack de métricas (Prometheus & Grafana)..."
	docker compose -f docker-compose.metrics.yml up -d

metrics-stop:
	@echo "🛑 Parando stack de métricas..."
	docker compose -f docker-compose.metrics.yml stop

metrics-down:
	@echo "🗑️  Removendo stack de métricas..."
	docker compose -f docker-compose.metrics.yml down

# Setup local Git Pre-Commit hooks
init-hooks:
	@echo "⚙️  Configurando Git Pre-Commit Hooks local..."
	@chmod +x .githooks/pre-commit
	@git config core.hooksPath .githooks
	@echo "✅ Hooks configurados com sucesso!"

