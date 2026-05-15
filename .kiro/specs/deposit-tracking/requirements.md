# Documento de Requisitos — Seguimiento de Depósitos de Garantía

## Introducción

Los contratos de alquiler típicamente requieren un depósito de garantía (depósito de garantía). La entidad contrato ya tiene un campo `deposito` (Decimal, nullable), pero no existe seguimiento del ciclo de vida del depósito: si fue cobrado, cuándo se cobró, si fue devuelto o retenido parcialmente, y por qué motivo. Este módulo agrega campos de seguimiento al contrato existente y un endpoint dedicado para gestionar las transiciones de estado del depósito, junto con una sección visual en la vista de detalle del contrato.

## Glosario

- **Sistema_Depositos**: Módulo backend y frontend que gestiona el seguimiento de depósitos de garantía en contratos, incluyendo campos adicionales en la entidad contrato, endpoint REST para cambio de estado, y sección de interfaz en el detalle del contrato.
- **Estado_Deposito**: Valor que indica la fase actual del depósito de garantía: `pendiente`, `cobrado`, `devuelto`, o `retenido`.
- **Monto_Retenido**: Porción del depósito de garantía que se retiene por daños u otras razones, expresada como DECIMAL(12,2). Debe ser mayor a cero y menor o igual al monto del depósito.
- **Motivo_Retencion**: Texto libre que describe la razón por la cual se retiene parte o todo el depósito.
- **WriteAccess**: Extractor de Actix-web que permite acceso a usuarios con rol `admin` o `gerente`.
- **Claims**: Datos del usuario autenticado extraídos del token JWT (sub, email, rol, organizacion_id).

## Requisitos

### Requisito 1: Campos de seguimiento del depósito en el contrato

**Historia de usuario:** Como gerente, quiero que cada contrato con depósito tenga campos de seguimiento (estado, fechas de cobro y devolución, monto retenido, motivo de retención), para saber en todo momento el estado del depósito de garantía.

#### Criterios de aceptación

1. THE Sistema_Depositos SHALL almacenar un campo `estado_deposito` de tipo VARCHAR(20) nullable en la tabla `contratos` con valores válidos: `pendiente`, `cobrado`, `devuelto`, `retenido`.
2. THE Sistema_Depositos SHALL almacenar un campo `fecha_cobro_deposito` de tipo TIMESTAMP WITH TIME ZONE nullable en la tabla `contratos`.
3. THE Sistema_Depositos SHALL almacenar un campo `fecha_devolucion_deposito` de tipo TIMESTAMP WITH TIME ZONE nullable en la tabla `contratos`.
4. THE Sistema_Depositos SHALL almacenar un campo `monto_retenido` de tipo DECIMAL(12,2) nullable en la tabla `contratos`.
5. THE Sistema_Depositos SHALL almacenar un campo `motivo_retencion` de tipo TEXT nullable en la tabla `contratos`.
6. WHEN un contrato es creado con un valor de `deposito` mayor a cero, THE Sistema_Depositos SHALL asignar `estado_deposito` con valor `pendiente` automáticamente.
7. WHEN un contrato es creado sin valor de `deposito` o con `deposito` igual a cero o nulo, THE Sistema_Depositos SHALL mantener `estado_deposito` como nulo.

### Requisito 2: Transiciones de estado del depósito

**Historia de usuario:** Como gerente, quiero cambiar el estado del depósito de un contrato siguiendo un flujo definido, para registrar cuándo se cobró, devolvió, o retuvo el depósito.

#### Criterios de aceptación

1. WHEN un usuario con WriteAccess cambia el estado del depósito de `pendiente` a `cobrado`, THE Sistema_Depositos SHALL actualizar el Estado_Deposito y registrar la fecha actual en `fecha_cobro_deposito`.
2. WHEN un usuario con WriteAccess cambia el estado del depósito de `cobrado` a `devuelto`, THE Sistema_Depositos SHALL actualizar el Estado_Deposito y registrar la fecha actual en `fecha_devolucion_deposito`.
3. WHEN un usuario con WriteAccess cambia el estado del depósito de `cobrado` a `retenido` proporcionando `monto_retenido` y `motivo_retencion`, THE Sistema_Depositos SHALL actualizar el Estado_Deposito, almacenar el Monto_Retenido y el Motivo_Retencion, y registrar la fecha actual en `fecha_devolucion_deposito`.
4. WHEN un usuario con WriteAccess intenta cambiar el estado del depósito de `pendiente` directamente a `devuelto` o `retenido`, THE Sistema_Depositos SHALL rechazar la transición con código HTTP 422 y un mensaje indicando que el depósito debe ser cobrado antes de ser devuelto o retenido.
5. WHEN un usuario con WriteAccess intenta cambiar el estado del depósito de `devuelto` o `retenido` a cualquier otro estado, THE Sistema_Depositos SHALL rechazar la transición con código HTTP 422 y un mensaje indicando que los depósitos devueltos o retenidos no pueden cambiar de estado.
6. WHEN un usuario con WriteAccess intenta cambiar el estado del depósito de un contrato que no tiene depósito registrado, THE Sistema_Depositos SHALL rechazar la solicitud con código HTTP 422 y un mensaje indicando que el contrato no tiene depósito de garantía.
7. WHEN un usuario con rol visualizador intenta cambiar el estado del depósito, THE Sistema_Depositos SHALL rechazar la solicitud con código HTTP 403.

### Requisito 3: Validación de retención parcial

**Historia de usuario:** Como gerente, quiero registrar una retención parcial del depósito con el monto retenido y el motivo, para documentar cuánto se retuvo y por qué.

#### Criterios de aceptación

1. WHEN un usuario con WriteAccess cambia el estado a `retenido` sin proporcionar `monto_retenido`, THE Sistema_Depositos SHALL rechazar la solicitud con código HTTP 422 indicando que el monto retenido es requerido.
2. WHEN un usuario con WriteAccess cambia el estado a `retenido` con un `monto_retenido` menor o igual a cero, THE Sistema_Depositos SHALL rechazar la solicitud con código HTTP 422 indicando que el monto retenido debe ser mayor a cero.
3. WHEN un usuario con WriteAccess cambia el estado a `retenido` con un `monto_retenido` mayor al valor del depósito del contrato, THE Sistema_Depositos SHALL rechazar la solicitud con código HTTP 422 indicando que el monto retenido no puede exceder el depósito.
4. WHEN un usuario con WriteAccess cambia el estado a `retenido` sin proporcionar `motivo_retencion` o con texto vacío, THE Sistema_Depositos SHALL rechazar la solicitud con código HTTP 422 indicando que el motivo de retención es requerido.

### Requisito 4: Endpoint para cambiar estado del depósito

**Historia de usuario:** Como gerente, quiero un endpoint dedicado para cambiar el estado del depósito de un contrato, para gestionar el ciclo de vida del depósito de forma independiente a la actualización general del contrato.

#### Criterios de aceptación

1. WHEN un usuario con WriteAccess envía una solicitud PUT a `/contratos/{id}/deposito` con un estado válido, THE Sistema_Depositos SHALL actualizar el estado del depósito y devolver el contrato actualizado con código HTTP 200.
2. WHEN un usuario con WriteAccess envía una solicitud PUT a `/contratos/{id}/deposito` para un contrato que no existe, THE Sistema_Depositos SHALL devolver código HTTP 404 con un mensaje indicando que el contrato no fue encontrado.
3. WHEN un usuario con WriteAccess envía una solicitud PUT a `/contratos/{id}/deposito` con un valor de `estado` que no es `pendiente`, `cobrado`, `devuelto`, o `retenido`, THE Sistema_Depositos SHALL rechazar la solicitud con código HTTP 422 indicando los valores válidos.
4. THE Sistema_Depositos SHALL incluir los campos de depósito (estado_deposito, fecha_cobro_deposito, fecha_devolucion_deposito, monto_retenido, motivo_retencion) en la respuesta de ContratoResponse para todos los endpoints de contratos.

### Requisito 5: Registro de auditoría

**Historia de usuario:** Como administrador, quiero que todos los cambios de estado del depósito queden registrados en la auditoría, para tener trazabilidad de quién cambió el estado y cuándo.

#### Criterios de aceptación

1. WHEN un usuario cambia el estado del depósito de un contrato, THE Sistema_Depositos SHALL registrar la operación en la tabla de registros de auditoría incluyendo el ID del usuario, la acción "cambiar_estado_deposito", el ID del contrato, y los valores anterior y nuevo del estado.

### Requisito 6: Interfaz de usuario para seguimiento de depósitos

**Historia de usuario:** Como gerente, quiero ver una sección dedicada al depósito en la vista de detalle del contrato con el estado actual, fechas, y datos de retención, para gestionar el depósito de forma visual.

#### Criterios de aceptación

1. THE Sistema_Depositos SHALL mostrar una sección "Depósito de Garantía" en la vista de detalle del contrato cuando el contrato tiene un valor de depósito registrado.
2. THE Sistema_Depositos SHALL mostrar el monto del depósito con formato de moneda apropiado (DOP o USD), el Estado_Deposito con un badge de color diferenciado (pendiente=amarillo, cobrado=azul, devuelto=verde, retenido=rojo), y las fechas en formato DD/MM/YYYY.
3. WHEN el Estado_Deposito es `retenido`, THE Sistema_Depositos SHALL mostrar el Monto_Retenido, el monto devuelto (depósito menos Monto_Retenido), y el Motivo_Retencion.
4. THE Sistema_Depositos SHALL mostrar botones para cambiar el estado del depósito según las transiciones válidas desde el estado actual.
5. WHILE un usuario tiene rol visualizador, THE Sistema_Depositos SHALL ocultar los botones de cambio de estado del depósito.
6. THE Sistema_Depositos SHALL mostrar todos los textos de la sección de depósito en español.
7. WHEN el contrato no tiene depósito registrado, THE Sistema_Depositos SHALL ocultar la sección "Depósito de Garantía".
