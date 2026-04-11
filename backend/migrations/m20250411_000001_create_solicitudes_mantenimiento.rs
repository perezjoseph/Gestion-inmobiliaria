use sea_orm_migration::prelude::*;

use super::m20250408_000002_create_propiedades::Propiedades;
use super::m20250408_000003_create_inquilinos::Inquilinos;
use super::m20250410_000001_create_unidades::Unidades;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m20250411_000001_create_solicitudes_mantenimiento"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(SolicitudesMantenimiento::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(SolicitudesMantenimiento::Id)
                            .uuid()
                            .not_null()
                            .primary_key()
                            .extra("DEFAULT gen_random_uuid()"),
                    )
                    .col(
                        ColumnDef::new(SolicitudesMantenimiento::PropiedadId)
                            .uuid()
                            .not_null(),
                    )
                    .col(ColumnDef::new(SolicitudesMantenimiento::UnidadId).uuid())
                    .col(ColumnDef::new(SolicitudesMantenimiento::InquilinoId).uuid())
                    .col(
                        ColumnDef::new(SolicitudesMantenimiento::Titulo)
                            .string_len(255)
                            .not_null(),
                    )
                    .col(ColumnDef::new(SolicitudesMantenimiento::Descripcion).text())
                    .col(
                        ColumnDef::new(SolicitudesMantenimiento::Estado)
                            .string_len(20)
                            .not_null()
                            .default("pendiente"),
                    )
                    .col(
                        ColumnDef::new(SolicitudesMantenimiento::Prioridad)
                            .string_len(20)
                            .not_null()
                            .default("media"),
                    )
                    .col(ColumnDef::new(SolicitudesMantenimiento::NombreProveedor).string_len(255))
                    .col(ColumnDef::new(SolicitudesMantenimiento::TelefonoProveedor).string_len(50))
                    .col(ColumnDef::new(SolicitudesMantenimiento::EmailProveedor).string_len(255))
                    .col(ColumnDef::new(SolicitudesMantenimiento::CostoMonto).decimal_len(12, 2))
                    .col(ColumnDef::new(SolicitudesMantenimiento::CostoMoneda).string_len(3))
                    .col(
                        ColumnDef::new(SolicitudesMantenimiento::FechaInicio)
                            .timestamp_with_time_zone(),
                    )
                    .col(
                        ColumnDef::new(SolicitudesMantenimiento::FechaFin)
                            .timestamp_with_time_zone(),
                    )
                    .col(
                        ColumnDef::new(SolicitudesMantenimiento::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .extra("DEFAULT now()"),
                    )
                    .col(
                        ColumnDef::new(SolicitudesMantenimiento::UpdatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .extra("DEFAULT now()"),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_solicitudes_mant_propiedad")
                            .from(
                                SolicitudesMantenimiento::Table,
                                SolicitudesMantenimiento::PropiedadId,
                            )
                            .to(Propiedades::Table, Propiedades::Id)
                            .on_delete(ForeignKeyAction::Restrict)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_solicitudes_mant_unidad")
                            .from(
                                SolicitudesMantenimiento::Table,
                                SolicitudesMantenimiento::UnidadId,
                            )
                            .to(Unidades::Table, Unidades::Id)
                            .on_delete(ForeignKeyAction::Restrict)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_solicitudes_mant_inquilino")
                            .from(
                                SolicitudesMantenimiento::Table,
                                SolicitudesMantenimiento::InquilinoId,
                            )
                            .to(Inquilinos::Table, Inquilinos::Id)
                            .on_delete(ForeignKeyAction::Restrict)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_solicitudes_mant_propiedad_id")
                    .table(SolicitudesMantenimiento::Table)
                    .col(SolicitudesMantenimiento::PropiedadId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_solicitudes_mant_estado")
                    .table(SolicitudesMantenimiento::Table)
                    .col(SolicitudesMantenimiento::Estado)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_solicitudes_mant_prioridad")
                    .table(SolicitudesMantenimiento::Table)
                    .col(SolicitudesMantenimiento::Prioridad)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_solicitudes_mant_unidad_id")
                    .table(SolicitudesMantenimiento::Table)
                    .col(SolicitudesMantenimiento::UnidadId)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(
                Table::drop()
                    .table(SolicitudesMantenimiento::Table)
                    .to_owned(),
            )
            .await
    }
}

#[derive(Iden)]
pub enum SolicitudesMantenimiento {
    Table,
    Id,
    PropiedadId,
    UnidadId,
    InquilinoId,
    Titulo,
    Descripcion,
    Estado,
    Prioridad,
    NombreProveedor,
    TelefonoProveedor,
    EmailProveedor,
    CostoMonto,
    CostoMoneda,
    FechaInicio,
    FechaFin,
    CreatedAt,
    UpdatedAt,
}
