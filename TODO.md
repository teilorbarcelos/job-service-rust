# Roadmap — job-service-rust

> Esqueleto (boilerplate) para execução de jobs agendados em Rust 1.75+.
> Conecta-se ao `backend-rust` para consumir PostgreSQL, Redis e RabbitMQ.

---

## Visao

Fornecer um **ponto de partida enxuto e idiomático** para quem precisa rodar jobs
agendados (cron) em Rust — sem toda a complexidade de um backend HTTP completo.
O backend continua sendo dono do schema, dos migrations, do seed e do ciclo de vida
do banco. Os jobs apenas **consomem** esses serviços para executar tarefas recorrentes
(limpezas, sincronizações, health checks, relatórios, etc.).

### Principios

- **S** — Single Responsibility (cada job tem um único propósito)
- **O** — Open/Closed (adicionar um job = criar uma struct + 1 linha de registro)
- **L** — Liskov Substitution (todo `BaseJob` é intercambiável)
- **I** — Interface Segregation (dependências injetadas via construtor)
- **D** — Dependency Inversion (jobs dependem de traits, não de implementações)
- **DRY** — Lógica compartilhada fica em `BaseJob`; nunca duplicada
- **Clean Code** — Nomes expressivos, funções curtas, sem side-effects implícitos

---

## Arquitetura

```
src/
├── main.rs                          # Entry point (boot + graceful shutdown + loop)
├── lib.rs                           # Re-exports de modulos
├── core/
│   ├── job.rs                       # Trait BaseJob + JobContext + JobResult
│   ├── job_status.rs                # Enum: SUCCESS, FAILED, CANCELLED, TIMEOUT
│   ├── cron.rs                      # CronAdapter trait + CronExpressionAdapter
│   ├── scheduler.rs                 # Scheduler (start/stop/tick)
│   └── errors.rs                    # AppError, ConfigurationError, ConnectionError
├── infra/
│   ├── database.rs                  # SeaORM (postgres + sqlite test)
│   ├── redis.rs                     # deadpool-redis
│   ├── messaging.rs                 # lapin (Publisher + IsOpen)
│   └── health.rs                    # HealthChecker (PG + Redis + Rabbit)
├── jobs/
│   ├── health_check_job.rs          # Diagnostico a cada minuto
│   └── register_jobs.rs             # Registro central (1 lista)
├── shared/
│   ├── config.rs                    # AppConfig via env vars
│   └── logger.rs                    # tracing setup estruturado
└── register_jobs!()                 # Macro que retorna Vec<Box<dyn BaseJob>>
tests/
├── core/                            # Testes do core
├── infra/                           # Testes dos providers
├── jobs/                            # Testes dos jobs
└── shared/                          # Testes de config/utils
```

### Fluxo de vida

```
SIGTERM/SIGINT (Container shutdown)
   |
   v
scheduler.stop()  -->  cancel job contexts
                   -->  wait for in-flight jobs
                   -->  close rabbit channel + connection
                   -->  close redis pool
                   -->  close db pool
                   -->  application exit
```

---

## Fases

### Fase 0 — Diagnostico (concluido)

- [x] Mapear o que existe (clone do `backend-rust` — Axum + sea-orm + JWT + audit + RabbitMQ consumer + Prometheus + Swagger)
- [x] Identificar o que precisa sair (API HTTP, modules, auth, audit, PDF, storage, swagger, migrations, tower-http, axum)

### Fase 1 — Limpeza estrutural

- [ ] `Cargo.toml`:
  - **Remover**: `axum`, `tower`, `tower-http`, `serde_json` (manter so em dev), `sea-orm`, `sea-orm-migration`, `bcrypt`, `jsonwebtoken`, `utoipa`, `utoipa-swagger-ui`, `prometheus`, `lettre`, `dialoguer`, `reqwest`, `validator`, `async-trait`, `dotenvy` (manter), `futures-util` (so em dev), `tower-http`
  - **Manter**: `tokio`, `serde`, `redis`, `deadpool-redis`, `lapin`, `chrono`, `tracing`, `tracing-subscriber`, `uuid`, `once_cell`
  - **Adicionar**: `cron` (parser), `dotenvy`
  - Renomear: `name = "job-service-rust"`, `default-run = "job-service-rust"`
- [ ] `src/main.rs`:
  - Remover todo o setup de Axum
  - Manter apenas: tracing init + signal handling + scheduler loop
- [ ] `src/lib.rs`:
  - Remover `mod migration` (jobs nao fazem migrations)
  - Manter `core`, `infra`, `jobs`, `shared`
- [ ] Remover `src/middleware/` (inteiro)
- [ ] Remover `src/modules/` (inteiro: auth, user, role, product, audit, dashboard, upload, audit_explorer, observability)
- [ ] Remover `src/models/` (inteiro, jobs nao usam Eloquent/sea-orm models)
- [ ] Remover `src/infra/`: `auth.rs`, `bootstrap.rs`, `email.rs`, `pdf.rs`, `storage/`
  - **Manter** e refatorar: `database.rs`, `messaging/` (lapin), `cache.rs` (renomear para `redis.rs`)
- [ ] Remover `src/migration/` (jobs nao fazem migrations)
- [ ] Remover `src/bin/` (CLI antiga)
- [ ] Remover `tests/common/`, `tests/compliance/`, `tests/integration_tests.rs` (legado)
- [ ] Remover `docker-compose.metrics.yml` (Prometheus/Grafana nao fazem parte deste esqueleto)
- [ ] Limpar `Makefile`, `Dockerfile`
- [ ] Remover `templates/` (CRUD generator nao faz sentido para jobs)
- [ ] Limpar `magerc.json`
- [ ] Remover `infra/` (Caddy, Prometheus, Grafana)

### Fase 2 — Core de jobs

- [ ] `src/core/job_status.rs`:
  - `enum JobStatus { Success, Failed, Cancelled, Timeout }`
  - Display impl, From impl
- [ ] `src/core/job.rs`:
  - `pub trait BaseJob: Send + Sync { fn name(&self) -> &str; fn schedule(&self) -> &str; fn description(&self) -> &str; fn enabled(&self) -> bool; async fn handle(&self, ctx: JobContext) -> Result<(), AppError>; }`
  - `pub struct JobContext { pub logger: Arc<Logger>, pub signal: Arc<JobSignal> }`
  - `pub struct JobResult { pub job: String, pub status: JobStatus, pub duration_ms: u64, pub error: Option<String> }`
  - `pub struct JobInfo { pub name, schedule, enabled, description }`
  - `pub fn execute_job<F>(job: &str, fut: F) -> JobResult where F: Future<Output=Result<(), AppError>>` — stopwatch + try/catch
- [ ] `src/core/job_signal.rs`:
  - `pub struct JobSignal { aborted: AtomicBool }`
  - `pub fn abort(&self)`, `pub fn aborted(&self) -> bool`, `pub fn throw_if_aborted(&self) -> Result<(), AppError>`
- [ ] `src/core/cron.rs`:
  - `pub trait CronAdapter: Send + Sync { fn is_valid(&self, expr: &str) -> bool; fn next_run_date(&self, expr: &str, from: DateTime<Utc>) -> Option<DateTime<Utc>>; }`
  - `pub struct CronExpressionAdapter` (impl usando crate `cron`)
- [ ] `src/core/scheduler.rs`:
  - `pub struct Scheduler { jobs: Vec<Arc<dyn BaseJob>>, cron: Arc<dyn CronAdapter>, logger: Arc<Logger>, timeout_ms: u64, running: Mutex<HashSet<String>>, stopped: AtomicBool }`
  - `pub fn new(jobs, cron, logger, timeout_ms) -> Result<Self, AppError>` — valida nomes duplicados
  - `pub async fn start(&self)` — valida cron expressions de cada job
  - `pub fn stop(&self)` — seta stopped
  - `pub async fn wait_for_running_jobs(&self)` — aguarda fila drenar
  - `pub fn is_running(&self, name: &str) -> bool`
  - `pub fn list_jobs(&self) -> Vec<JobInfo>`
  - `pub async fn tick(&self)` — verifica se algum job deve rodar
  - Loop principal: `while !stopped { tick(); sleep(1s); }` (ou via tokio::time::interval)
  - Previne sobreposicao
  - Aplica timeout via `tokio::time::timeout`
- [ ] `src/core/errors.rs`:
  - `pub enum AppError { Configuration(String), Connection(String), Job(String), Validation(String) }`
  - Display + Error + From impls

### Fase 3 — Infra de jobs

- [ ] `src/infra/database.rs`:
  - Singleton `Database` usando `sqlx` (postgres + sqlite):
    - **Decisao**: usar `sqlx` direto em vez de `sea-orm` (mais leve, suficiente para health check)
  - `pub async fn connect(config: &DatabaseConfig) -> Result<Self, AppError>`
  - `pub fn pool(&self) -> &Pool<Postgres>` (ou `SqlitePool`)
  - `pub async fn ping(&self) -> bool`
  - `pub async fn close(&self)`
- [ ] `src/infra/redis.rs`:
  - Singleton `RedisProvider` usando `deadpool-redis`:
  - `pub async fn connect(config: &RedisConfig) -> Result<Self, AppError>`
  - `pub fn pool(&self) -> &Pool`
  - `pub async fn ping(&self) -> bool`
  - `pub async fn close(&self)`
- [ ] `src/infra/messaging.rs`:
  - `pub struct MessagingProvider` usando `lapin`:
  - `pub async fn connect(config: &RabbitConfig) -> Result<Self, AppError>` com retry
  - `pub async fn publish(&self, queue: &str, msg: &[u8]) -> Result<(), AppError>`
  - `pub fn is_open(&self) -> bool`
  - `pub async fn close(&self)` — fecha channel + connection
  - **Remover** `subscribe()` (jobs publicam, nao consomem)
- [ ] `src/infra/health.rs`:
  - `pub trait HealthChecker: Send + Sync { async fn check_postgres(&self) -> HealthCheckResult; async fn check_redis(&self) -> HealthCheckResult; async fn check_rabbitmq(&self) -> HealthCheckResult; }`
  - `pub struct HealthCheckResult { pub status: String, pub latency_ms: Option<u64>, pub error: Option<String> }`
  - `pub struct DefaultHealthChecker { db: Arc<Database>, redis: Arc<RedisProvider>, rabbit: Arc<MessagingProvider> }`

### Fase 4 — Shared (Config + Logger)

- [ ] `src/shared/config.rs`:
  - `pub struct AppConfig { env, log_level, shutdown_timeout_ms, job_execution_timeout_ms, database: DatabaseConfig, redis: RedisConfig, messaging: MessagingConfig, jobs: JobsConfig }`
  - `pub struct DatabaseConfig { driver, url, host, port, database, username, password }`
  - `pub struct RedisConfig { host, port, password, db }`
  - `pub struct MessagingConfig { enabled, host, port, user, password }`
  - `pub struct JobsConfig { health_check_cron: String, health_check_enabled: bool }`
  - `pub fn load() -> Result<AppConfig, AppError>` — le env vars com defaults, valida tipos
- [ ] `src/shared/logger.rs`:
  - `pub fn setup_tracing(level: &str) -> tracing_subscriber::EnvFilter` — structured JSON logging

### Fase 5 — Módulo Jobs

- [ ] `src/jobs/health_check_job.rs`:
  - `pub struct HealthCheckJob { checker: Arc<dyn HealthChecker> }`
  - `impl BaseJob for HealthCheckJob { ... }`
  - Cron `*/1 * * * *` (configuravel via `HEALTH_CHECK_CRON`)
  - `handle()`:
    - Chama os 3 checkers em paralelo via `tokio::join!`
    - Calcula `all_up` (status == "up" em todos)
    - Loga `[HealthCheck ISO] postgres=up redis=up rabbitmq=up` no stdout
    - Retorna Ok(()) ou erro
- [ ] `src/jobs/register_jobs.rs`:
  - `pub fn register_jobs(config: &AppConfig, cron: Arc<dyn CronAdapter>, logger: Arc<Logger>, checker: Arc<DefaultHealthChecker>) -> Vec<Arc<dyn BaseJob>>`
  - Instancia `HealthCheckJob`, aplica `enabled` e `schedule` do config
  - Markers `// [GENERATOR_IMPORTS]` e `// [GENERATOR_JOBS]` para Fase 8
- [ ] `src/main.rs`:
  - `let config = AppConfig::load()?;`
  - `tracing_subscriber::fmt()...init();`
  - `let db = Database::connect(&config.database).await?;`
  - `let redis = RedisProvider::connect(&config.redis).await?;`
  - `let rabbit = MessagingProvider::connect(&config.messaging).await?;`
  - `let checker = Arc::new(DefaultHealthChecker { db, redis, rabbit });`
  - `let cron = Arc::new(CronExpressionAdapter);`
  - `let jobs = register_jobs(&config, cron, logger, checker);`
  - `let scheduler = Scheduler::new(jobs, ...)?;`
  - `tokio::select! { _ = scheduler.run() => {}, _ = shutdown_signal() => {} }`
  - Cleanup: fechar db/redis/rabbit, esperar jobs em curso

### Fase 6 — Testes (100% cobertura)

- [ ] `tests/core/job_status_test.rs` — todos os variants + Display
- [ ] `tests/core/job_signal_test.rs` — abort, throw_if_aborted (ok + aborted)
- [ ] `tests/core/job_test.rs` — execute_job success, failure, cancelado, duration, error message
- [ ] `tests/core/cron_test.rs` — CronExpressionAdapter: valid/invalid, next_run_date
- [ ] `tests/core/scheduler_test.rs`:
  - Novo job: lista
  - Nomes duplicados: throws
  - Cron invalido: throws
  - Jobs desabilitados: nao agendam
  - stop: para tasks
  - wait_for_running_jobs: aguarda
  - is_running: reflete estado
  - list_jobs: retorna info
  - Previne overlap
  - Aplica timeout
- [ ] `tests/core/errors_test.rs` — Display, Error impl, From
- [ ] `tests/infra/database_test.rs` — connection ok, ping, close (com SQLite in-memory)
- [ ] `tests/infra/redis_test.rs` — ping (com mock), close
- [ ] `tests/infra/messaging_test.rs` — connect (com mock), publish (com mock), is_open, close
- [ ] `tests/infra/health_test.rs` — todos os branches de cada check (mocks dos providers)
- [ ] `tests/shared/config_test.rs` — load defaults, overrides, invalidos
- [ ] `tests/shared/logger_test.rs` — setup_tracing com varios niveis
- [ ] `tests/jobs/health_check_job_test.rs`:
  - 3 checkers up -> loga healthy
  - postgres down -> loga degraded
  - redis down -> loga degraded
  - rabbitmq down -> loga degraded
  - rabbitmq disabled -> loga `rabbitmq=disabled`
  - Imprime linha no stdout com formato `[HealthCheck ISO] postgres=up redis=up rabbitmq=up`
  - run() retorna status SUCCESS
- [ ] `tests/jobs/register_jobs_test.rs` — retorna Vec com HealthCheckJob configurado, HEALTH_CHECK_ENABLED=false desabilita
- [ ] Configurar `tarpaulin.toml`:
  - `out = ["Html", "Xml"]`
  - `output-dir = "coverage"`
  - `exclude-files = ["*/tests/*"]`
  - `fail-under = 80` (ajustar conforme necessario)
- [ ] `tarpaulin --workspace --timeout 300 --out Html --out Xml` gera clover.xml

### Fase 7 — Configuracao e DevOps

- [ ] `Cargo.toml`:
  - Garantir `edition = "2021"` e `rust-version = "1.75"`
  - Adicionar `profile.release` com `lto = true` e `strip = true`
  - Manter deps minimas (tokio, redis, lapin, cron, tracing, etc.)
- [ ] `.env.example`:
  - Apenas envs de job:
    ```
    APP_ENV=development
    APP_DEBUG=true
    LOG_LEVEL=info
    SHUTDOWN_TIMEOUT_MS=30000
    JOB_EXECUTION_TIMEOUT_MS=300000
    DATABASE_URL=postgres://postgres:postgrespw@localhost:5432/backend_rust
    REDIS_HOST=localhost
    REDIS_PORT=6379
    MESSAGING_ENABLED=false
    RABBIT_HOST=localhost
    RABBIT_PORT=5672
    RABBIT_USER=guest
    RABBIT_PASSWORD=guest
    HEALTH_CHECK_CRON=*/1 * * * *
    HEALTH_CHECK_ENABLED=true
    ```
- [ ] `.env.production.example`:
  - Versao endurecida (sem defaults em senhas, APP_DEBUG=false, MESSAGING_ENABLED=true)
- [ ] `docker-compose.yml`:
  - **Remover** servicos `db`, `redis`, `rabbitmq` (jobs consomem servicos externos)
  - Manter so `app` (rust:1.75-slim ou debian-slim com build deps)
- [ ] `docker-compose.infra.yml` (novo):
  - Igual ao `job-service-node` / `job-service-java`: `db` (postgres:15), `redis` (redis:7), `rabbitmq` (rabbitmq:3-management)
- [ ] `Dockerfile`:
  - Multi-stage: `rust:1.75-slim AS builder` (com `cargo build --release`), depois `debian:bookworm-slim` com o binario
  - `CMD ["./job-service-rust"]`
- [ ] `Makefile`:
  - Alvo `help` como no `job-service-node`
  - `dev` -> roda local (sem docker, `cargo run`)
  - `test` -> `cargo test --no-fail-fast`
  - `coverage` -> `cargo tarpaulin --workspace --timeout 300 --out Html --out Xml`
  - `lint` -> `cargo clippy --all-targets --all-features -- -D warnings`
  - `check` -> `lint + test`
  - `infra-up` -> `docker compose -f docker-compose.infra.yml up -d`
  - `infra-stop` / `infra-down` / `infra-clean`
  - `generate-job` -> CLI para criar job
  - `build` / `run` Docker
  - `sonar` -> `./scripts/sonar-scan.sh`
  - `clean` -> `cargo clean`
- [ ] `scripts/start.sh`:
  - Simplificado: `exec ./job-service-rust` (sem migrate, sem web server)
- [ ] `scripts/sonar-scan.sh`:
  - Baseado no `auth-service-php/scripts/sonar-scan.sh` / `job-service-node`
  - Project key fixo em `job-service-rust`
  - Sonar host: `http://localhost:9000` (porta **9000**, credenciais **admin:Admin@123456**)
  - Token via `SONAR_TOKEN` env var (obrigatorio)
- [ ] `magerc.json`:
  - Scripts: install, infra-up, infra-stop, dev, test, coverage, generate-job, sonar
- [ ] `.github/workflows/ci.yml`:
  - `cargo build` + `cargo test` + `cargo clippy` + `cargo tarpaulin`
  - Usa `services: postgres, redis, rabbitmq` ou `docker-compose.infra.yml`
- [ ] `.gitignore`:
  - Manter `TODO.md`, `target/`, `coverage/`, `.scannerwork/`, `*.log`
- [ ] `README.md`:
  - Reescrito do zero (foco em jobs, como o `job-service-node`)

### Fase 8 — Generator de jobs

- [ ] `scripts/generate-job.rs` ou sub-comando em `Cargo.toml`:
  - CLI com args: name, schedule, description
  - Converte kebab/snake/pascal → PascalCase
  - Strip sufixo "Job" se duplicado
  - Renderiza templates em `templates/job/`
  - Atualiza `src/jobs/register_jobs.rs` nos markers `// [GENERATOR_IMPORTS]` e `// [GENERATOR_JOBS]`
- [ ] `templates/job/job.rs.tpl`:
  - Stub completo de `BaseJob` com `name`, `schedule`, `description`, `handle()`
- [ ] `templates/job/job_test.rs.tpl`:
  - 5 testes: enabled, handle success, handle throws, abort, run lifecycle
- [ ] `src/jobs/register_jobs.rs`:
  - Markers `// [GENERATOR_IMPORTS]` e `// [GENERATOR_JOBS]` (ja previsto na Fase 5)
- [ ] `Makefile`:
  - Target `generate-job name=...` (ja previsto na Fase 7)
- [ ] `README.md`:
  - Secao "Como adicionar um novo job" com exemplo do generator

### Fase 9 — SonarQube

- [ ] Subir o SonarQube (ja esta rodando na porta **9000**, credenciais **admin:Admin@123456**)
- [ ] Garantir que o Sonar scanner esta em `$PATH` (ou ajustar o caminho em `scripts/sonar-scan.sh`)
- [ ] Subir a infraestrutura para teste:
  ```bash
  make infra-up
  ```
- [ ] Rodar `make coverage` para gerar `coverage/clover.xml` (via tarpaulin)
- [ ] Rodar `make sonar` (executa `scripts/sonar-scan.sh`)
- [ ] Verificar Quality Gate: **0 bugs, 0 vulns, 0 smells, 0 hotspots, cobertura em new code**
- [ ] Resolver issues remanescentes (ajustar `cargo clippy -- -D warnings` se necessario, adicionar `// NOSONAR` em casos justificados)
- [ ] Validar a pagina do projeto: `http://localhost:9000/dashboard?id=job-service-rust`

---

## Estado atual

**Fase 0 concluida.** Iniciando Fase 1 (limpeza estrutural).

> Antes de iniciar a Fase 1, validar pre-requisitos:
> - Rust 1.75+ disponivel (validar com `rustc --version`)
> - Cargo disponivel
> - Docker disponivel (para tarpaulin em container, se necessario)
> - SonarQube rodando em `http://localhost:9000` (ja validado)
> - `Cargo.toml` e `Cargo.lock` consistentes

---

## Proximos passos (opcional, nao incluso no esqueleto)

- [ ] **Metricas Prometheus para jobs** — counter `job_executions_total{job,status}`,
      histogram `job_duration_seconds{job}`, gauge `job_last_success_timestamp{job}`
      (expor via `prometheus` crate em `actix-web` ou `axum` em porta separada)
- [ ] **Persistencia de execucoes** — tabela `job_run` com `id`, `job`, `started_at`,
      `finished_at`, `status`, `error`, `duration_ms` (consumida pelo backend via sea-orm)
- [ ] **Distributed lock** — `redis.set(key, value, "NX", "EX", ttl)` para garantir
      que apenas uma instancia rode cada job por vez
- [ ] **Retry com backoff** — `BaseJob` com suporte a `max_retries` e `retry_delay_ms`
- [ ] **One-shot jobs** — alem de cron, suporte a `run_once()` para jobs disparados
      manualmente ou por evento (mensagem RabbitMQ)
- [ ] **CLI de execucao** — `cargo run -- run health-check` para rodar um job
      especifico uma vez
- [ ] **Distributed tracing** — propagar `traceparent` via `tracing-actix-web`/`tower-http`
- [ ] **Job queue consumer** — re-adicionar `subscribe()` ao MessagingProvider para
      jobs que disparam ao receber mensagem
- [ ] **Health HTTP endpoint** — pequeno servidor HTTP (`axum` em `0.0.0.0:9090`)
      com `/health` e `/jobs` para inspecao
