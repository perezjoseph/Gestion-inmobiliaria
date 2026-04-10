# Design: MVP — Real Estate Property Management

## Architecture Overview

```
┌─────────────────────┐     HTTP/JSON     ┌──────────────────────────────┐
│   Yew Frontend      │ ◄──────────────► │   Actix-web Backend          │
│   (WASM + Tailwind) │                   │                              │
│                     │                   │  Middleware (JWT + RBAC)     │
│   Pages:            │                   │  ├─ Handlers (HTTP layer)   │
│   - Login           │                   │  ├─ Services (business)     │
│   - Dashboard       │                   │  └─ Entities (SeaORM)      │
│   - Propiedades     │                   │                              │
│   - Inquilinos      │                   │         ▼                    │
│   - Contratos       │                   │  ┌──────────────┐           │
│   - Pagos           │                   │  │ PostgreSQL   │           │
└─────────────────────┘                   │  └──────────────┘           │
                                          └──────────────────────────────┘
```

## Database Schema

### usuarios
| Column | Type | Constraints |
|--------|------|-------------|
| id | UUID | PK, DEFAULT gen_random_uuid() |
| nombre | VARCHAR(100) | NOT NULL |
| email | VARCHAR(255) | NOT NULL, UNIQUE |
| password_hash | VARCHAR(255) | NOT NULL |
| rol | VARCHAR(20) | NOT NULL, CHECK (admin, gerente, visualizador) |
| activo | BOOLEAN | NOT NULL, DEFAULT true |
| created_at | TIMESTAMPTZ | NOT NULL, DEFAULT now() |
| updated_at | TIMESTAMPTZ | NOT NULL, DEFAULT now() |

Index: email

### propiedades
| Column | Type | Constraints |
|--------|------|-------------|
| id | UUID | PK |
| titulo | VARCHAR(200) | NOT NULL |
| descripcion | TEXT | |
| direccion | VARCHAR(300) | NOT NULL |
| ciudad | VARCHAR(100) | NOT NULL |
| provincia | VARCHAR(100) | NOT NULL |
| tipo_propiedad | VARCHAR(20) | NOT NULL (casa, apartamento, comercial, terreno) |
| habitaciones | INTEGER | |
| banos | INTEGER | |
| area_m2 | DECIMAL(10,2) | |
| precio | DECIMAL(12,2) | NOT NULL |
| moneda | VARCHAR(3) | NOT NULL, DEFAULT 'DOP' |
| estado | VARCHAR(20) | NOT NULL, DEFAULT 'disponible' |
| imagenes | JSONB | DEFAULT '[]' |
| created_at | TIMESTAMPTZ | NOT NULL |
| updated_at | TIMESTAMPTZ | NOT NULL |

Indexes: ciudad, provincia, tipo_propiedad, estado

### inquilinos
| Column | Type | Constraints |
|--------|------|-------------|
| id | UUID | PK |
| nombre | VARCHAR(100) | NOT NULL |
| apellido | VARCHAR(100) | NOT NULL |
| email | VARCHAR(255) | |
| telefono | VARCHAR(20) | |
| cedula | VARCHAR(20) | NOT NULL, UNIQUE |
| contacto_emergencia | VARCHAR(200) | |
| notas | TEXT | |
| created_at | TIMESTAMPTZ | NOT NULL |
| updated_at | TIMESTAMPTZ | NOT NULL |

Index: cedula

### contratos
| Column | Type | Constraints |
|--------|------|-------------|
| id | UUID | PK |
| propiedad_id | UUID | NOT NULL, FK → propiedades(id) |
| inquilino_id | UUID | NOT NULL, FK → inquilinos(id) |
| fecha_inicio | DATE | NOT NULL |
| fecha_fin | DATE | NOT NULL |
| monto_mensual | DECIMAL(12,2) | NOT NULL |
| deposito | DECIMAL(12,2) | |
| moneda | VARCHAR(3) | NOT NULL, DEFAULT 'DOP' |
| estado | VARCHAR(20) | NOT NULL, DEFAULT 'activo' |
| created_at | TIMESTAMPTZ | NOT NULL |
| updated_at | TIMESTAMPTZ | NOT NULL |

Indexes: propiedad_id, inquilino_id, estado

### pagos
| Column | Type | Constraints |
|--------|------|-------------|
| id | UUID | PK |
| contrato_id | UUID | NOT NULL, FK → contratos(id) |
| monto | DECIMAL(12,2) | NOT NULL |
| moneda | VARCHAR(3) | NOT NULL, DEFAULT 'DOP' |
| fecha_pago | DATE | |
| fecha_vencimiento | DATE | NOT NULL |
| metodo_pago | VARCHAR(20) | (efectivo, transferencia, cheque) |
| estado | VARCHAR(20) | NOT NULL, DEFAULT 'pendiente' |
| notas | TEXT | |
| created_at | TIMESTAMPTZ | NOT NULL |
| updated_at | TIMESTAMPTZ | NOT NULL |

Indexes: contrato_id, estado, fecha_vencimiento

## API Endpoints

### Auth
- `POST /api/auth/register` — Register new user
- `POST /api/auth/login` — Login, returns JWT

### Propiedades
- `GET /api/propiedades` — List (paginated, filterable)
- `GET /api/propiedades/{id}` — Get by ID
- `POST /api/propiedades` — Create (gerente, admin)
- `PUT /api/propiedades/{id}` — Update (gerente, admin)
- `DELETE /api/propiedades/{id}` — Delete (admin)

### Inquilinos
- `GET /api/inquilinos` — List (searchable)
- `GET /api/inquilinos/{id}` — Get by ID
- `POST /api/inquilinos` — Create (gerente, admin)
- `PUT /api/inquilinos/{id}` — Update (gerente, admin)
- `DELETE /api/inquilinos/{id}` — Delete (admin)

### Contratos
- `GET /api/contratos` — List
- `GET /api/contratos/{id}` — Get by ID
- `POST /api/contratos` — Create (gerente, admin)
- `PUT /api/contratos/{id}` — Update (gerente, admin)
- `DELETE /api/contratos/{id}` — Delete (admin)

### Pagos
- `GET /api/pagos` — List (filterable by contrato_id, estado)
- `GET /api/pagos/{id}` — Get by ID
- `POST /api/pagos` — Create (gerente, admin)
- `PUT /api/pagos/{id}` — Update (gerente, admin)
- `DELETE /api/pagos/{id}` — Delete (admin)

### Dashboard
- `GET /api/dashboard/stats` — Aggregate statistics

## Backend Layer Design

### Handler Pattern
```rust
pub async fn create(
    db: web::Data<DatabaseConnection>,
    claims: Claims,           // extracted by JWT middleware
    body: web::Json<CreateRequest>,
) -> Result<HttpResponse, AppError> {
    let result = service::create(&db, body.into_inner()).await?;
    Ok(HttpResponse::Created().json(result))
}
```

### Service Pattern
```rust
pub async fn create(
    db: &DatabaseConnection,
    input: CreateRequest,
) -> Result<ResponseDto, AppError> {
    // validate, build ActiveModel, insert, return DTO
}
```

### JWT Claims
```rust
struct Claims {
    sub: Uuid,        // user ID
    email: String,
    rol: String,
    exp: usize,
}
```

### RBAC Middleware
Extracts Claims from request extensions (set by JWT middleware) and checks `rol` against allowed roles for the route.

## Frontend Component Tree

```
App
├── Login (public)
└── AuthenticatedLayout (protected, has sidebar + navbar)
    ├── Dashboard
    ├── PropiedadesPage
    │   └── PropiedadForm (create/edit modal)
    ├── InquilinosPage
    │   └── InquilinoForm
    ├── ContratosPage
    │   └── ContratoForm (with property/tenant dropdowns)
    └── PagosPage
        └── PagoForm
```

### Auth Context
A Yew context provider wrapping the authenticated layout that holds the JWT token and user info, providing login/logout callbacks to children.

### API Service
Centralized in `frontend/src/services/api.rs`. All requests go through a helper that attaches the Bearer token and handles 401 redirects.
