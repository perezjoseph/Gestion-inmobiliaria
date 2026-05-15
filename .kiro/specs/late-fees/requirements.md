# Documento de Requisitos — Recargos por Mora (Late Fees)

## Introducción

Los contratos de alquiler en República Dominicana comúnmente incluyen una cláusula de penalidad por pago tardío (recargos). Este módulo agrega la capacidad de configurar un porcentaje de recargo y un período de gracia a nivel de organización y de contrato, calcular automáticamente el monto del recargo cuando un pago se marca como atrasado, almacenar el recargo en el registro de pago, y mostrarlo en la interfaz de usuario.

El recargo se calcula como: `recargo = monto * (recargo_porcentaje / 100)`. El período de gracia (`dias_gracia`) permite que un pago no se considere atrasado hasta que hayan transcurrido esos días adicionales después de la fecha de vencimiento.

## Glosario

- **Sistema_Recargos**: Módulo backend y frontend que gestiona la configuración, cálculo, almacenamiento y visualización de recargos por mora en pagos atrasados.
- **Recargo**: Monto de penalidad calculado sobre un pago atrasado, expresado como un valor decimal con dos dígitos de precisión. Se calcula como `monto * (recargo_porcentaje / 100)`.
- **Recargo_Porcentaje**: Porcentaje de penalidad aplicable a pagos atrasados. Valor decimal nullable. Configurable a nivel de organización (por defecto) y a nivel de contrato (override).
- **Dias_Gracia**: Cantidad de días adicionales después de la fecha de vencimiento durante los cuales un pago no se considera atrasado. Valor entero nullable. Configurable a nivel de contrato.
- **Pago_Atrasado**: Pago cuyo estado es "atrasado", ya sea marcado automáticamente por `mark_overdue` o manualmente por un usuario.
- **Configuracion_Org**: Registro en la tabla `configuracion` con clave `recargo_porcentaje_defecto` que almacena el porcentaje de recargo por defecto a nivel de organización.
- **WriteAccess**: Extractor de Actix-web que permite acceso a usuarios con rol `admin` o `gerente`.
- **AdminOnly**: Extractor de Actix-web que permite acceso exclusivo a usuarios con rol `admin`.
- **Claims**: Datos del usuario autenticado extraídos del token JWT (sub, email, rol, organizacion_id).

## Requisitos

### Requisito 1: Campos de recargo en el contrato

**Historia de usuario:** Como gerente, quiero configurar un porcentaje de recargo y un período de gracia en cada contrato, para definir las condiciones de penalidad por pago tardío específicas de cada acuerdo.

#### Criterios de aceptación

1. THE Sistema_Recargos SHALL almacenar un campo `recargo_porcentaje` de tipo DECIMAL(5,2) nullable en la tabla `contratos`.
2. THE Sistema_Recargos SHALL almacenar un campo `dias_gracia` de tipo INTEGER nullable en la tabla `contratos`.
3. WHEN un usuario con WriteAccess crea un contrato sin especificar `recargo_porcentaje`, THE Sistema_Recargos SHALL almacenar NULL en el campo `recargo_porcentaje` del contrato.
4. WHEN un usuario con WriteAccess crea un contrato sin especificar `dias_gracia`, THE Sistema_Recargos SHALL almacenar NULL en el campo `dias_gracia` del contrato.
5. WHEN un usuario con WriteAccess crea o actualiza un contrato con un `recargo_porcentaje` menor a 0 o mayor a 100, THE Sistema_Recargos SHALL rechazar la solicitud con código HTTP 422 indicando que el porcentaje de recargo debe estar entre 0 y 100.
6. WHEN un usuario con WriteAccess crea o actualiza un contrato con un `dias_gracia` menor a 0, THE Sistema_Recargos SHALL rechazar la solicitud con código HTTP 422 indicando que los días de gracia deben ser mayor o igual a 0.
7. WHEN un usuario autenticado consulta un contrato, THE Sistema_Recargos SHALL incluir los campos `recargo_porcentaje` y `dias_gracia` en la respuesta.

### Requisito 2: Campo de recargo en el pago

**Historia de usuario:** Como gerente, quiero que el monto del recargo se almacene en cada pago atrasado, para tener un registro claro de la penalidad aplicada.

#### Criterios de aceptación

1. THE Sistema_Recargos SHALL almacenar un campo `recargo` de tipo DECIMAL(12,2) nullable en la tabla `pagos`.
2. WHEN un usuario autenticado consulta un pago que tiene recargo calculado, THE Sistema_Recargos SHALL incluir el campo `recargo` en la respuesta.
3. WHEN un usuario autenticado consulta un pago sin recargo, THE Sistema_Recargos SHALL devolver NULL en el campo `recargo` de la respuesta.

### Requisito 3: Configuración de recargo por defecto a nivel de organización

**Historia de usuario:** Como administrador, quiero configurar un porcentaje de recargo por defecto a nivel de organización, para que los contratos que no especifiquen un porcentaje propio utilicen este valor.

#### Criterios de aceptación

1. WHEN un usuario con AdminOnly actualiza el porcentaje de recargo por defecto, THE Sistema_Recargos SHALL almacenar el valor en la tabla `configuracion` con clave `recargo_porcentaje_defecto`.
2. WHEN un usuario con AdminOnly envía un porcentaje de recargo por defecto menor a 0 o mayor a 100, THE Sistema_Recargos SHALL rechazar la solicitud con código HTTP 422 indicando que el porcentaje debe estar entre 0 y 100.
3. WHEN un usuario autenticado consulta la configuración de recargo, THE Sistema_Recargos SHALL devolver el porcentaje de recargo por defecto almacenado.
4. WHEN no existe configuración de recargo por defecto, THE Sistema_Recargos SHALL devolver NULL como valor del porcentaje de recargo por defecto.

### Requisito 4: Resolución de porcentaje de recargo (contrato vs organización)

**Historia de usuario:** Como gerente, quiero que el porcentaje de recargo del contrato tenga prioridad sobre el valor por defecto de la organización, para poder personalizar las condiciones por contrato.

#### Criterios de aceptación

1. WHEN el Sistema_Recargos calcula un recargo y el contrato tiene `recargo_porcentaje` definido (no NULL), THE Sistema_Recargos SHALL utilizar el `recargo_porcentaje` del contrato para el cálculo.
2. WHEN el Sistema_Recargos calcula un recargo y el contrato tiene `recargo_porcentaje` NULL, THE Sistema_Recargos SHALL utilizar el `recargo_porcentaje_defecto` de la Configuracion_Org para el cálculo.
3. WHEN el Sistema_Recargos calcula un recargo y tanto el contrato como la Configuracion_Org tienen porcentaje NULL, THE Sistema_Recargos SHALL omitir el cálculo del recargo y dejar el campo `recargo` del pago como NULL.

### Requisito 5: Cálculo automático de recargo al marcar pagos como atrasados

**Historia de usuario:** Como gerente, quiero que el recargo se calcule automáticamente cuando un pago se marca como atrasado, para no tener que calcular manualmente la penalidad.

#### Criterios de aceptación

1. WHEN la función `mark_overdue` marca un pago como "atrasado", THE Sistema_Recargos SHALL calcular el recargo como `monto * (recargo_porcentaje_efectivo / 100)` y almacenarlo en el campo `recargo` del pago.
2. WHEN un usuario con WriteAccess actualiza manualmente el estado de un pago a "atrasado", THE Sistema_Recargos SHALL calcular el recargo como `monto * (recargo_porcentaje_efectivo / 100)` y almacenarlo en el campo `recargo` del pago.
3. THE Sistema_Recargos SHALL calcular el recargo con precisión DECIMAL(12,2), redondeando a dos decimales.
4. WHEN el recargo_porcentaje_efectivo es NULL (ni contrato ni organización lo definen), THE Sistema_Recargos SHALL dejar el campo `recargo` del pago como NULL al marcarlo como atrasado.
5. WHEN el recargo_porcentaje_efectivo es 0, THE Sistema_Recargos SHALL almacenar 0.00 como valor del recargo.

### Requisito 6: Período de gracia en la determinación de atraso

**Historia de usuario:** Como gerente, quiero que el período de gracia se considere antes de marcar un pago como atrasado, para dar un margen de tiempo al inquilino antes de aplicar penalidades.

#### Criterios de aceptación

1. WHEN el contrato tiene `dias_gracia` definido y la función `mark_overdue` evalúa un pago pendiente, THE Sistema_Recargos SHALL considerar el pago como atrasado solo si la fecha actual es posterior a `fecha_vencimiento + dias_gracia` días.
2. WHEN el contrato tiene `dias_gracia` NULL, THE Sistema_Recargos SHALL utilizar el comportamiento actual donde el pago se considera atrasado si la fecha actual es posterior a `fecha_vencimiento`.
3. WHILE un pago pendiente se encuentra dentro del período de gracia (fecha actual <= fecha_vencimiento + dias_gracia), THE Sistema_Recargos SHALL mantener el estado del pago como "pendiente".

### Requisito 7: Visualización del recargo en la interfaz

**Historia de usuario:** Como gerente, quiero ver el monto del recargo junto al pago en la interfaz, para tener visibilidad clara de las penalidades aplicadas.

#### Criterios de aceptación

1. WHEN un pago tiene un recargo calculado (no NULL), THE Sistema_Recargos SHALL mostrar el monto del recargo junto al monto del pago en la lista de pagos y en la vista de detalle.
2. WHEN un pago tiene un recargo calculado, THE Sistema_Recargos SHALL mostrar el monto total (monto + recargo) como referencia visual.
3. THE Sistema_Recargos SHALL mostrar el recargo con formato de moneda apropiado (DOP o USD) con dos decimales.
4. WHEN un pago no tiene recargo (NULL), THE Sistema_Recargos SHALL omitir la visualización del recargo en la interfaz.
5. THE Sistema_Recargos SHALL mostrar todos los textos relacionados con recargos en español, incluyendo etiquetas como "Recargo", "Monto Total", y "Porcentaje de Recargo".

### Requisito 8: Visualización de configuración de recargo en contratos

**Historia de usuario:** Como gerente, quiero ver y editar el porcentaje de recargo y los días de gracia al crear o editar un contrato, para configurar las condiciones de penalidad.

#### Criterios de aceptación

1. THE Sistema_Recargos SHALL mostrar campos para `recargo_porcentaje` y `dias_gracia` en el formulario de creación y edición de contratos.
2. THE Sistema_Recargos SHALL mostrar el porcentaje de recargo y los días de gracia en la vista de detalle del contrato.
3. WHEN el contrato no tiene `recargo_porcentaje` definido, THE Sistema_Recargos SHALL mostrar una indicación de que se utilizará el valor por defecto de la organización (si existe).

### Requisito 9: Configuración de recargo en la página de configuración

**Historia de usuario:** Como administrador, quiero gestionar el porcentaje de recargo por defecto desde la página de configuración, para establecer la política de penalidades de la organización.

#### Criterios de aceptación

1. THE Sistema_Recargos SHALL mostrar el porcentaje de recargo por defecto actual en la página de configuración de la organización.
2. WHEN un usuario con AdminOnly actualiza el porcentaje de recargo por defecto, THE Sistema_Recargos SHALL guardar el nuevo valor y mostrar un mensaje de confirmación.
3. WHILE un usuario tiene rol diferente a admin, THE Sistema_Recargos SHALL deshabilitar la edición del porcentaje de recargo por defecto.
4. THE Sistema_Recargos SHALL mostrar todos los textos de configuración de recargo en español.

### Requisito 10: Registro de auditoría

**Historia de usuario:** Como administrador, quiero que los cambios en la configuración de recargos y el cálculo de recargos queden registrados en la auditoría, para tener trazabilidad.

#### Criterios de aceptación

1. WHEN un usuario actualiza la configuración de recargo por defecto de la organización, THE Sistema_Recargos SHALL registrar la operación en la tabla de registros de auditoría incluyendo el ID del usuario, la acción realizada, y los valores anterior y nuevo.
2. WHEN la función `mark_overdue` calcula y almacena recargos en pagos, THE Sistema_Recargos SHALL registrar una entrada de auditoría con la cantidad de pagos afectados y los recargos calculados.
