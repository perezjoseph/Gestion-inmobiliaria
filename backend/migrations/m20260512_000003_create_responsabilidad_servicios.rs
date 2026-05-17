use sea_orm_migration::prelude::*;

use super::m20250408_000004_create_contratos::Contratos;
use super::m20250410_000001_create_unidades::Unidades;
use super::m20250413_000001_create_organizaciones::Organizaciones;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m20260512_000003_create_responsabilidad_servicios"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(ResponsabilidadServicios::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(ResponsabilidadServicios::Id)
                            .uuid()
                            .not_null()
                            .primary_key()
                            .extra("DEFAULT gen_random_uuid()"),
                    )
                    .col(
                        ColumnDef::new(ResponsabilidadServicios::UnidadId)
                            .uuid()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(ResponsabilidadServicios::ProveedorServicio)
                            .string_len(20)
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(ResponsabilidadServicios::Responsable)
                            .string_len(20)
                            .not_null(),
                    )
                    .col(ColumnDef::new(ResponsabilidadServicios::ContratoId).uuid())
                    .col(
                        ColumnDef::new(ResponsabilidadServicios::OrganizacionId)
                            .uuid()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(ResponsabilidadServicios::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .extra("DEFAULT now()"),
                    )
                    .col(
                        ColumnDef::new(ResponsabilidadServicios::UpdatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .extra("DEFAULT now()"),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_responsabilidad_unidad")
                            .from(
                                ResponsabilidadServicios::Table,
                                ResponsabilidadServicios::UnidadId,
                            )
                            .to(Unidades::Table, Unidades::Id)
                            .on_delete(ForeignKeyAction::Restrict)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_responsabilidad_contrato")
                            .from(
                                ResponsabilidadServicios::Table,
                                ResponsabilidadServicios::ContratoId,
                            )
                            .to(Contratos::Table, Contratos::Id)
                            .on_delete(ForeignKeyAction::SetNull)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_responsabilidad_organizacion")
                            .from(
                                ResponsabilidadServicios::Table,
                                ResponsabilidadServicios::OrganizacionId,
                            )
                            .to(Organizaciones::Table, Organizaciones::Id)
                            .on_delete(ForeignKeyAction::Restrict)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        // Regular index on unidad_id
        manager
            .create_index(
                Index::create()
                    .name("idx_responsabilidad_unidad_id")
                    .table(ResponsabilidadServicios::Table)
                    .col(ResponsabilidadServicios::UnidadId)
                    .to_owned(),
            )
            .await?;

        // Partial index on contrato_id (WHERE contrato_id IS NOT NULL) via raw SQL
        let db = manager.get_connection();
        db.execute_unprepared(
            "CREATE INDEX idx_responsabilidad_contrato_id ON responsabilidad_servicios (contrato_id) WHERE contrato_id IS NOT NULL",
        )
        .await?;

        // Unique constraint with COALESCE via raw SQL
        db.execute_unprepared(
            "CREATE UNIQUE INDEX uq_responsabilidad_unidad_proveedor_contrato ON responsabilidad_servicios (unidad_id, proveedor_servicio, COALESCE(contrato_id, '00000000-0000-0000-0000-000000000000'))",
        )
        .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(
                Table::drop()
                    .table(ResponsabilidadServicios::Table)
                    .to_owned(),
            )
            .await
    }
}

#[derive(Iden)]
pub enum ResponsabilidadServicios {
    Table,
    Id,
    UnidadId,
    ProveedorServicio,
    ContratoId,
    Responsable,
    OrganizacionId,
    CreatedAt,
    UpdatedAt,
}
