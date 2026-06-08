use sea_orm_migration::prelude::*;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m20260609_000001_add_organizacion_id_to_plantillas"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();

        // 1. Add nullable organizacion_id UUID column
        db.execute_unprepared("ALTER TABLE plantillas_documento ADD COLUMN organizacion_id UUID")
            .await?;

        // 2. Backfill existing templates to the first existing org
        db.execute_unprepared(
            "UPDATE plantillas_documento SET organizacion_id = (SELECT id FROM organizaciones ORDER BY created_at ASC LIMIT 1)",
        )
        .await?;

        // 3. Set column to NOT NULL after backfill
        db.execute_unprepared(
            "ALTER TABLE plantillas_documento ALTER COLUMN organizacion_id SET NOT NULL",
        )
        .await?;

        // 4. Add FK constraint to organizacion(id)
        db.execute_unprepared(
            "ALTER TABLE plantillas_documento ADD CONSTRAINT fk_plantillas_documento_organizacion \
             FOREIGN KEY (organizacion_id) REFERENCES organizaciones(id)",
        )
        .await?;

        // 5. Create index on organizacion_id for query performance
        db.execute_unprepared(
            "CREATE INDEX idx_plantillas_documento_organizacion_id ON plantillas_documento (organizacion_id)",
        )
        .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();

        db.execute_unprepared("DROP INDEX IF EXISTS idx_plantillas_documento_organizacion_id")
            .await?;

        db.execute_unprepared(
            "ALTER TABLE plantillas_documento DROP CONSTRAINT IF EXISTS fk_plantillas_documento_organizacion",
        )
        .await?;

        db.execute_unprepared(
            "ALTER TABLE plantillas_documento DROP COLUMN IF EXISTS organizacion_id",
        )
        .await?;

        Ok(())
    }
}
