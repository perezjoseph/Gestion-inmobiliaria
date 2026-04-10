use sea_orm_migration::prelude::*;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m20250601_000001_create_ejecuciones_tareas"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(EjecucionesTareas::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(EjecucionesTareas::Id)
                            .uuid()
                            .not_null()
                            .primary_key()
                            .extra("DEFAULT gen_random_uuid()"),
                    )
                    .col(
                        ColumnDef::new(EjecucionesTareas::NombreTarea)
                            .string_len(100)
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(EjecucionesTareas::IniciadoEn)
                            .timestamp_with_time_zone()
                            .not_null()
                            .extra("DEFAULT now()"),
                    )
                    .col(
                        ColumnDef::new(EjecucionesTareas::DuracionMs)
                            .big_integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(EjecucionesTareas::Exitosa)
                            .boolean()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(EjecucionesTareas::RegistrosAfectados)
                            .big_integer()
                            .not_null()
                            .default(0),
                    )
                    .col(ColumnDef::new(EjecucionesTareas::MensajeError).text())
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_ejecuciones_tareas_nombre")
                    .table(EjecucionesTareas::Table)
                    .col(EjecucionesTareas::NombreTarea)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_ejecuciones_tareas_iniciado_en")
                    .table(EjecucionesTareas::Table)
                    .col(EjecucionesTareas::IniciadoEn)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_ejecuciones_tareas_nombre_iniciado")
                    .table(EjecucionesTareas::Table)
                    .col(EjecucionesTareas::NombreTarea)
                    .col(EjecucionesTareas::IniciadoEn)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(EjecucionesTareas::Table).to_owned())
            .await
    }
}

#[derive(Iden)]
pub enum EjecucionesTareas {
    Table,
    Id,
    NombreTarea,
    IniciadoEn,
    DuracionMs,
    Exitosa,
    RegistrosAfectados,
    MensajeError,
}
