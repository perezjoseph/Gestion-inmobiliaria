# Arquitectura del Sistema

Plataforma de gestión de propiedades para administradores inmobiliarios en República Dominicana.

## Diagrama General

```
┌─────────────────────────────────────────────────────────────────────────────────┐
│                              INTERNET / CLIENTES                                 │
└───────────┬─────────────────────────────────┬───────────────────────────────────┘
            │ HTTPS                           │ HTTPS
            ▼                                 ▼
┌───────────────────────┐          ┌─────────────────────┐
│   Android App         │          │   Navegador Web      │
│   (Kotlin + Compose)  │          │                     │
│   ─────────────────   │          └──────────┬──────────┘
│   feature:auth        │                     │
│   feature:dashboard   │                     │
│   feature:propiedades │                     │
│   feature:inquilinos  │                     │
│   feature:contratos   │                     │
│   feature:pagos       │                     │
│   feature:gastos      │                     │
│   feature:chatbot     │                     │
│   ... (19 módulos)    │                     │
└───────────┬───────────┘                     │
            │ /api/v1/*                       │
            ▼                                 ▼
┌─────────────────────────────────────────────────────────────────────────────────┐
│                         KUBERNETES CLUSTER (k3s)                                 │
│                         Namespace: realestate                                    │
│                                                                                  │
│  ┌────────────────────────────────────────────────────────────────────────────┐  │
│  │                     TRAEFIK INGRESS                                         │  │
│  │                     Host: gestion.local                                     │  │
│  │                     HTTP → HTTPS redirect                                   │  │
│  └────────────────────────────────────┬───────────────────────────────────────┘  │
│                                       │                                          │
│                                       ▼                                          │
│  ┌────────────────────────────────────────────────────────────────────────────┐  │
│  │                     FRONTEND (Caddy :8443)                                  │  │
│  │                     ───────────────────────                                 │  │
│  │  • Yew 0.21 (Rust → WASM) SPA                                              │  │
│  │  • Caddy sirve archivos estáticos + TLS interno                            │  │
│  │  • Proxy reverso: /api/* → backend:8080                                    │  │
│  │  • Proxy reverso: /uploads/* → backend:8080                                │  │
│  │  • Security headers (CSP, HSTS, X-Frame-Options)                           │  │
│  │  • SPA fallback: try_files → /index.html                                   │  │
│  └────────────────────────────────────┬───────────────────────────────────────┘  │
│                                       │ /api/*, /uploads/*                       │
│                                       ▼                                          │
│  ┌────────────────────────────────────────────────────────────────────────────┐  │
│  │                     BACKEND (Actix-web :8080) × 2 réplicas                  │  │
│  │                     ──────────────────────────────────────                  │  │
│  │  • Rust / Actix-web 4 + SeaORM                                              │  │
│  │  • JWT auth + RBAC (admin, gerente, visualizador)                           │  │
│  │  • Rate limiting por grupo de rutas (actix-governor)                        │  │
│  │  • Prometheus métricas + health checks                                      │  │
│  │  • Background job scheduler (pagos vencidos, notificaciones)                │  │
│  │  • SMTP client (Mailcow) para correos                                       │  │
│  │  • NFS shared uploads volume                                                │  │
│  │                                                                              │  │
│  │  Dominios:                                                                   │  │
│  │  ┌──────────────┬──────────────┬──────────────┬──────────────┐              │  │
│  │  │ auth         │ propiedades  │ inquilinos   │ contratos    │              │  │
│  │  │ pagos        │ gastos       │ mantenimiento│ dashboard    │              │  │
│  │  │ auditoria    │ usuarios     │ perfil       │ notificaciones│             │  │
│  │  │ reportes     │ documentos   │ configuracion│ importacion  │              │  │
│  │  │ chatbot      │ desahucios   │ dgii         │ firmas       │              │  │
│  │  │ ocr          │ tareas       │ recibos      │ organizacion │              │  │
│  │  └──────────────┴──────────────┴──────────────┴──────────────┘              │  │
│  └───┬──────────────────┬──────────────────┬──────────────────┬───────────────┘  │
│      │                  │                  │                  │                   │
│      │ SQL              │ HTTP             │ HTTP             │ HTTP (OpenAI API) │
│      ▼                  ▼                  ▼                  ▼                   │
│  ┌──────────┐   ┌──────────────┐   ┌──────────────┐   ┌──────────────────────┐  │
│  │PostgreSQL│   │   Baileys    │   │  OCR Service │   │        OVMS          │  │
│  │  16      │   │   :3100      │   │    :8000     │   │       :8000          │  │
│  │──────────│   │──────────────│   │──────────────│   │──────────────────────│  │
│  │• DB:     │   │• Node.js     │   │• Python      │   │• OpenVINO Model      │  │
│  │  realestate│ │• WhatsApp    │   │• PaddleOCR   │   │  Server              │  │
│  │• 10Gi PVC│   │  gateway     │   │• OpenVINO    │   │• Qwen3-30B-A3B      │  │
│  │• Secrets │   │• Webhook →   │   │  (Intel GPU) │   │  (int4, Intel Xe)    │  │
│  │  para    │   │  backend     │   │• Extracción  │   │• LLM inference       │  │
│  │  creds   │   │• Encrypted   │   │  de docs     │   │  para chatbot        │  │
│  │          │   │  sessions    │   │              │   │• 25Gi modelo PVC     │  │
│  └──────────┘   └──────┬───────┘   └──────────────┘   └──────────────────────┘  │
│       ▲                 │                                                        │
│       │ SQL (isolated)  │                                                        │
│       └─────────────────┘                                                        │
│       whatsapp_session_rw role                                                   │
│       (solo tablas whatsapp_auth_*)                                              │
│                                                                                  │
│  ┌────────────────────────────────────────────────────────────────────────────┐  │
│  │  STORAGE                                                                    │  │
│  │  • uploads-nfs-pvc (5Gi RWX) — documentos compartidos entre réplicas       │  │
│  │  • realestate-db-pvc (10Gi) — datos PostgreSQL                             │  │
│  │  • ovms-models-pvc (25Gi) — modelos LLM                                    │  │
│  └────────────────────────────────────────────────────────────────────────────┘  │
│                                                                                  │
│  ┌────────────────────────────────────────────────────────────────────────────┐  │
│  │  SECRETS (K8s Secrets)                                                      │  │
│  │  • realestate-db-secret — PostgreSQL credentials                            │  │
│  │  • realestate-db-url — DATABASE_URL                                         │  │
│  │  • realestate-app-secret — JWT_SECRET                                       │  │
│  │  • realestate-chatbot-secret — baileys-internal-token, session-encryption   │  │
│  │  • realestate-bcrd-secret — BCRD_API_TOKEN                                  │  │
│  │  • mailcow-smtp — SMTP credentials                                          │  │
│  └────────────────────────────────────────────────────────────────────────────┘  │
│                                                                                  │
└──────────────────────────────────────────────────────────────────────────────────┘


                    ┌─────────────────────────┐
                    │   F-Droid Repo          │
                    │   Host: fdroid.local    │
                    │   HTTP :80             │
                    │   APK distribution      │
                    └─────────────────────────┘
```

## Flujo de Comunicación

```
┌──────────┐         ┌──────────┐         ┌──────────┐
│ Cliente  │──HTTPS──▶│ Traefik  │──HTTPS──▶│ Frontend │
│ (Web/App)│         │ Ingress  │         │ (Caddy)  │
└──────────┘         └──────────┘         └────┬─────┘
                                                │ /api/*
                                                ▼
                                          ┌──────────┐
                                          │ Backend  │
                                          │ (Actix)  │
                                          └──┬─┬─┬─┬─┘
                                             │ │ │ │
                    ┌────────────────────────┘ │ │ └────────────────────────┐
                    │                          │ │                          │
                    ▼                          │ │                          ▼
              ┌──────────┐                    │ │                    ┌──────────┐
              │PostgreSQL│                    │ │                    │   OVMS   │
              │          │                    │ │                    │  (LLM)   │
              └──────────┘                    │ │                    └──────────┘
                                              │ │
                              ┌───────────────┘ └───────────────┐
                              │                                 │
                              ▼                                 ▼
                        ┌──────────┐                     ┌──────────┐
                        │ Baileys  │                     │   OCR    │
                        │(WhatsApp)│                     │ Service  │
                        └──────────┘                     └──────────┘
```

## Stack Tecnológico

| Capa | Tecnología |
|------|-----------|
| Frontend Web | Rust (Yew 0.21) → WASM, Tailwind CSS |
| Frontend Móvil | Kotlin, Jetpack Compose, Hilt, Room |
| Backend API | Rust, Actix-web 4, SeaORM |
| Base de Datos | PostgreSQL 16 |
| ORM / Migraciones | SeaORM + sea-orm-migration |
| Autenticación | JWT (jsonwebtoken) + Argon2 |
| WhatsApp | Baileys (Node.js sidecar) |
| OCR | PaddleOCR + OpenVINO (Python) |
| LLM / AI | OpenVINO Model Server + Qwen3-30B-A3B |
| Web Server | Caddy (TLS, proxy, static files) |
| Ingress | Traefik |
| Orquestación | Kubernetes (k3s) |
| CI/CD | GitHub Actions |
| Monitoreo | Prometheus + Grafana |
| Email | Mailcow SMTP |
| Linting | clippy (Rust), detekt + spotless (Kotlin), trunk (multi) |

## Módulos Android

```
android/
├── app/                          # Application module (Hilt entry point)
├── core/
│   ├── common/                   # Shared utilities, Result wrappers
│   ├── data/                     # Repositories implementation
│   ├── database/                 # Room database, DAOs
│   ├── model/                    # Pure Kotlin domain models
│   ├── network/                  # Retrofit API definitions
│   └── ui/                       # Shared Compose components, theme
└── feature/
    ├── auth/                     # Login, registro
    ├── dashboard/                # Vista principal con KPIs
    ├── propiedades/              # CRUD propiedades + unidades
    ├── inquilinos/               # CRUD inquilinos
    ├── contratos/                # Gestión de contratos
    ├── pagos/                    # Registro y seguimiento de pagos
    ├── gastos/                   # Control de gastos
    ├── mantenimiento/            # Solicitudes de mantenimiento
    ├── reportes/                 # Reportes financieros
    ├── documentos/               # Gestión documental
    ├── notificaciones/           # Centro de notificaciones
    ├── auditoria/                # Log de auditoría
    ├── perfil/                   # Perfil de usuario
    ├── configuracion/            # Ajustes del sistema
    ├── importacion/              # Importación masiva
    ├── scanner/                  # OCR scanner de documentos
    ├── usuarios/                 # Gestión de usuarios (admin)
    ├── chatbot/                  # Interfaz chatbot WhatsApp
    └── plantillas/               # Plantillas de documentos
```

## Capas del Backend

```
┌─────────────────────────────────────────────────────┐
│                    HTTP Layer                         │
│  routes.rs → Rate limits → Auth middleware → RBAC   │
└───────────────────────────┬─────────────────────────┘
                            │
┌───────────────────────────▼─────────────────────────┐
│                  Handlers (handlers/)                 │
│  Parse request → Validate input → Call service       │
│  → Format response (JSON, PDF, XLSX)                 │
└───────────────────────────┬─────────────────────────┘
                            │
┌───────────────────────────▼─────────────────────────┐
│                  Services (services/)                 │
│  Business logic → Transactions → Invariant checks   │
│  → External calls (OCR, Baileys, OVMS, SMTP)        │
└───────────────────────────┬─────────────────────────┘
                            │
┌───────────────────────────▼─────────────────────────┐
│              Entities (entities/) + Models            │
│  SeaORM generated entities │ DTOs (models/)          │
│  Migrations (migrations/)                            │
└───────────────────────────┬─────────────────────────┘
                            │
┌───────────────────────────▼─────────────────────────┐
│                    PostgreSQL                         │
│  UUIDs PKs │ DECIMAL money │ TIMESTAMPTZ dates       │
└─────────────────────────────────────────────────────┘
```

## Modelo de Datos (Entidades Principales)

```
┌──────────────┐       ┌──────────────┐       ┌──────────────┐
│   Usuario    │       │  Propiedad   │       │  Inquilino   │
│──────────────│       │──────────────│       │──────────────│
│ email (unique)│      │ titulo       │       │ nombre       │
│ rol          │       │ direccion    │       │ apellido     │
│ activo       │       │ precio+moneda│       │ cedula(unique)│
└──────┬───────┘       │ estado       │       └──────┬───────┘
       │               └──┬───┬───┬───┘              │
       │ audita            │   │   │                  │
       │                   │   │   │                  │
       │     ┌─────────────┘   │   └──────────┐      │
       │     │                 │              │      │
       │     ▼                 │              ▼      │
       │  ┌──────────┐        │     ┌──────────────┐│
       │  │  Unidad  │        │     │Mantenimiento ││
       │  │──────────│        │     │──────────────││
       │  │num_unidad│        │     │ estado       ││
       │  │precio    │        │     │ prioridad    ││
       │  └──────────┘        │     │ costo        ││
       │                      │     └──────────────┘│
       │                      │                     │
       │                      ▼                     │
       │              ┌──────────────┐              │
       │              │   Contrato   │◀─────────────┘
       │              │──────────────│
       │              │ fecha_inicio │
       │              │ fecha_fin    │
       │              │ monto_mensual│
       │              │ moneda       │
       │              │ estado       │
       │              └──────┬───────┘
       │                     │
       │                     ▼
       │              ┌──────────────┐
       │              │    Pago      │
       │              │──────────────│
       │              │ monto+moneda │
       │              │ fecha_venc   │
       │              │ fecha_pago   │
       │              │ estado       │
       │              └──────────────┘
       │
       │    ┌──────────────┐    ┌──────────────┐
       │    │    Gasto     │    │  Documento   │
       │    │──────────────│    │──────────────│
       └───▶│ propiedad_id │    │ entity_type  │
            │ categoria    │    │ entity_id    │
            │ monto+moneda │    │ filename     │
            │ estado       │    │ file_path    │
            └──────────────┘    └──────────────┘
```

## Roles y Permisos

| Rol | Escritura | Gestión Usuarios | Alcance |
|-----|-----------|-----------------|---------|
| `admin` | ✅ | ✅ | Acceso completo |
| `gerente` | ✅ | ❌ | Propiedades, inquilinos, contratos, pagos, gastos, mantenimiento |
| `visualizador` | ❌ | ❌ | Solo lectura |

## CI/CD Pipeline

```
GitHub Push → GitHub Actions
  ├── build-and-test.yml     → cargo clippy, cargo test, trunk build
  ├── security.yml           → CodeQL, Trivy, Semgrep
  ├── containers.yml         → Build & push Docker images (GHCR)
  ├── deploy.yml             → kubectl rollout to k3s cluster
  ├── android-fdroid.yml     → Build APK, publish to F-Droid repo
  └── scheduled-security-scans.yml → Weekly vulnerability scans
```
