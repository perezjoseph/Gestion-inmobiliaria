use sea_orm_migration::prelude::*;
use uuid::Uuid;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m20250413_000002_add_organizacion_id"
    }
}

/// Tables that need the `organizacion_id` column.
const TABLES: &[&str] = &[
    "usuarios",
    "propiedades",
    "inquilinos",
    "contratos",
    "pagos",
    "gastos",
    "solicitudes_mantenimiento",
];

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();

        // 1. Add nullable organizacion_id UUID column to all 7 tables
        for table in TABLES {
            db.execute_unprepared(&format!(
                "ALTER TABLE {table} ADD COLUMN organizacion_id UUID"
            ))
            .await?;
        }

        // 2. Insert a default organization
        let default_org_id = Uuid::new_v4();
        db.execute_unprepared(&format!(
            "INSERT INTO organizaciones (id, tipo, nombre, estado, created_at, updated_at) \
             VALUES ('{default_org_id}', 'persona_fisica', 'Organización Predeterminada', 'activo', now(), now())"
        ))
        .await?;

        // 3. Update all existing rows in all 7 tables to reference the default org
        for table in TABLES {
            db.execute_unprepared(&format!(
                "UPDATE {table} SET organizacion_id = '{default_org_id}'"
            ))
            .await?;
        }

        // 4. Promote the first user (by created_at) to admin role
        db.execute_unprepared(
            "UPDATE usuarios SET rol = 'admin' \
             WHERE id = (SELECT id FROM usuarios ORDER BY created_at ASC LIMIT 1)",
        )
        .await?;

        // 5. Alter all organizacion_id columns to NOT NULL
        for table in TABLES {
            db.execute_unprepared(&format!(
                "ALTER TABLE {table} ALTER COLUMN organizacion_id SET NOT NULL"
            ))
            .await?;
        }

        // 6. Add FK constraints to organizaciones(id) for each column
        for table in TABLES {
            db.execute_unprepared(&format!(
                "ALTER TABLE {table} ADD CONSTRAINT fk_{table}_organizacion \
                 FOREIGN KEY (organizacion_id) REFERENCES organizaciones(id)"
            ))
            .await?;
        }

        // 7. Create indexes on organizacion_id for all 7 tables
        for table in TABLES {
            db.execute_unprepared(&format!(
                "CREATE INDEX idx_{table}_organizacion_id ON {table} (organizacion_id)"
            ))
            .await?;
        }

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();

        // Reverse order: drop indexes, drop FKs, drop columns
        for table in TABLES {
            db.execute_unprepared(&format!(
                "DROP INDEX IF EXISTS idx_{table}_organizacion_id"
            ))
            .await?;
        }

        for table in TABLES {
            db.execute_unprepared(&format!(
                "ALTER TABLE {table} DROP CONSTRAINT IF EXISTS fk_{table}_organizacion"
            ))
            .await?;
        }

        for table in TABLES {
            db.execute_unprepared(&format!(
                "ALTER TABLE {table} DROP COLUMN IF EXISTS organizacion_id"
            ))
            .await?;
        }

        // Remove the default organization
        db.execute_unprepared(
            "DELETE FROM organizaciones WHERE nombre = 'Organización Predeterminada'",
        )
        .await?;

        Ok(())
    }
}
