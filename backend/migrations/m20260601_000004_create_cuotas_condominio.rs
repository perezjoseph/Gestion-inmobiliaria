use sea_orm_migration::prelude::*;

use super::m20250408_000002_create_propiedades::Propiedades;
use super::m20250408_000004_create_contratos::Contratos;
use super::m20250413_000001_create_organizaciones::Organizaciones;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m20260601_000004_create_cuotas_condominio"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(CuotasCondominio::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(CuotasCondominio::Id)
                            .uuid()
                            .not_null()
                            .primary_key()
                            .extra("DEFAULT gen_random_uuid()"),
                    )
                    .col(
                        ColumnDef::new(CuotasCondominio::PropiedadId)
                            .uuid()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(CuotasCondominio::Monto)
                            .decimal_len(12, 2)
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(CuotasCondominio::Moneda)
                            .string_len(3)
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(CuotasCondominio::Frecuencia)
                            .string_len(20)
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(CuotasCondominio::FechaInicio)
                            .date()
                            .not_null(),
                    )
                    .col(ColumnDef::new(CuotasCondominio::FechaFin).date())
                    .col(
                        ColumnDef::new(CuotasCondominio::EsPassthrough)
                            .boolean()
                            .not_null()
                            .default(false),
                    )
                    .col(ColumnDef::new(CuotasCondominio::ContratoId).uuid())
                    .col(
                        ColumnDef::new(CuotasCondominio::OrganizacionId)
                            .uuid()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(CuotasCondominio::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .extra("DEFAULT now()"),
                    )
                    .col(
                        ColumnDef::new(CuotasCondominio::UpdatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .extra("DEFAULT now()"),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_cuotas_condominio_propiedad")
                            .from(CuotasCondominio::Table, CuotasCondominio::PropiedadId)
                            .to(Propiedades::Table, Propiedades::Id)
                            .on_delete(ForeignKeyAction::Restrict)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_cuotas_condominio_contrato")
                            .from(CuotasCondominio::Table, CuotasCondominio::ContratoId)
                            .to(Contratos::Table, Contratos::Id)
                            .on_delete(ForeignKeyAction::SetNull)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_cuotas_condominio_organizacion")
                            .from(CuotasCondominio::Table, CuotasCondominio::OrganizacionId)
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
                    .name("idx_cuotas_condominio_propiedad_id")
                    .table(CuotasCondominio::Table)
                    .col(CuotasCondominio::PropiedadId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_cuotas_condominio_organizacion_id")
                    .table(CuotasCondominio::Table)
                    .col(CuotasCondominio::OrganizacionId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_cuotas_condominio_contrato_id")
                    .table(CuotasCondominio::Table)
                    .col(CuotasCondominio::ContratoId)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(CuotasCondominio::Table).to_owned())
            .await
    }
}

#[derive(Iden)]
pub enum CuotasCondominio {
    Table,
    Id,
    PropiedadId,
    Monto,
    Moneda,
    Frecuencia,
    FechaInicio,
    FechaFin,
    EsPassthrough,
    ContratoId,
    OrganizacionId,
    CreatedAt,
    UpdatedAt,
}
