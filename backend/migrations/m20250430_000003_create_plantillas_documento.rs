use sea_orm_migration::prelude::*;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m20250430_000003_create_plantillas_documento"
    }
}

#[derive(Iden)]
pub enum PlantillasDocumento {
    Table,
    Id,
    Nombre,
    TipoDocumento,
    EntityType,
    Contenido,
    Activo,
    CreatedAt,
    UpdatedAt,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(PlantillasDocumento::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(PlantillasDocumento::Id)
                            .uuid()
                            .not_null()
                            .primary_key()
                            .extra("DEFAULT gen_random_uuid()"),
                    )
                    .col(
                        ColumnDef::new(PlantillasDocumento::Nombre)
                            .string_len(100)
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(PlantillasDocumento::TipoDocumento)
                            .string_len(50)
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(PlantillasDocumento::EntityType)
                            .string_len(50)
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(PlantillasDocumento::Contenido)
                            .json_binary()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(PlantillasDocumento::Activo)
                            .boolean()
                            .not_null()
                            .default(true),
                    )
                    .col(
                        ColumnDef::new(PlantillasDocumento::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .extra("DEFAULT now()"),
                    )
                    .col(
                        ColumnDef::new(PlantillasDocumento::UpdatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .extra("DEFAULT now()"),
                    )
                    .to_owned(),
            )
            .await?;

        // Create indexes
        manager
            .create_index(
                Index::create()
                    .name("idx_plantillas_entity_type")
                    .table(PlantillasDocumento::Table)
                    .col(PlantillasDocumento::EntityType)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_plantillas_tipo_documento")
                    .table(PlantillasDocumento::Table)
                    .col(PlantillasDocumento::TipoDocumento)
                    .to_owned(),
            )
            .await?;

        // Seed built-in templates
        let db = manager.get_connection();

        // 1. Contrato de Arrendamiento
        db.execute_unprepared(
            r#"INSERT INTO plantillas_documento (nombre, tipo_documento, entity_type, contenido) VALUES (
                'Contrato de Arrendamiento',
                'contrato_arrendamiento',
                'contrato',
                '{
                    "version": 1,
                    "blocks": [
                        {"type": "heading", "level": 1, "text": "CONTRATO DE ARRENDAMIENTO"},
                        {"type": "paragraph", "text": "Entre las partes: {{propiedad.propietario}} (en adelante EL ARRENDADOR) y {{inquilino.nombre}} {{inquilino.apellido}} (en adelante EL ARRENDATARIO), portador/a de la cédula de identidad No. {{inquilino.cedula}}."},
                        {"type": "heading", "level": 2, "text": "OBJETO DEL CONTRATO"},
                        {"type": "paragraph", "text": "EL ARRENDADOR da en alquiler al ARRENDATARIO el inmueble ubicado en {{propiedad.direccion}}, para uso exclusivo de vivienda."},
                        {"type": "heading", "level": 2, "text": "DURACIÓN"},
                        {"type": "paragraph", "text": "El presente contrato tendrá una duración desde {{contrato.fecha_inicio}} hasta {{contrato.fecha_fin}}."},
                        {"type": "heading", "level": 2, "text": "PRECIO Y FORMA DE PAGO"},
                        {"type": "paragraph", "text": "El canon de arrendamiento mensual es de {{contrato.moneda}} {{contrato.monto_mensual}}, pagadero dentro de los primeros cinco (5) días de cada mes."},
                        {"type": "heading", "level": 2, "text": "DEPÓSITO DE GARANTÍA"},
                        {"type": "paragraph", "text": "EL ARRENDATARIO entrega la suma de {{contrato.moneda}} {{contrato.deposito}} como depósito de garantía."},
                        {"type": "heading", "level": 2, "text": "FIRMAS"},
                        {"type": "paragraph", "text": "EL ARRENDADOR: ________________________     EL ARRENDATARIO: ________________________"},
                        {"type": "paragraph", "text": "Fecha: {{contrato.fecha_inicio}}     Lugar: {{propiedad.ciudad}}"}
                    ]
                }'::jsonb
            )"#,
        )
        .await?;

        // 2. Recibo de Pago
        db.execute_unprepared(
            r#"INSERT INTO plantillas_documento (nombre, tipo_documento, entity_type, contenido) VALUES (
                'Recibo de Pago',
                'recibo_pago',
                'pago',
                '{
                    "version": 1,
                    "blocks": [
                        {"type": "heading", "level": 1, "text": "RECIBO DE PAGO"},
                        {"type": "paragraph", "text": "Recibí de: {{inquilino.nombre}} {{inquilino.apellido}}"},
                        {"type": "paragraph", "text": "Cédula: {{inquilino.cedula}}"},
                        {"type": "paragraph", "text": "La suma de: {{pago.moneda}} {{pago.monto}}"},
                        {"type": "paragraph", "text": "Concepto: Pago de alquiler correspondiente al período {{pago.fecha_vencimiento}}"},
                        {"type": "paragraph", "text": "Propiedad: {{propiedad.direccion}}"},
                        {"type": "paragraph", "text": "Método de pago: {{pago.metodo_pago}}"},
                        {"type": "paragraph", "text": "Fecha de pago: {{pago.fecha_pago}}"},
                        {"type": "paragraph", "text": "Recibido por: ________________________"},
                        {"type": "paragraph", "text": "Firma: ________________________"}
                    ]
                }'::jsonb
            )"#,
        )
        .await?;

        // 3. Acta Notarial
        db.execute_unprepared(
            r#"INSERT INTO plantillas_documento (nombre, tipo_documento, entity_type, contenido) VALUES (
                'Acta Notarial',
                'acta_notarial',
                'contrato',
                '{
                    "version": 1,
                    "blocks": [
                        {"type": "heading", "level": 1, "text": "ACTA NOTARIAL"},
                        {"type": "paragraph", "text": "En la ciudad de {{propiedad.ciudad}}, República Dominicana, a los ______ días del mes de ______ del año ______."},
                        {"type": "paragraph", "text": "Ante mí, ______________________________, Notario Público de los del número del Distrito Nacional, comparecen:"},
                        {"type": "paragraph", "text": "PRIMERA PARTE: {{propiedad.propietario}}, dominicano/a, mayor de edad."},
                        {"type": "paragraph", "text": "SEGUNDA PARTE: {{inquilino.nombre}} {{inquilino.apellido}}, portador/a de la cédula No. {{inquilino.cedula}}."},
                        {"type": "paragraph", "text": "Quienes declaran haber convenido el contrato de arrendamiento del inmueble ubicado en {{propiedad.direccion}}, por un monto mensual de {{contrato.moneda}} {{contrato.monto_mensual}}."},
                        {"type": "paragraph", "text": "Leída la presente acta, los comparecientes la encuentran conforme y firman."},
                        {"type": "paragraph", "text": "PRIMERA PARTE: ________________________"},
                        {"type": "paragraph", "text": "SEGUNDA PARTE: ________________________"},
                        {"type": "paragraph", "text": "NOTARIO PÚBLICO: ________________________"}
                    ]
                }'::jsonb
            )"#,
        )
        .await?;

        // 4. Carta de Referencia
        db.execute_unprepared(
            r#"INSERT INTO plantillas_documento (nombre, tipo_documento, entity_type, contenido) VALUES (
                'Carta de Referencia',
                'carta_referencia',
                'inquilino',
                '{
                    "version": 1,
                    "blocks": [
                        {"type": "heading", "level": 1, "text": "CARTA DE REFERENCIA"},
                        {"type": "paragraph", "text": "A quien pueda interesar:"},
                        {"type": "paragraph", "text": "Por medio de la presente, certifico que el/la Sr./Sra. {{inquilino.nombre}} {{inquilino.apellido}}, portador/a de la cédula de identidad No. {{inquilino.cedula}}, ha sido inquilino/a del inmueble ubicado en {{propiedad.direccion}}."},
                        {"type": "paragraph", "text": "Durante el período de arrendamiento, desde {{contrato.fecha_inicio}} hasta {{contrato.fecha_fin}}, el/la mencionado/a inquilino/a cumplió con sus obligaciones de pago de manera puntual y mantuvo el inmueble en buen estado."},
                        {"type": "paragraph", "text": "Se expide la presente carta de referencia a solicitud del interesado/a, para los fines que estime conveniente."},
                        {"type": "paragraph", "text": "Atentamente,"},
                        {"type": "paragraph", "text": "________________________"},
                        {"type": "paragraph", "text": "Nombre: {{propiedad.propietario}}"},
                        {"type": "paragraph", "text": "Fecha: ________________________"}
                    ]
                }'::jsonb
            )"#,
        )
        .await?;

        // 5. Addendum
        db.execute_unprepared(
            r#"INSERT INTO plantillas_documento (nombre, tipo_documento, entity_type, contenido) VALUES (
                'Addendum al Contrato',
                'addendum',
                'contrato',
                '{
                    "version": 1,
                    "blocks": [
                        {"type": "heading", "level": 1, "text": "ADDENDUM AL CONTRATO DE ARRENDAMIENTO"},
                        {"type": "paragraph", "text": "El presente addendum modifica el contrato de arrendamiento celebrado entre {{propiedad.propietario}} (EL ARRENDADOR) y {{inquilino.nombre}} {{inquilino.apellido}} (EL ARRENDATARIO), cédula No. {{inquilino.cedula}}."},
                        {"type": "paragraph", "text": "Propiedad: {{propiedad.direccion}}"},
                        {"type": "paragraph", "text": "Contrato original vigente desde {{contrato.fecha_inicio}} hasta {{contrato.fecha_fin}}."},
                        {"type": "heading", "level": 2, "text": "MODIFICACIONES"},
                        {"type": "list", "ordered": true, "items": ["Primera modificación: ________________________", "Segunda modificación: ________________________"]},
                        {"type": "paragraph", "text": "Las demás cláusulas del contrato original permanecen vigentes y sin modificación."},
                        {"type": "heading", "level": 2, "text": "FIRMAS"},
                        {"type": "paragraph", "text": "EL ARRENDADOR: ________________________     EL ARRENDATARIO: ________________________"},
                        {"type": "paragraph", "text": "Fecha: ________________________"}
                    ]
                }'::jsonb
            )"#,
        )
        .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(PlantillasDocumento::Table).to_owned())
            .await
    }
}
