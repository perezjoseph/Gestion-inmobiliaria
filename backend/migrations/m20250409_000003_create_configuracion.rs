use sea_orm_migration::prelude::*;

use super::m20250408_000001_create_usuarios::Usuarios;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m20250409_000003_create_configuracion"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Configuracion::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Configuracion::Clave)
                            .string_len(100)
                            .not_null()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(Configuracion::Valor)
                            .json_binary()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(Configuracion::UpdatedAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .col(ColumnDef::new(Configuracion::UpdatedBy).uuid().null())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_configuracion_updated_by")
                            .from(Configuracion::Table, Configuracion::UpdatedBy)
                            .to(Usuarios::Table, Usuarios::Id)
                            .on_delete(ForeignKeyAction::SetNull)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        let db = manager.get_connection();
        db.execute_unprepared(
            "INSERT INTO configuracion (clave, valor, updated_at) VALUES ('tasa_cambio_dop_usd', '{\"tasa\": 58.50, \"actualizado\": \"2025-01-01\"}', NOW())"
        ).await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Configuracion::Table).to_owned())
            .await
    }
}

#[derive(Iden)]
pub enum Configuracion {
    Table,
    Clave,
    Valor,
    UpdatedAt,
    UpdatedBy,
}
