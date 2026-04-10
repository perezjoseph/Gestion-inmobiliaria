use sea_orm_migration::prelude::*;

use super::m20250408_000001_create_usuarios::Usuarios;
use super::m20250411_000001_create_solicitudes_mantenimiento::SolicitudesMantenimiento;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m20250411_000002_create_notas_mantenimiento"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(NotasMantenimiento::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(NotasMantenimiento::Id)
                            .uuid()
                            .not_null()
                            .primary_key()
                            .extra("DEFAULT gen_random_uuid()"),
                    )
                    .col(
                        ColumnDef::new(NotasMantenimiento::SolicitudId)
                            .uuid()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(NotasMantenimiento::AutorId)
                            .uuid()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(NotasMantenimiento::Contenido)
                            .text()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(NotasMantenimiento::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .extra("DEFAULT now()"),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_notas_mant_solicitud")
                            .from(NotasMantenimiento::Table, NotasMantenimiento::SolicitudId)
                            .to(
                                SolicitudesMantenimiento::Table,
                                SolicitudesMantenimiento::Id,
                            )
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_notas_mant_autor")
                            .from(NotasMantenimiento::Table, NotasMantenimiento::AutorId)
                            .to(Usuarios::Table, Usuarios::Id)
                            .on_delete(ForeignKeyAction::Restrict)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_notas_mant_solicitud_id")
                    .table(NotasMantenimiento::Table)
                    .col(NotasMantenimiento::SolicitudId)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(NotasMantenimiento::Table).to_owned())
            .await
    }
}

#[derive(Iden)]
pub enum NotasMantenimiento {
    Table,
    Id,
    SolicitudId,
    AutorId,
    Contenido,
    CreatedAt,
}
