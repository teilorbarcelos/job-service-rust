# Stage 1: Builder
FROM rust:1.83-slim-bookworm AS builder

# Instala dependências nativas necessárias para compilar
RUN apt-get update && apt-get install -y pkg-config libssl-dev cmake && rm -rf /var/lib/apt/lists/*

WORKDIR /usr/src/app

# Copia arquivos e compila apenas dependências primeiro para usar cache do Docker
COPY Cargo.toml Cargo.lock ./
RUN mkdir src && echo "fn main() {}" > src/main.rs && cargo build --release && rm -rf src/

# Agora copia o código fonte real e compila a aplicação
COPY . .
# To update the modification time and force rebuild of the real main.rs
RUN touch src/main.rs
RUN cargo build --release

# Stage 2: Runner
FROM debian:bookworm-slim

# Instala certificados CA para requisições HTTPS e bibliotecas necessárias em runtime
RUN apt-get update && apt-get install -y ca-certificates libssl3 && rm -rf /var/lib/apt/lists/*

# Cria usuário não-root (appuser)
RUN useradd -ms /bin/bash appuser

WORKDIR /app

# Copia o binário compilado e altera o dono
COPY --from=builder --chown=appuser:appuser /usr/src/app/target/release/backend-rust /app/backend-rust

# Copia arquivos estáticos
COPY --from=builder --chown=appuser:appuser /usr/src/app/.env.example /app/.env
COPY --from=builder --chown=appuser:appuser /usr/src/app/templates /app/templates

# Garante permissão de execução
RUN chmod +x /app/backend-rust

# Configuração de portas e variáveis padrão
EXPOSE 8888
ENV ENVIRONMENT=production

# Muda para o usuário sem privilégios
USER appuser

CMD ["/app/backend-rust"]
