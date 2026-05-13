use sea_orm_migration::prelude::*;

use super::m20250408_000004_create_contratos::Contratos;
use super::m20250413_000001_create_organizaciones::Organizaciones;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m20260512_000002_create_desahucios"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Desahucios::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Desahucios::Id)
                            .uuid()
                            .not_null()
                            .primary_key()
                            .extra("DEFAULT gen_random_uuid()"),
                    )
                    .col(ColumnDef::new(Desahucios::ContratoId).uuid().not_null())
                    .col(ColumnDef::new(Desahucios::Estado).string_len(20).not_null())
                    .col(ColumnDef::new(Desahucios::FechaInicio).date().not_null())
                    .col(ColumnDef::new(Desahucios::FechaResolucion).date())
                    .col(ColumnDef::new(Desahucios::Motivo).text().not_null())
                    .col(ColumnDef::new(Desahucios::OrganizacionId).uuid().not_null())
                    .col(
                        ColumnDef::new(Desahucios::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .extra("DEFAULT now()"),
                    )
                    .col(
                        ColumnDef::new(Desahucios::UpdatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .extra("DEFAULT now()"),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_desahucios_contrato")
                            .from(Desahucios::Table, Desahucios::ContratoId)
                            .to(Contratos::Table, Contratos::Id)
                            .on_delete(ForeignKeyAction::Restrict)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_desahucios_organizacion")
                            .from(Desahucios::Table, Desahucios::OrganizacionId)
                            .to(Organizaciones::Table, Organizaciones::Id)
                            .on_delete(ForeignKeyAction::Restrict)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_desahucios_contrato_id")
                    .table(Desahucios::Table)
                    .col(Desahucios::ContratoId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_desahucios_organizacion_id")
                    .table(Desahucios::Table)
                    .col(Desahucios::OrganizacionId)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Desahucios::Table).to_owned())
            .await
    }
}

#[derive(Iden)]
pub enum Desahucios {
    Table,
    Id,
    ContratoId,
    Estado,
    FechaInicio,
    FechaResolucion,
    Motivo,
    OrganizacionId,
    CreatedAt,
    UpdatedAt,
}
