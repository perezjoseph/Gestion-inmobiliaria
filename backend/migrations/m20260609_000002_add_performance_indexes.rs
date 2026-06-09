use sea_orm_migration::prelude::*;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m20260609_000002_add_performance_indexes"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Composite index on pagos(organizacion_id, estado) — dashboard and list queries
        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_pagos_org_estado")
                    .table(Pagos::Table)
                    .col(Pagos::OrganizacionId)
                    .col(Pagos::Estado)
                    .to_owned(),
            )
            .await?;

        // Index on pagos(fecha_vencimiento) — range queries in dashboard and pagos_proximos
        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_pagos_fecha_vencimiento")
                    .table(Pagos::Table)
                    .col(Pagos::FechaVencimiento)
                    .to_owned(),
            )
            .await?;

        // Composite index on contratos(organizacion_id, estado) — dashboard and list queries
        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_contratos_org_estado")
                    .table(Contratos::Table)
                    .col(Contratos::OrganizacionId)
                    .col(Contratos::Estado)
                    .to_owned(),
            )
            .await?;

        // Composite index on documentos(entity_type, entity_id) — compliance queries
        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_documentos_entity_type_id")
                    .table(Documentos::Table)
                    .col(Documentos::EntityType)
                    .col(Documentos::EntityId)
                    .to_owned(),
            )
            .await?;

        // Composite index on gastos(organizacion_id, estado, fecha_gasto) — dashboard sum queries
        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_gastos_org_estado_fecha")
                    .table(Gastos::Table)
                    .col(Gastos::OrganizacionId)
                    .col(Gastos::Estado)
                    .col(Gastos::FechaGasto)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_index(
                Index::drop()
                    .name("idx_pagos_org_estado")
                    .table(Pagos::Table)
                    .to_owned(),
            )
            .await?;

        manager
            .drop_index(
                Index::drop()
                    .name("idx_pagos_fecha_vencimiento")
                    .table(Pagos::Table)
                    .to_owned(),
            )
            .await?;

        manager
            .drop_index(
                Index::drop()
                    .name("idx_contratos_org_estado")
                    .table(Contratos::Table)
                    .to_owned(),
            )
            .await?;

        manager
            .drop_index(
                Index::drop()
                    .name("idx_documentos_entity_type_id")
                    .table(Documentos::Table)
                    .to_owned(),
            )
            .await?;

        manager
            .drop_index(
                Index::drop()
                    .name("idx_gastos_org_estado_fecha")
                    .table(Gastos::Table)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }
}

#[derive(DeriveIden)]
enum Pagos {
    Table,
    OrganizacionId,
    Estado,
    FechaVencimiento,
}

#[derive(DeriveIden)]
enum Contratos {
    Table,
    OrganizacionId,
    Estado,
}

#[derive(DeriveIden)]
enum Documentos {
    Table,
    EntityType,
    EntityId,
}

#[derive(DeriveIden)]
enum Gastos {
    Table,
    OrganizacionId,
    Estado,
    FechaGasto,
}
