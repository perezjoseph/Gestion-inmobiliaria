pub mod m20250408_000001_create_usuarios;
pub mod m20250408_000002_create_propiedades;
pub mod m20250408_000003_create_inquilinos;
pub mod m20250408_000004_create_contratos;
pub mod m20250408_000005_create_pagos;

use sea_orm_migration::prelude::*;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20250408_000001_create_usuarios::Migration),
            Box::new(m20250408_000002_create_propiedades::Migration),
            Box::new(m20250408_000003_create_inquilinos::Migration),
            Box::new(m20250408_000004_create_contratos::Migration),
            Box::new(m20250408_000005_create_pagos::Migration),
        ]
    }
}
