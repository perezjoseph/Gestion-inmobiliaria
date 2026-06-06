use sea_orm_migration::prelude::*;

use super::m20250408_000001_create_usuarios::Usuarios;
use super::m20250413_000001_create_organizaciones::Organizaciones;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(ReportesDgii::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(ReportesDgii::Id)
                            .uuid()
                            .not_null()
                            .primary_key()
                            .extra("DEFAULT gen_random_uuid()"),
                    )
                    .col(
                        ColumnDef::new(ReportesDgii::OrganizacionId)
                            .uuid()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(ReportesDgii::TipoReporte)
                            .string()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(ReportesDgii::Periodo)
                            .string_len(6)
                            .not_null(),
                    )
                    .col(ColumnDef::new(ReportesDgii::Estado).string().not_null())
                    .col(
                        ColumnDef::new(ReportesDgii::CantidadRegistros)
                            .integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(ReportesDgii::MontoTotal)
                            .decimal_len(14, 2)
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(ReportesDgii::ItbisTotal)
                            .decimal_len(14, 2)
                            .not_null(),
                    )
                    .col(ColumnDef::new(ReportesDgii::Contenido).text().not_null())
                    .col(ColumnDef::new(ReportesDgii::RegistrosExcluidos).json_binary())
                    .col(ColumnDef::new(ReportesDgii::GeneratedBy).uuid().not_null())
                    .col(
                        ColumnDef::new(ReportesDgii::GeneratedAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .col(ColumnDef::new(ReportesDgii::SubmittedAt).timestamp_with_time_zone())
                    .col(
                        ColumnDef::new(ReportesDgii::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .extra("DEFAULT now()"),
                    )
                    .col(
                        ColumnDef::new(ReportesDgii::UpdatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .extra("DEFAULT now()"),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_reportes_dgii_organizacion")
                            .from(ReportesDgii::Table, ReportesDgii::OrganizacionId)
                            .to(Organizaciones::Table, Organizaciones::Id)
                            .on_delete(ForeignKeyAction::Restrict)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_reportes_dgii_generated_by")
                            .from(ReportesDgii::Table, ReportesDgii::GeneratedBy)
                            .to(Usuarios::Table, Usuarios::Id)
                            .on_delete(ForeignKeyAction::Restrict)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        // Unique constraint: (organizacion_id, tipo_reporte, periodo, estado)
        manager
            .create_index(
                Index::create()
                    .name("idx_reportes_dgii_org_tipo_periodo_estado")
                    .table(ReportesDgii::Table)
                    .col(ReportesDgii::OrganizacionId)
                    .col(ReportesDgii::TipoReporte)
                    .col(ReportesDgii::Periodo)
                    .col(ReportesDgii::Estado)
                    .unique()
                    .to_owned(),
            )
            .await?;

        // Index on organizacion_id for FK lookups
        manager
            .create_index(
                Index::create()
                    .name("idx_reportes_dgii_organizacion_id")
                    .table(ReportesDgii::Table)
                    .col(ReportesDgii::OrganizacionId)
                    .to_owned(),
            )
            .await?;

        // Index on generated_by for FK lookups
        manager
            .create_index(
                Index::create()
                    .name("idx_reportes_dgii_generated_by")
                    .table(ReportesDgii::Table)
                    .col(ReportesDgii::GeneratedBy)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(ReportesDgii::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
pub enum ReportesDgii {
    Table,
    Id,
    OrganizacionId,
    TipoReporte,
    Periodo,
    Estado,
    CantidadRegistros,
    MontoTotal,
    ItbisTotal,
    Contenido,
    RegistrosExcluidos,
    GeneratedBy,
    GeneratedAt,
    SubmittedAt,
    CreatedAt,
    UpdatedAt,
}
