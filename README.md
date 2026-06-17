# Advanced Rust Backend API 🦀

Uma arquitetura robusta, segura e ultra-performática construída em **Rust**, utilizando as melhores práticas da linguagem para entregar tipagem estática, máxima concorrência assíncrona, arquitetura modular e aderência perfeita às premissas de LGPD, Auditoria, Observabilidade e RBAC do **Mage Compliance Standard**.

---

## 🚀 Tecnologias Core

O projeto utiliza o estado da arte do ecossistema Rust assíncrono:

- **Runtime Assíncrono:** [Tokio](https://tokio.rs/) (O padrão da indústria para alta concorrência)
- **Framework Web:** [Axum](https://github.com/tokio-rs/axum) (Altamente modular, rápido e baseado na stack robusta de `tower` e `hyper`)
- **ORM:** [Sea-ORM](https://www.sea-ql.org/) (ORM premium assíncrono com tipagem segura baseado em SQLx)
- **Database:** PostgreSQL (Principal) & Redis (Cache, Controle de Sessão e Rate Limit)
- **Mensageria (RabbitMQ):** [lapin](https://github.com/CleverCloud/lapin) (Cliente AMQP 0.9.1 puro em Rust assíncrono)
- **Documentação:** Swagger (OpenAPI 3.0) via `utoipa` com UI integrada em `/v1/docs`
- **Linter & Formatter:** Clippy & Rustfmt (Garantia de 100% clean-code)
- **Mensageria de E-mail:** Lettre 0.11 (Integração assíncrona robusta via SMTP)

---

## ✨ Principais Funcionalidades

### 🔐 Segurança e Autenticação
- **RBAC Dinâmico e Modular:** Controle de acesso baseado em perfis (Roles) com permissões granulares por feature (`view`, `create`, `delete`, `activate`) encapsulado em uma macro declarativa limpa e intuitiva: `auth_route!`.
- **Gerenciamento de Sessão via Redis (Session Epoch):** Rastreamento de tokens JWT em tempo real no cache Redis com **invalidação O(1) atômica** via padrão de versionamento (session epoch). Um único `INCR` invalida todas as sessões de um usuário instantaneamente — sem varredura de chaves.
- **Rate Limiting Global com Lua Script:** Middleware nativo Axum integrado ao Redis com **script Lua atômico** via `EVAL`, eliminando race conditions TOCTOU entre `ZCARD` e `ZADD`.
- **CORS Configurável por Ambiente:** Controlado via variável `CORS_ALLOWED_ORIGINS`. Em produção, origens são restritas à lista configurada; em desenvolvimento, permite qualquer origem como fallback.

### 🏗️ Arquitetura Core (Base Layer)
- **Core CRUD Genérico:** Sistema de CRUD genérico através de traits (`CrudEntity`, `CrudActiveModel`) e da macro declarativa `impl_crud_traits!`, centralizando operações comuns de banco de dados (busca por ID, criação com mapeamento de conflitos, atualização, exclusão lógica, alteração de status) e reduzindo drasticamente o código repetitivo em novos módulos.
- **Arquitetura Sem Repositório Boilerplate (DIP):** Chamadas diretas do banco a partir dos Services com Sea-ORM, mantendo o código conciso, ágil e livre de padrões redundantes que poluem o projeto.
- **Response Mappings Elegantes:** Implementação idiomática da trait `From` para converter registros do banco de dados em DTOs de resposta, eliminando mapeamentos manuais repetitivos dos Services.
- **Filtragem Dinâmica:** Módulo `QueryValidator` robusto capaz de validar campos, ordenar dinamicamente, impor limites rígidos de paginação e validar ranges de data de forma automática.
- **Paginação Genérica DRY:** Helper genérico `.paginate()` integrado ao parse de filtros que reduz a lógica de busca e contagem do banco a uma linha simples de código.
- **Soft Delete e LGPD:** Suporte nativo a exclusão lógica (`is_deleted`), combinado com a anonimização automática de dados de usuário em conformidade com as regras da LGPD.

### 📧 Email Infrastructure (DIP)
- **Decoupled Architecture:** Abstração completa através da trait assíncrona `EmailService`, permitindo injeção limpa de drivers.
- **Mock Driver:** `MockEmailService` para simulação visual de e-mails em console durante testes e desenvolvimento.
- **SMTP Driver:** `SmtpEmailService` assíncrono completo que utiliza a biblioteca `lettre` 0.11 com suporte a TLS, credenciais e variáveis de ambiente configuráveis.

### 💬 Mensageria & Integração com RabbitMQ
- **MessagingProvider Abstraído:** Gerenciador global de conexão AMQP com RabbitMQ integrado opcionalmente via `.env` (`MESSAGING_ENABLED=true`).
- **Publicação e Consumo Resilientes:** Suporta reconexão automática, publicação de mensagens JSON tipadas e tratamento resiliente de erros/rejeição com `nack`.

### 📂 Cloud Storage Providers (Multi-Provider CLI)
- **Interface Abstraída:** Suporta uploads transparentes através de `StorageProvider` global que encapsula a trait `StorageService`.
- **Provedor Local Nódigo:** `LocalStorageService` padrão para armazenar arquivos em diretório local durante o desenvolvimento com suporte a rota pública Axum de serving estático.
- **CLI Scaffold Generator:** CLI interativa (`make generate-storage`) que instala e configura provedores de produção sob demanda (**AWS S3**, **Google Cloud Storage (GCS)**, **Azure Blob Storage**) com dependências automáticas e variáveis `.env`.
- **Suporte Offline Automático:** Drivers gerados possuem fallback mock resiliente quando as credenciais não estão definidas em desenvolvimento.

### 📄 PDF Service Integration (Streaming Bypass)
- **Zero Memory Footprint:** O backend funciona como um proxy de streaming direto para o microserviço de PDF. O payload gerado em bytes é transmitido instantaneamente ao cliente sem carregar dados em memória ou disco local.
- **Endpoints de Debug:** Rotas GET/POST dedicadas para validar visualmente templates PDF.
- **Circuit Breaker Integrado:** Proteção contra cascata de falhas com estados `Closed`/`Open`/`HalfOpen`. Após 3 falhas consecutivas, o circuito abre por 10 segundos antes de tentar novamente, evitando timeouts em massa.

### 🖥️ Audit Explorer UI
- Interface administrativa construída diretamente no backend que permite consultar, auditar e inspecionar logs de auditoria e ocorrências de erros registradas no banco de dados.

### 📊 Real-time Observability (Prometheus & Grafana)
- **Métricas Nativas:** Endpoint `/metrics` exportando dados em tempo real sobre requisições, latências e concorrência para Prometheus.
- **Painéis de Grafana Prontos:** Grafana local pré-configurado via Docker Compose para visualização visual de CPU, memória, RPS e taxas de status HTTP das rotas Axum.
- **Health Checks em 3 Níveis:**
  - `/health` e `/liveness` — liveness simples (processo ativo)
  - `/ready` — **deep health check** que valida conectividade com PostgreSQL (`SELECT 1`) e Redis (`PING`), retornando 503 se alguma dependência crítica estiver fora
- **Correlation ID (Request ID):** Middleware que lê/gera `X-Request-ID`, propaga via `tracing::Span` em todos os logs e métricas, e retorna no header da resposta para rastreabilidade ponta a ponta.

### 🔄 Resiliência e Operação

- **Graceful Shutdown:** Tratamento de `SIGINT` e `SIGTERM` via `tokio::signal`. O servidor aguarda requests em voo concluírem e desconecta RabbitMQ ordenadamente antes de desligar.
- **Boot Resiliente com Retry:** Todas as dependências externas (PostgreSQL, Redis, RabbitMQ, migrações, bootstrap, StorageProvider) utilizam retry com 5 tentativas e sleep de 2s entre falhas, com log estruturado antes de abortar.
- **Sanitização de Erros Internos:** Erros de banco de dados (`DbErr`) são logados internamente com `tracing::error!` e retornam ao cliente apenas a mensagem genérica `"Erro interno ao processar a requisição"`, sem vazar schema/queries/nomes de tabela.

---

## 🔌 Plug-and-Play: Auth-Service (Microsserviço)

O `backend-rust` suporta dois modos de operação para autenticação:

| Modo | Descrição | Uso |
|------|-----------|-----|
| **`AUTH_MODE=local`** (default) | Auth gerenciado pelo próprio monólito | Desenvolvimento simples, MVP |
| **`AUTH_MODE=remote`** | Auth delegado ao `auth-service-rust` | Alta concorrência, escalabilidade |

### Modo Monolítico (default)

```bash
# backend-rust/.env
AUTH_MODE=local
```

Nenhuma configuração extra — o monólito gerencia login, refresh, logout e sessão como sempre.

### Modo Microsserviço (opt-in)

```bash
# backend-rust/.env
AUTH_MODE=remote
```

Neste modo:

1. O `backend-rust` **remove** os endpoints `/v1/auth/login`, `/v1/auth/refresh` e `/v1/auth/logout`
2. O middleware JWT **continua funcionando** — valida o token localmente
3. O middleware RBAC **continua funcionando** — lê permissões do Redis
4. A sessão Redis **continua funcionando** — `auth-service-rust` cria as sessões no mesmo Redis
5. O frontend passa a chamar o `auth-service-rust` (porta 8001) para operações de auth

> **Nenhuma alteração no middleware, RBAC ou sessão.** Apenas 3 handlers são desligados.

### Arquitetura

```
FRONTEND                   AUTH SERVICE (8001)         MONOLITH (8888)
   │                            │                          │
   ├─ POST /login ────────────→│                          │
   │                            ├─ SELECT User+Auth+Role  │
   │                            ├─ bcrypt verify          │
   │                            ├─ JWT (HS256)            │
   │                            ├─ Redis: session + perms │
   │←── { token, refresh } ────│                          │
   │                                                      │
   ├─ GET /api (JWT) ───────────────────────────────────→│
   │                          ├─ valida JWT local         │
   │                          ├─ lê permissions do Redis  │
   │                          ├─ RBAC sem chamada de rede │
   │←─────────────────────────────────────────────────────│
```

### Compliance

```bash
# Modo monolítico
cp .env.rust .env && make test-rust        # 48 testes

# Modo auth-service
cp .env.auth.rust .env && make test-auth-rust  # 48 testes
```
- **Docker Multi-Stage:** `Dockerfile` com dois estágios — compilação em `rust:1.83-slim-bookworm` e imagem final mínima em `debian:bookworm-slim` com usuário não-root.

---

## 🛠️ Gerador de Módulos (CLI CRUD Generator) ⚙️

Como o Rust possui uma verbosidade natural devido à sua forte tipagem estática e segurança em tempo de compilação, adicionamos uma ferramenta de CLI interativa para automatizar todo o processo de criação de novos recursos. 

Com um único comando, o gerador automatiza a criação do CRUD completo, a migration correspondente, a documentação Swagger OpenAPI, as permissões RBAC no banco de dados, e a **suíte completa de testes de integração**.

### 🎮 Como utilizar

#### Método 1: Modo Interativo (Recomendado)
Basta digitar o seguinte comando no terminal:
```bash
make generate
```
Se nenhum argumento for fornecido, a CLI iniciará o assistente interativo por prompts com seleção por setas e Enter:

1. **Nome da Entidade:** Digite em PascalCase (ex: `ProductCategory`).
2. **Definição de Campos:** Digite o nome do campo. Em seguida, selecione o tipo e o nível de obrigatoriedade usando as setas:
   - **Tipos disponíveis:** `string` (VARCHAR(255)), `text` (TEXT), `int` (INTEGER), `bool` (BOOLEAN), `decimal` (NUMERIC(10,2)), `float` (DOUBLE PRECISION), `date` (TIMESTAMP WITH TIME ZONE).
   - **Obrigatoriedade:** `Nullable (opcional)` (padrão) ou `Not Null (obrigatório)`.
3. **Registro no RBAC:** Escolha se deseja registrar a feature no sistema de controle de acesso (RBAC). Se sim, informe o ID, nome e descrição da feature.

#### Método 2: Modo Direct CLI (Passagem de Parâmetros)
Você também pode rodar o comando fornecendo os argumentos diretamente no terminal:
```bash
make generate name=Category fields="name:string:notnull description:text active:bool"
```
*Formato:* `campo:tipo` (opcional/nullable por padrão) ou `campo:tipo:notnull` (obrigatório).

### 📂 Arquivos Gerados Automaticamente
Ao rodar o gerador para a entidade `Category`, ele criará e registrará a seguinte estrutura de arquivos:

* 📄 **`src/models/category.rs`** - Entidade de banco mapeada via Sea-ORM.
* 📄 **`src/modules/category/schemas.rs`** - DTOs de entrada e saída (CreateRequest, UpdateRequest, Response).
* 📄 **`src/modules/category/service.rs`** - Regras de negócio, paginação DRY e filtragem dinâmica.
* 📄 **`src/modules/category/controller.rs`** - Handlers Axum mapeando requisições e OpenAPI Docs.
* 📄 **`src/modules/category/routes.rs`** - Definição de rotas HTTP protegidas por RBAC.
* 📄 **`src/modules/category/mod.rs`** - Arquivo centralizador do módulo.
* 📄 **`src/migration/mYYYYMMDD_HHMMSS_create_category_table.rs`** - Script de migration SQL para o banco.
* 📄 **`tests/compliance/t17_category.rs`** - Arquivo contendo todos os cenários de testes de integração do CRUD.
* 📝 **`src/models/mod.rs`** - Auto-registro do model.
* 📝 **`src/modules/mod.rs`** - Auto-registro do módulo Axum.
* 📝 **`src/migration/mod.rs`** - Auto-registro do script de migração.
* 📝 **`src/modules/observability.rs`** - Integração automática aos Swagger OpenAPI Docs.
* 📝 **`src/infra/bootstrap.rs`** - Auto-registro da nova feature nos perfis RBAC do banco.
* 📝 **`tests/compliance/mod.rs` & `tests/integration_tests.rs`** - Inclusão da suite de testes de conformidade.

---

## 🧬 Core CRUD (Generic CRUD Layer)

Para evitar a escrita repetitiva de rotinas CRUD (Create, Read, Update, Delete), o projeto possui uma infraestrutura genérica de CRUD centralizada em `src/core/crud.rs`.

### Como Funciona:

1. **Implementação de Traits no Modelo (`src/models/`):**
   Utilizando a macro `impl_crud_traits!`, declaramos as capacidades de filtragem, busca, ordenação e tratamento de erros do modelo:
   ```rust
   crate::impl_crud_traits!(
       Entity,
       ActiveModel,
       Column::IsDeleted,
       Column::Active,
       "Entidade não encontrada",
       |_| "Conflito ao criar registro".to_string(),
       |_| "Conflito ao atualizar registro".to_string(),
       {
           // Definições de filtros dinâmicos
           vec![FilterDefinition::contains("name", (Entity, Column::Name))]
       },
       // Definições de busca por string
       vec![SearchDefinition::contains("name", (Entity, Column::Name))],
       // Definições de ordenação
       vec![OrderDefinition::column("createdAt", (Entity, Column::CreatedAt))],
       Column::CreatedAt // Coluna de ordenação padrão
   );
   ```

2. **Validação Automática no Controller (`src/modules/`):**
   Valide parâmetros de consulta, busca e ordenação enviados via Query String diretamente no controller:
   ```rust
   let parsed_filters = crate::core::crud::validate_and_parse::<user::Entity>(&params)?;
   ```

3. **Uso Simplificado no Service (`src/modules/`):**
   Os métodos de serviço podem delegar o trabalho pesado aos utilitários genéricos:
   - **Listagem:** `crate::core::crud::list_records` ou `list_records_with_query` para queries com JOINs.
   - **Busca por ID:** `crate::core::crud::get_by_id::<Entity>(id, db)` (lança erro `NotFound` formatado automaticamente).
   - **Criação:** `crate::core::crud::create_record::<Entity, _>(db, active_model)` (lança erro `Conflict` formatado automaticamente).
   - **Atualização:** `crate::core::crud::update_record::<Entity, _>(db, active_model)`.
   - **Status (Ativo/Inativo):** `crate::core::crud::toggle_status::<Entity, ActiveModel>(id, active, db)`.

---

## 📂 Cloud Storage Provider Generator ☁️

Para adicionar drivers de armazenamento em nuvem sob demanda, use o utilitário CLI:

```bash
make generate-storage
```

Selecione o provedor desejado:
1. **AWS S3**
2. **Google Cloud Storage (GCS)**
3. **Azure Blob Storage**

A CLI irá automaticamente copiar o template otimizado, instalar as dependências necessárias no `Cargo.toml`, registrar o novo provedor no bootstrap do `StorageProvider` e configurar as variáveis no arquivo `.env`.

---

## 🛡️ Qualidade de Código & Automação Git

Mantemos um padrão de elite absoluto de integridade e limpeza de código:

### Pre-commit Hooks Nativos (Zero Dependencies)
Para registrar o hook de commit nativo no repositório local, execute uma única vez:

```bash
make init-hooks
```

Este hook interceptará os commits locais e garantirá:
1. **`cargo fmt`:** O código deve estar 100% formatado segundo as regras da linguagem.
2. **`cargo clippy`:** Zero warnings permitidas!
3. **Detector de Comentários Legados:** Proíbe o commit de restos de códigos comentados (ex: `// let x = 1;`).
4. **Doc-Comments:** Evita documentações de código vazias ou incompletas.
5. **Comentários Inline:** Proíbe comentários de linha (`//` ou `///`) em arquivos de código fonte para manter o código autodescritivo.
6. **Cobertura Mínima de Testes (Tarpaulin):** Garante que a cobertura de linhas esteja em no mínimo **95%** antes de aceitar o commit.

---

## ⚙️ Configuração Local

### 🛠️ Instalação de Pré-requisitos
Antes de compilar, instale os cabeçalhos de desenvolvimento do PostgreSQL e OpenSSL em seu sistema Linux:
```bash
sudo apt update
sudo apt install -y build-essential libssl-dev pkg-config libpq-dev
```

### Variáveis de Ambiente
Copie o arquivo `.env.example` para `.env` e configure suas variáveis locais:
```bash
cp .env.example .env
```

### Gerenciamento da Infraestrutura (Docker)
```bash
make infra-up       # Sobe Postgres, Redis e RabbitMQ em segundo plano
make infra-stop     # Pausa os containers de infraestrutura
make infra-down     # Remove os containers locais de infraestrutura
make infra-clean    # Remove containers, volumes persistidos e imagens locais
```

### Executando o Servidor de Desenvolvimento
```bash
make dev            # Inicia o servidor com hot-reload (cargo watch)
```

### Executando Testes e Cobertura
```bash
cargo test          # Roda todos os testes (unitários e integração em paralelo)
make coverage       # Gera relatório detalhado de cobertura (cargo tarpaulin)
```

---

## 🧪 CI/CD (GitHub Actions)

A cada push ou pull request nas branches `main` e `develop`, o pipeline automatizado realiza as seguintes validações em ambiente limpo:
1. **Setup de Infraestrutura:** Inicializa o Postgres, Redis e RabbitMQ via Docker Compose e aguarda a saúde dos serviços.
2. **Format Check:** Executa `cargo fmt --all -- --check`.
3. **Clippy Static Analysis:** Executa `cargo clippy --all-targets --all-features -- -D warnings`.
4. **Testes Unitários e de Integração:** Roda `cargo test` para atestar a funcionalidade completa da plataforma.

---

## 📖 API Documentation & Observability

Portas e URLs padrão dos serviços locais:

- **Swagger UI (OpenAPI 3.0):** `http://localhost:8888/v1/docs`
- **Health Check:** `http://localhost:8888/health`
- **Prometheus Metrics:** `http://localhost:8888/metrics`
- **Liveness Probe:** `http://localhost:8888/liveness`
- **PDF Debug Template (GET/POST):** `http://localhost:8888/v1/debug/pdf`
- **Audit Explorer UI:** `http://localhost:8888/v1/audit/explore`