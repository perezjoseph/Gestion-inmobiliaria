# Documento de Requisitos — Auto-Generación de Pagos desde Contratos

## Introducción

Actualmente los pagos (pagos) se crean manualmente uno por uno. Cuando se crea un contrato con monto_mensual y rango de fechas (fecha_inicio a fecha_fin), el administrador de propiedades debe crear cada registro de pago mensual a mano, lo cual es tedioso y propenso a errores.

Este módulo automatiza la generación de registros de pago mensuales a partir de contratos. Al crear o renovar un contrato activo, el sistema genera automáticamente un pago por cada mes del período contractual con estado "pendiente", el monto_mensual del contrato, y la moneda del contrato. También provee endpoints para previsualizar y disparar manualmente la generación, y maneja la cancelación de pagos futuros cuando un contrato se termina anticipadamente.

## Glosario

- **Sistema_Pagos_Auto**: Módulo backend y frontend que gestiona la generación automática de pagos a partir de contratos, incluyendo lógica de generación, endpoints REST, y componentes de interfaz.
- **Contrato**: Registro existente en la tabla `contratos` que vincula una propiedad con un inquilino por un período definido (fecha_inicio a fecha_fin) con un monto_mensual y moneda.
- **Pago**: Registro existente en la tabla `pagos` que representa una obligación de pago mensual vinculada a un contrato, con monto, moneda, fecha_vencimiento, y estado.
- **Pago_Generado**: Pago creado automáticamente por el Sistema_Pagos_Auto con estado "pendiente", monto igual al monto_mensual del contrato, moneda igual a la moneda del contrato, y fecha_vencimiento calculada.
- **Día_Vencimiento**: Día del mes en que vence cada pago generado. Por defecto es el día 1 de cada mes. Configurable por contrato o a nivel de organización.
- **Período_Contractual**: Rango de meses desde fecha_inicio hasta fecha_fin del contrato, donde cada mes genera un pago.
- **Preview_Pagos**: Respuesta que muestra los pagos que se generarían para un contrato sin crearlos en la base de datos.
- **WriteAccess**: Extractor de Actix-web que permite acceso a usuarios con rol `admin` o `gerente`.
- **Claims**: Datos del usuario autenticado extraídos del token JWT (sub, email, rol, organizacion_id).

## Requisitos

### Requisito 1: Generación automática de pagos al crear un contrato activo

**Historia de usuario:** Como gerente, quiero que al crear un contrato activo se generen automáticamente los pagos mensuales para todo el período contractual, para no tener que crearlos manualmente uno por uno.

#### Criterios de aceptación

1. WHEN un contrato con estado "activo" es creado exitosamente, THE Sistema_Pagos_Auto SHALL generar un Pago_Generado por cada mes del Período_Contractual dentro de la misma transacción de base de datos.
2. THE Sistema_Pagos_Auto SHALL asignar a cada Pago_Generado el monto_mensual del contrato como monto, la moneda del contrato como moneda, y el estado "pendiente".
3. THE Sistema_Pagos_Auto SHALL calcular la fecha_vencimiento de cada Pago_Generado como el Día_Vencimiento del mes correspondiente dentro del Período_Contractual.
4. WHEN el contrato tiene fecha_inicio que no coincide con el primer día del mes, THE Sistema_Pagos_Auto SHALL generar un Pago_Generado para el mes parcial inicial con la misma fecha_vencimiento calculada según el Día_Vencimiento de ese mes.
5. WHEN el contrato tiene fecha_fin que no coincide con el último día del mes, THE Sistema_Pagos_Auto SHALL generar un Pago_Generado para el mes parcial final con la misma fecha_vencimiento calculada según el Día_Vencimiento de ese mes.
6. WHEN el Período_Contractual es menor a un mes completo, THE Sistema_Pagos_Auto SHALL generar exactamente un Pago_Generado para ese período.
7. WHEN el contrato creado tiene un estado diferente a "activo", THE Sistema_Pagos_Auto SHALL omitir la generación de pagos para ese contrato.
8. THE Sistema_Pagos_Auto SHALL incluir en la respuesta de creación del contrato la cantidad de pagos generados en un campo `pagos_generados`.

### Requisito 2: Generación automática de pagos al renovar un contrato

**Historia de usuario:** Como gerente, quiero que al renovar un contrato se generen automáticamente los pagos mensuales para el nuevo período, para mantener la continuidad de cobros sin intervención manual.

#### Criterios de aceptación

1. WHEN un contrato es renovado exitosamente a través del endpoint `/renovar`, THE Sistema_Pagos_Auto SHALL generar Pagos_Generados para el nuevo Período_Contractual del contrato renovado dentro de la misma transacción.
2. THE Sistema_Pagos_Auto SHALL utilizar el monto_mensual del nuevo contrato renovado para los Pagos_Generados, no el monto del contrato original.
3. THE Sistema_Pagos_Auto SHALL utilizar la moneda del contrato original (heredada por el nuevo contrato) para los Pagos_Generados.
4. THE Sistema_Pagos_Auto SHALL incluir en la respuesta de renovación la cantidad de pagos generados en un campo `pagos_generados`.

### Requisito 3: Cancelación de pagos futuros al terminar un contrato anticipadamente

**Historia de usuario:** Como gerente, quiero que al terminar un contrato anticipadamente se cancelen automáticamente los pagos pendientes futuros, para que no queden obligaciones de pago por un período que ya no aplica.

#### Criterios de aceptación

1. WHEN un contrato activo es terminado anticipadamente a través del endpoint `/terminar`, THE Sistema_Pagos_Auto SHALL cambiar el estado a "cancelado" de todos los Pagos del contrato cuyo estado sea "pendiente" y cuya fecha_vencimiento sea posterior a la fecha de terminación.
2. THE Sistema_Pagos_Auto SHALL preservar sin cambios los Pagos del contrato cuyo estado sea "pagado" o "atrasado", independientemente de su fecha_vencimiento.
3. THE Sistema_Pagos_Auto SHALL preservar sin cambios los Pagos del contrato cuyo estado sea "pendiente" pero cuya fecha_vencimiento sea igual o anterior a la fecha de terminación.

### Requisito 4: Endpoint para previsualizar pagos a generar

**Historia de usuario:** Como gerente, quiero poder previsualizar qué pagos se generarían para un contrato antes de confirmar, para verificar que las fechas y montos son correctos.

#### Criterios de aceptación

1. WHEN un usuario autenticado solicita la previsualización de pagos para un contrato existente, THE Sistema_Pagos_Auto SHALL devolver un Preview_Pagos con la lista de pagos que se generarían, incluyendo monto, moneda, y fecha_vencimiento de cada uno.
2. THE Sistema_Pagos_Auto SHALL calcular el Preview_Pagos sin crear registros en la base de datos.
3. WHEN el contrato referenciado no existe, THE Sistema_Pagos_Auto SHALL devolver código HTTP 404 con un mensaje indicando que el contrato no fue encontrado.
4. THE Sistema_Pagos_Auto SHALL incluir en el Preview_Pagos la cantidad total de pagos que se generarían y el monto total acumulado.
5. WHEN el contrato ya tiene pagos generados previamente, THE Sistema_Pagos_Auto SHALL indicar en el Preview_Pagos cuántos pagos ya existen y cuántos nuevos se generarían, excluyendo los meses que ya tienen un pago asociado.

### Requisito 5: Endpoint para generación manual de pagos

**Historia de usuario:** Como gerente, quiero poder disparar manualmente la generación de pagos para un contrato específico, para cubrir casos donde la generación automática fue omitida o necesita re-ejecutarse.

#### Criterios de aceptación

1. WHEN un usuario con WriteAccess solicita la generación manual de pagos para un contrato activo existente, THE Sistema_Pagos_Auto SHALL generar Pagos_Generados para los meses del Período_Contractual que aún no tienen un pago asociado.
2. THE Sistema_Pagos_Auto SHALL omitir la generación de pagos para meses que ya tienen un Pago existente vinculado al contrato con la misma fecha_vencimiento.
3. WHEN el contrato referenciado no existe, THE Sistema_Pagos_Auto SHALL devolver código HTTP 404 con un mensaje indicando que el contrato no fue encontrado.
4. WHEN el contrato referenciado tiene un estado diferente a "activo", THE Sistema_Pagos_Auto SHALL devolver código HTTP 422 con un mensaje indicando que solo se pueden generar pagos para contratos activos.
5. THE Sistema_Pagos_Auto SHALL devolver la cantidad de pagos nuevos generados y la lista de pagos creados.
6. WHEN un usuario con rol visualizador intenta la generación manual, THE Sistema_Pagos_Auto SHALL rechazar la solicitud con código HTTP 403.

### Requisito 6: Cálculo del día de vencimiento

**Historia de usuario:** Como gerente, quiero que la fecha de vencimiento de cada pago generado sea el día 1 de cada mes por defecto, con la posibilidad de configurar un día diferente, para alinear los cobros con las fechas acordadas con los inquilinos.

#### Criterios de aceptación

1. THE Sistema_Pagos_Auto SHALL utilizar el día 1 de cada mes como Día_Vencimiento por defecto cuando no se especifique un valor diferente.
2. WHEN el Día_Vencimiento configurado excede la cantidad de días del mes (por ejemplo, día 31 en un mes de 30 días), THE Sistema_Pagos_Auto SHALL utilizar el último día de ese mes como fecha_vencimiento.
3. WHEN se solicita la generación manual de pagos con un parámetro `dia_vencimiento`, THE Sistema_Pagos_Auto SHALL utilizar ese valor como Día_Vencimiento para los pagos generados.
4. WHEN el parámetro `dia_vencimiento` tiene un valor menor a 1 o mayor a 31, THE Sistema_Pagos_Auto SHALL rechazar la solicitud con código HTTP 422 indicando que el día de vencimiento debe estar entre 1 y 31.

### Requisito 7: Manejo de pagos duplicados

**Historia de usuario:** Como gerente, quiero que el sistema evite generar pagos duplicados para el mismo mes de un contrato, para mantener la integridad de los datos financieros.

#### Criterios de aceptación

1. WHEN el Sistema_Pagos_Auto genera pagos para un contrato que ya tiene pagos existentes, THE Sistema_Pagos_Auto SHALL comparar las fechas de vencimiento de los pagos existentes con las fechas calculadas y omitir la generación para meses que ya tienen un pago.
2. THE Sistema_Pagos_Auto SHALL determinar que un mes ya tiene pago cuando existe un Pago vinculado al contrato cuya fecha_vencimiento cae en el mismo año y mes que la fecha_vencimiento calculada.
3. THE Sistema_Pagos_Auto SHALL generar pagos únicamente para los meses faltantes del Período_Contractual.

### Requisito 8: Registro de auditoría

**Historia de usuario:** Como administrador, quiero que todas las operaciones de generación y cancelación automática de pagos queden registradas en la auditoría, para tener trazabilidad de los cambios realizados por el sistema.

#### Criterios de aceptación

1. WHEN el Sistema_Pagos_Auto genera pagos automáticamente (al crear o renovar un contrato), THE Sistema_Pagos_Auto SHALL registrar una entrada de auditoría con la acción "generar_pagos_auto", el ID del contrato, y la cantidad de pagos generados.
2. WHEN el Sistema_Pagos_Auto genera pagos manualmente (a través del endpoint de generación manual), THE Sistema_Pagos_Auto SHALL registrar una entrada de auditoría con la acción "generar_pagos_manual", el ID del contrato, y la cantidad de pagos generados.
3. WHEN el Sistema_Pagos_Auto cancela pagos futuros (al terminar un contrato), THE Sistema_Pagos_Auto SHALL registrar una entrada de auditoría con la acción "cancelar_pagos_futuros", el ID del contrato, y la cantidad de pagos cancelados.

### Requisito 9: Interfaz de usuario para generación de pagos

**Historia de usuario:** Como gerente, quiero un botón en la vista de detalle del contrato para generar pagos y ver cuántos pagos se generaron, para gestionar los pagos de forma visual.

#### Criterios de aceptación

1. THE Sistema_Pagos_Auto SHALL mostrar un botón "Generar Pagos" en la vista de detalle del contrato cuando el contrato tiene estado "activo".
2. WHILE el contrato tiene un estado diferente a "activo", THE Sistema_Pagos_Auto SHALL ocultar el botón "Generar Pagos".
3. WHEN el usuario hace clic en el botón "Generar Pagos", THE Sistema_Pagos_Auto SHALL mostrar primero el Preview_Pagos con la lista de pagos a generar y solicitar confirmación antes de proceder.
4. WHEN la generación de pagos se completa exitosamente, THE Sistema_Pagos_Auto SHALL mostrar un mensaje indicando la cantidad de pagos generados.
5. WHEN se crea un contrato exitosamente, THE Sistema_Pagos_Auto SHALL mostrar en la notificación de éxito la cantidad de pagos generados automáticamente.
6. WHILE un usuario tiene rol visualizador, THE Sistema_Pagos_Auto SHALL ocultar el botón "Generar Pagos".
7. THE Sistema_Pagos_Auto SHALL mostrar todos los textos de la interfaz en español.
