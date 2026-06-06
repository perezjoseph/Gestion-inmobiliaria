use sea_orm_migration::prelude::*;

use super::m20250408_000002_create_propiedades::Propiedades;
use super::m20250413_000001_create_organizaciones::Organizaciones;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Copropietarios::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Copropietarios::Id)
                            .uuid()
                            .not_null()
                            .primary_key()
                            .extra("DEFAULT gen_random_uuid()"),
                    )
                    .col(
                        ColumnDef::new(Copropietarios::PropiedadId)
                            .uuid()
                            .not_null(),
                    )
                    .col(ColumnDef::new(Copropietarios::Nombre).string().not_null())
                    .col(
                        ColumnDef::new(Copropietarios::CedulaRnc)
                            .string()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(Copropietarios::PorcentajePropiedad)
                            .decimal_len(5, 2)
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(Copropietarios::OrganizacionId)
                            .uuid()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(Copropietarios::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .extra("DEFAULT now()"),
                    )
                    .col(
                        ColumnDef::new(Copropietarios::UpdatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .extra("DEFAULT now()"),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_copropietarios_propiedad")
                            .from(Copropietarios::Table, Copropietarios::PropiedadId)
                            .to(Propiedades::Table, Propiedades::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_copropietarios_organizacion")
                            .from(Copropietarios::Table, Copropietarios::OrganizacionId)
                            .to(Organizaciones::Table, Organizaciones::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        // Index on propiedad_id for FK lookups
        manager
            .create_index(
                Index::create()
                    .name("idx_copropietarios_propiedad_id")
                    .table(Copropietarios::Table)
                    .col(Copropietarios::PropiedadId)
                    .to_owned(),
            )
            .await?;

        // Index on organizacion_id for FK lookups
        manager
            .create_index(
                Index::create()
                    .name("idx_copropietarios_organizacion_id")
                    .table(Copropietarios::Table)
                    .col(Copropietarios::OrganizacionId)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Copropietarios::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
pub enum Copropietarios {
    Table,
    Id,
    PropiedadId,
    Nombre,
    CedulaRnc,
    PorcentajePropiedad,
    OrganizacionId,
    CreatedAt,
    UpdatedAt,
}
