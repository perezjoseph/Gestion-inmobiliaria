use sea_orm_migration::prelude::*;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m20260615_000001_add_organizacion_id_to_configuracion"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();

        db.execute_unprepared("ALTER TABLE configuracion ADD COLUMN organizacion_id UUID")
            .await?;

        db.execute_unprepared(
            "INSERT INTO configuracion (clave, valor, updated_at, updated_by, organizacion_id)
             SELECT c.clave, c.valor, c.updated_at, c.updated_by, o.id
             FROM configuracion c
             CROSS JOIN organizaciones o
             WHERE c.organizacion_id IS NULL",
        )
        .await?;

        db.execute_unprepared("DELETE FROM configuracion WHERE organizacion_id IS NULL")
            .await?;

        db.execute_unprepared(
            "ALTER TABLE configuracion ALTER COLUMN organizacion_id SET NOT NULL",
        )
        .await?;

        db.execute_unprepared("ALTER TABLE configuracion DROP CONSTRAINT configuracion_pkey")
            .await?;

        db.execute_unprepared("ALTER TABLE configuracion ADD PRIMARY KEY (clave, organizacion_id)")
            .await?;

        db.execute_unprepared(
            "ALTER TABLE configuracion ADD CONSTRAINT fk_configuracion_organizacion
             FOREIGN KEY (organizacion_id) REFERENCES organizaciones(id)
             ON DELETE CASCADE ON UPDATE CASCADE",
        )
        .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();

        db.execute_unprepared(
            "ALTER TABLE configuracion DROP CONSTRAINT fk_configuracion_organizacion",
        )
        .await?;

        db.execute_unprepared("ALTER TABLE configuracion DROP CONSTRAINT configuracion_pkey")
            .await?;

        db.execute_unprepared(
            "DELETE FROM configuracion c1
             USING configuracion c2
             WHERE c1.clave = c2.clave
               AND c1.organizacion_id > c2.organizacion_id",
        )
        .await?;

        db.execute_unprepared("ALTER TABLE configuracion ADD PRIMARY KEY (clave)")
            .await?;

        db.execute_unprepared("ALTER TABLE configuracion DROP COLUMN organizacion_id")
            .await?;

        Ok(())
    }
}
