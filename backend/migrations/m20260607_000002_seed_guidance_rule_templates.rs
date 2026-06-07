use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

/// Builds a SQL expression for a single guidance rule JSON object.
fn rule_json_expr(category: &str, instruction: &str, sort_order: i32) -> String {
    // Escape single quotes in instruction text for SQL
    let escaped = instruction.replace('\'', "''");
    format!(
        "jsonb_build_object(\
            'id', gen_random_uuid()::text, \
            'category', '{category}', \
            'instruction', '{escaped}', \
            'enabled', true, \
            'isTemplate', true, \
            'sortOrder', {sort_order}, \
            'createdAt', now()::text, \
            'updatedAt', now()::text\
        )"
    )
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();

        // Define all 16 template rules
        let templates: &[(&str, &str)] = &[
            // estilo_comunicacion (sort_order 0-3)
            (
                "estilo_comunicacion",
                "Tratar a todos los inquilinos de 'usted', nunca de 'tú'",
            ),
            (
                "estilo_comunicacion",
                "Incluir siempre el símbolo de moneda (RD$ o US$) al mencionar montos",
            ),
            (
                "estilo_comunicacion",
                "Mantener mensajes cortos: máximo 3 oraciones por respuesta",
            ),
            (
                "estilo_comunicacion",
                "Responder siempre en español, sin importar el idioma del mensaje recibido",
            ),
            // contexto_clarificacion (sort_order 4-6)
            (
                "contexto_clarificacion",
                "Antes de compartir cualquier dato financiero, confirmar la identidad del inquilino pidiendo nombre y número de unidad",
            ),
            (
                "contexto_clarificacion",
                "Si el inquilino pregunta por un balance sin especificar unidad, preguntar cuál unidad antes de responder",
            ),
            (
                "contexto_clarificacion",
                "Si hay ambigüedad sobre cuál contrato se refiere, listar los contratos activos y pedir que elija",
            ),
            // escalamiento (sort_order 7-10)
            (
                "escalamiento",
                "Si el inquilino menciona 'abogado', 'tribunal', 'demanda' o 'acción legal', transferir inmediatamente a un humano sin hacer más preguntas",
            ),
            (
                "escalamiento",
                "Si el inquilino reporta una emergencia (inundación, fuga de gas, incendio, fallo eléctrico), transferir a humano inmediatamente",
            ),
            (
                "escalamiento",
                "Si el inquilino pide hablar con una persona real o dice 'humano', 'agente' o 'hablar con alguien', respetar su solicitud y transferir",
            ),
            (
                "escalamiento",
                "Si el inquilino repite la misma pregunta 3 veces sin obtener la respuesta deseada, ofrecer transferencia a un humano",
            ),
            // politicas (sort_order 11-15)
            (
                "politicas",
                "Nunca compartir datos bancarios del propietario o la administración",
            ),
            (
                "politicas",
                "Nunca revelar información personal de otros inquilinos (nombres, balances, unidades)",
            ),
            (
                "politicas",
                "No confirmar la recepción de un pago sin verificar primero en el sistema",
            ),
            (
                "politicas",
                "No dar consejos legales ni financieros — derivar al profesional correspondiente",
            ),
            (
                "politicas",
                "No compartir términos de contrato con personas que no sean parte del contrato",
            ),
        ];

        // Build the jsonb_build_array expression with all 16 template rules
        let rule_exprs: Vec<String> = templates
            .iter()
            .enumerate()
            .map(|(i, (cat, instr))| {
                #[allow(clippy::cast_possible_wrap)]
                let order = i as i32;
                rule_json_expr(cat, instr, order)
            })
            .collect();

        let templates_array = format!("jsonb_build_array({})", rule_exprs.join(", "));

        // Step 1: Seed all 16 template rules into every existing chatbot_config row
        let seed_sql = format!("UPDATE chatbot_config SET guidance_rules = {templates_array}");
        db.execute_unprepared(&seed_sql).await?;

        // Step 2: For rows with a non-empty system_prompt, append a custom rule in politicas
        let custom_rule_expr = concat!(
            "jsonb_build_object(",
            "'id', gen_random_uuid()::text, ",
            "'category', 'politicas', ",
            "'instruction', LEFT(system_prompt, 500), ",
            "'enabled', true, ",
            "'isTemplate', false, ",
            "'sortOrder', 16, ",
            "'createdAt', now()::text, ",
            "'updatedAt', now()::text",
            ")"
        );

        let append_custom_sql = format!(
            "UPDATE chatbot_config \
             SET guidance_rules = guidance_rules || jsonb_build_array({custom_rule_expr}) \
             WHERE system_prompt IS NOT NULL AND TRIM(system_prompt) <> ''"
        );
        db.execute_unprepared(&append_custom_sql).await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();

        // Revert: clear guidance_rules back to empty array
        db.execute_unprepared("UPDATE chatbot_config SET guidance_rules = '[]'::jsonb")
            .await?;

        Ok(())
    }
}
