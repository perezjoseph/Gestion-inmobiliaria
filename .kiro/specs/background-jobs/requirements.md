# Documento de Requisitos — Tareas de Fondo Programadas

## Introducción

Varias funciones del sistema requieren ejecución periódica pero actualmente dependen de disparadores manuales o de efectos secundarios en endpoints de lectura (por ejemplo, `marcar_vencidos` se ejecuta dentro de `listar_documentos`). Esto causa contención innecesaria en endpoints GET y deja sin ejecutar tareas críticas como marcar pagos atrasados, vencer contratos expirados, y generar notificaciones.

Este módulo introduce un scheduler ligero de tareas de fondo que se ejecuta dentro del mismo proceso Actix-web usando `tokio::spawn` con `tokio::time::interval`. Cada tarea tiene un intervalo configurable (por defecto cada 24 horas), es idempotente, y registra su ejecución en una tabla de historial. Adicionalmente, se proveen endpoints de administración para disparar tareas manualmente y consultar el historial de ejecuciones.

## Glosario

- **Scheduler**: Componente que ejecuta tareas de fondo periódicamente dentro del proceso Actix-web usando `tokio::spawn` y `tokio::time::interval`.
- **Tarea_Fondo**: Función asíncrona registrada en el Scheduler que ejecuta una operación de negocio periódicamente. Cada tarea tiene un nombre único, un intervalo de ejecución, y es idempotente.
- **Ejecucion_Tarea**: Registro en la tabla `ejecuciones_tareas` que documenta cuándo se ejecutó una Tarea_Fondo, cuánto tardó, si fue exitosa, y cuántos registros afectó.
- **Sistema_Tareas**: Módulo backend que gestiona el Scheduler, las Tareas_Fondo, los endpoints de administración, y el registro de ejecuciones.
- **WriteAccess**: Extractor de Actix-web que permite acceso a usuarios con rol `admin` o `gerente`.
- **AdminOnly**: Extractor de Actix-web que permite acceso exclusivo a usuarios con rol `admin`.
- **Claims**: Datos del usuario autenticado extraídos del token JWT (sub, email, rol, organizacion_id).

## Requisitos

### Requisito 1: Marcar pagos atrasados

**Historia de usuario:** Como gerente, quiero que los pagos pendientes cuya fecha de vencimiento ya pasó se marquen automáticamente como atrasados, para tener visibilidad precisa del estado de cobros sin intervención manual.

#### Criterios de aceptación

1. WHEN el Scheduler ejecuta la tarea de marcar pagos atrasados, THE Sistema_Tareas SHALL actualizar el estado de todos los pagos con estado "pendiente" y fecha_vencimiento anterior a la fecha actual a estado "atrasado".
2. THE Sistema_Tareas SHALL invocar la función existente `pagos::mark_overdue` para ejecutar esta operación.
3. THE Sistema_Tareas SHALL registrar una Ejecucion_Tarea con el nombre "marcar_pagos_atrasados", la duración, el resultado (éxito o error), y la cantidad de pagos actualizados.
4. THE Sistema_Tareas SHALL ejecutar esta tarea de forma idempotente: ejecutarla múltiples veces consecutivas sin cambios en los datos entre ejecuciones produce cero actualizaciones adicionales.

### Requisito 2: Marcar contratos vencidos

**Historia de usuario:** Como gerente, quiero que los contratos activos cuya fecha de fin ya pasó se marquen automáticamente como vencidos, para reflejar el estado real de los contratos sin revisión manual.

#### Criterios de aceptación

1. WHEN el Scheduler ejecuta la tarea de marcar contratos vencidos, THE Sistema_Tareas SHALL actualizar el estado de todos los contratos con estado "activo" y fecha_fin anterior a la fecha actual a estado "vencido".
2. THE Sistema_Tareas SHALL ejecutar esta operación como un `update_many` en la tabla de contratos, actualizando también el campo `updated_at`.
3. THE Sistema_Tareas SHALL registrar una Ejecucion_Tarea con el nombre "marcar_contratos_vencidos", la duración, el resultado, y la cantidad de contratos actualizados.
4. THE Sistema_Tareas SHALL ejecutar esta tarea de forma idempotente: ejecutarla múltiples veces consecutivas sin cambios en los datos entre ejecuciones produce cero actualizaciones adicionales.

### Requisito 3: Marcar documentos vencidos

**Historia de usuario:** Como gerente, quiero que los documentos verificados cuya fecha de vencimiento ya pasó se marquen automáticamente como vencidos, para mantener el estado de cumplimiento actualizado.

#### Criterios de aceptación

1. WHEN el Scheduler ejecuta la tarea de marcar documentos vencidos, THE Sistema_Tareas SHALL actualizar el estado de verificación de todos los documentos con estado_verificacion "verificado" y fecha_vencimiento anterior a la fecha actual a estado_verificacion "vencido".
2. THE Sistema_Tareas SHALL invocar la función existente `documentos::marcar_vencidos` para ejecutar esta operación.
3. THE Sistema_Tareas SHALL registrar una Ejecucion_Tarea con el nombre "marcar_documentos_vencidos", la duración, el resultado, y la cantidad de documentos actualizados.
4. THE Sistema_Tareas SHALL ejecutar esta tarea de forma idempotente: ejecutarla múltiples veces consecutivas sin cambios en los datos entre ejecuciones produce cero actualizaciones adicionales.

### Requisito 4: Generar notificaciones

**Historia de usuario:** Como gerente, quiero que el sistema genere automáticamente notificaciones por pagos vencidos, contratos por vencer, y documentos por vencer, para recibir avisos oportunos sin depender de disparadores manuales.

#### Criterios de aceptación

1. WHEN el Scheduler ejecuta la tarea de generar notificaciones, THE Sistema_Tareas SHALL invocar la función `notificaciones::generar_notificaciones` para cada organización activa en el sistema.
2. THE Sistema_Tareas SHALL registrar una Ejecucion_Tarea con el nombre "generar_notificaciones", la duración, el resultado, y la cantidad total de notificaciones generadas.
3. THE Sistema_Tareas SHALL ejecutar esta tarea de forma idempotente: el generador de notificaciones verifica duplicados internamente, por lo que ejecutarla múltiples veces consecutivas produce cero notificaciones adicionales.

### Requisito 5: Registro de ejecución de tareas

**Historia de usuario:** Como administrador, quiero ver cuándo se ejecutó cada tarea, cuánto tardó, y si fue exitosa, para monitorear la salud del sistema y diagnosticar problemas.

#### Criterios de aceptación

1. WHEN una Tarea_Fondo se ejecuta (ya sea por el Scheduler o manualmente), THE Sistema_Tareas SHALL crear un registro de Ejecucion_Tarea con los campos: id (UUID), nombre_tarea (texto), iniciado_en (timestamp), duracion_ms (entero), exitosa (booleano), registros_afectados (entero), y mensaje_error (texto opcional).
2. IF una Tarea_Fondo falla durante su ejecución, THEN THE Sistema_Tareas SHALL registrar la Ejecucion_Tarea con exitosa en falso y el mensaje de error en el campo mensaje_error.
3. IF una Tarea_Fondo falla durante su ejecución, THEN THE Sistema_Tareas SHALL registrar el error con `tracing::error!` y continuar la ejecución del Scheduler sin interrumpir las demás tareas.
4. THE Sistema_Tareas SHALL almacenar el historial de ejecuciones en la tabla `ejecuciones_tareas` con índices en nombre_tarea e iniciado_en para consultas eficientes.

### Requisito 6: Scheduler de tareas de fondo

**Historia de usuario:** Como administrador, quiero que las tareas de fondo se ejecuten automáticamente al iniciar la aplicación con intervalos configurables, para que el sistema se mantenga actualizado sin intervención manual.

#### Criterios de aceptación

1. WHEN la aplicación Actix-web inicia, THE Sistema_Tareas SHALL iniciar el Scheduler como una tarea de fondo usando `tokio::spawn`.
2. THE Sistema_Tareas SHALL ejecutar cada Tarea_Fondo según su intervalo configurado usando `tokio::time::interval`.
3. THE Sistema_Tareas SHALL utilizar un intervalo por defecto de 24 horas (86400 segundos) para todas las tareas.
4. THE Sistema_Tareas SHALL ejecutar las tareas dentro del mismo proceso Actix-web, sin requerir un binario separado ni un cron externo.
5. IF el Scheduler encuentra un error al ejecutar una tarea, THEN THE Sistema_Tareas SHALL registrar el error y continuar con la siguiente ejecución programada sin detener el proceso.

### Requisito 7: Endpoint para disparar tarea manualmente

**Historia de usuario:** Como administrador, quiero poder disparar cualquier tarea de fondo manualmente a través de un endpoint, para forzar la ejecución inmediata cuando sea necesario.

#### Criterios de aceptación

1. WHEN un usuario con AdminOnly envía una solicitud para ejecutar una tarea específica por nombre, THE Sistema_Tareas SHALL ejecutar la Tarea_Fondo correspondiente de forma inmediata y devolver el resultado con código HTTP 200.
2. WHEN un usuario con AdminOnly envía una solicitud con un nombre de tarea que no existe, THE Sistema_Tareas SHALL devolver código HTTP 404 con un mensaje indicando que la tarea no fue encontrada.
3. THE Sistema_Tareas SHALL registrar una Ejecucion_Tarea para las ejecuciones manuales con los mismos campos que las ejecuciones automáticas.
4. WHEN un usuario con rol gerente intenta disparar una tarea manualmente, THE Sistema_Tareas SHALL rechazar la solicitud con código HTTP 403.
5. WHEN un usuario con rol visualizador intenta disparar una tarea manualmente, THE Sistema_Tareas SHALL rechazar la solicitud con código HTTP 403.

### Requisito 8: Endpoint para consultar historial de ejecuciones

**Historia de usuario:** Como administrador, quiero consultar el historial de ejecuciones de las tareas de fondo, para verificar que se están ejecutando correctamente y diagnosticar fallos.

#### Criterios de aceptación

1. WHEN un usuario con AdminOnly solicita el historial de ejecuciones, THE Sistema_Tareas SHALL devolver una respuesta paginada con las ejecuciones ordenadas por iniciado_en descendente.
2. WHEN un usuario con AdminOnly solicita el historial con filtro de nombre_tarea, THE Sistema_Tareas SHALL devolver únicamente las ejecuciones de la tarea especificada.
3. WHEN un usuario con AdminOnly solicita el historial con filtro de exitosa, THE Sistema_Tareas SHALL devolver únicamente las ejecuciones con el resultado especificado.
4. WHEN un usuario con rol gerente intenta consultar el historial, THE Sistema_Tareas SHALL rechazar la solicitud con código HTTP 403.
5. WHEN un usuario con rol visualizador intenta consultar el historial, THE Sistema_Tareas SHALL rechazar la solicitud con código HTTP 403.

### Requisito 9: Resiliencia del Scheduler

**Historia de usuario:** Como administrador, quiero que el fallo de una tarea individual no afecte la ejecución de las demás tareas ni la estabilidad de la aplicación, para garantizar la disponibilidad del sistema.

#### Criterios de aceptación

1. IF una Tarea_Fondo produce un panic durante su ejecución, THEN THE Sistema_Tareas SHALL capturar el panic, registrar el error, y continuar con las demás tareas programadas.
2. IF una Tarea_Fondo retorna un error, THEN THE Sistema_Tareas SHALL registrar el error en la Ejecucion_Tarea y en los logs, y continuar con la siguiente ejecución programada de esa tarea.
3. THE Sistema_Tareas SHALL ejecutar cada Tarea_Fondo de forma independiente, de modo que el fallo de una tarea no bloquee ni retrase la ejecución de las demás.
