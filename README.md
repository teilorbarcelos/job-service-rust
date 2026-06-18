# Job Service Rust

> Esqueleto (boilerplate) para execucao de jobs agendados em Rust 1.75+.
> Conecta-se ao `backend-rust` para consumir PostgreSQL, Redis e RabbitMQ.

## Stack

| Camada | Tecnologia |
|--------|------------|
| Runtime | Rust 1.75+ (tokio async) |
| DB | sqlx (Postgres 16 / SQLite) |
| Cache | deadpool-redis (Redis 7) |
| Mensageria | lapin (RabbitMQ 3) |
| Cron | cron (crate) |
| Logger | tracing + tracing-subscriber (JSON) |
| Qualidade | clippy + cargo-test + cargo-tarpaulin + SonarQube |

## Comandos

```bash
make dev              # Roda o scheduler (cargo run)
make test             # cargo test (55 testes)
make lint             # cargo clippy -- -D warnings
make infra-up         # Sobe PG + Redis + Rabbit (Docker)
make sonar            # Scan SonarQube
```

## Estrutura

```
src/
├── main.rs                   # Entry point (loop + graceful shutdown)
├── lib.rs                    # Re-exports
├── core/                     # BaseJob trait, Scheduler, CronAdapter, JobSignal
├── infra/                    # Database (sqlx), Redis (deadpool), Messaging (lapin), Health
├── jobs/                     # HealthCheckJob + register_jobs
└── shared/                   # Config (env vars), Logger (tracing)

tests/                        # 55 testes unitarios
```

## Adicionar um novo job

Editar `src/jobs/register_jobs.rs` e adicionar na lista:

```rust
jobs.push(Arc::new(MeuJob::new()));
```

Implementar `BaseJob` trait em `src/jobs/meu_job.rs`.
