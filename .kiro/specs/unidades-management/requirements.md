# Documento de Requisitos — Gestión de Unidades

## Introducción

Módulo de gestión de unidades (CRUD) para la aplicación de gestión inmobiliaria. Permite a gerentes y administradores crear, listar, actualizar y eliminar unidades dentro de una propiedad. Las unidades representan apartamentos, locales comerciales u otros espacios individuales dentro de un edificio multi-unidad. El módulo incluye endpoints REST anidados bajo propiedades, una pestaña dedicada en la vista de detalle de propiedad, seguimiento de gastos y mantenimiento a nivel de unidad, y métricas de ocupación en el listado de propiedades. Los visualizadores pueden consultar las unidades en modo solo lectura.

## Glosario

- **Sistema_Unidades**: Módulo backend y frontend que gestiona las unidades, incluyendo API REST, servicios, modelos y componentes de interfaz.
- **Unidad**: Registro que representa un espacio individual dentro de una propiedad (apartamento, local, oficina), con número identificador, precio, moneda y estado.
- **Propiedad**: Entidad padre que contiene una o más unidades. Cada unidad pertenece a exactamente una propiedad.
- **Estado_Unidad**: Valor que indica la disponibilidad de una unidad: `disponible`, `ocupada`, o `mantenimiento`.
- **Numero_Unidad**: Identificador alfanumérico de una unidad dentro de una propiedad. Debe ser único dentro de la misma propiedad.
- **Resumen_Ocupacion**: Datos agregados que muestran el conteo total de unidades y la tasa de ocupación de una propiedad.
- **WriteAccess**: Extractor de Actix-web que permite acceso a usuarios con rol `admin` o `gerente`.
- **AdminOnly**: Extractor de Actix-web que permite acceso exclusivo a usuarios con rol `admin`.
- **Claims**: Datos del usuario autenticado extraídos del token JWT (sub, email, rol, organizacion_id).

## Requisitos

### Requisito 1: Crear unidad dentro de una propiedad

**Historia de usuario:** Como gerente, quiero crear unidades dentro de una propiedad, para registrar los espacios individuales de un edificio multi-unidad.

#### Criterios de aceptación

1. WHEN un usuario con WriteAccess envía una solicitud de creación con propiedad_id (en la ruta), numero_unidad, precio, y moneda válidos, THE Sistema_Unidades SHALL crear una Unidad con estado `disponible` por defecto y devolver el registro creado con código HTTP 201.
2. WHEN un usuario con WriteAccess envía una solicitud de creación con un propiedad_id que no existe en la tabla de propiedades, THE Sistema_Unidades SHALL rechazar la solicitud con código HTTP 404 y un mensaje indicando que la propiedad no fue encontrada.
3. WHEN un usuario con WriteAccess envía una solicitud de creación con un numero_unidad que ya existe dentro de la misma propiedad, THE Sistema_Unidades SHALL rechazar la solicitud con código HTTP 409 y un mensaje indicando que el número de unidad ya existe en esta propiedad.
4. WHEN un usuario con WriteAccess envía una solicitud de creación sin el campo numero_unidad, THE Sistema_Unidades SHALL rechazar la solicitud con código HTTP 422 y un mensaje indicando que el número de unidad es requerido.
5. WHEN un usuario con WriteAccess envía una solicitud de creación con un valor de moneda diferente a `DOP` o `USD`, THE Sistema_Unidades SHALL rechazar la solicitud con código HTTP 422 y un mensaje indicando las monedas válidas.
6. WHEN un usuario con WriteAccess envía una solicitud de creación con un valor de estado que no es `disponible`, `ocupada`, o `mantenimiento`, THE Sistema_Unidades SHALL rechazar la solicitud con código HTTP 422 y un mensaje indicando los estados válidos.
7. WHEN un usuario con WriteAccess envía una solicitud de creación con un precio negativo, THE Sistema_Unidades SHALL rechazar la solicitud con código HTTP 422 y un mensaje indicando que el precio debe ser mayor o igual a cero.
8. WHEN un usuario con rol visualizador intenta crear una unidad, THE Sistema_Unidades SHALL rechazar la solicitud con código HTTP 403.

### Requisito 2: Listar y consultar unidades de una propiedad

**Historia de usuario:** Como gerente, quiero ver todas las unidades de una propiedad con su estado, para tener visibilidad del inventario de espacios disponibles.

#### Criterios de aceptación

1. WHEN un usuario autenticado solicita la lista de unidades de una propiedad existente sin filtros, THE Sistema_Unidades SHALL devolver una respuesta paginada con todas las unidades de esa propiedad ordenadas por numero_unidad ascendente.
2. WHEN un usuario autenticado solicita la lista de unidades con filtro de estado `disponible`, THE Sistema_Unidades SHALL devolver únicamente las unidades cuyo Estado_Unidad sea `disponible`.
3. WHEN un usuario autenticado solicita una unidad por su ID y la unidad existe, THE Sistema_Unidades SHALL devolver el detalle completo de la Unidad.
4. WHEN un usuario autenticado solicita una unidad por un ID que no existe, THE Sistema_Unidades SHALL devolver código HTTP 404 con un mensaje indicando que la unidad no fue encontrada.
5. WHEN un usuario autenticado solicita la lista de unidades de una propiedad que no existe, THE Sistema_Unidades SHALL devolver código HTTP 404 con un mensaje indicando que la propiedad no fue encontrada.

### Requisito 3: Actualizar unidad

**Historia de usuario:** Como gerente, quiero actualizar los datos de una unidad, para reflejar cambios en precio, estado o información descriptiva.

#### Criterios de aceptación

1. WHEN un usuario con WriteAccess envía una actualización con campos válidos para una unidad existente, THE Sistema_Unidades SHALL actualizar los campos proporcionados y devolver el registro actualizado con código HTTP 200.
2. WHEN un usuario con WriteAccess envía una actualización para una unidad que no existe, THE Sistema_Unidades SHALL devolver código HTTP 404.
3. WHEN un usuario con WriteAccess envía una actualización con un numero_unidad que ya existe en otra unidad de la misma propiedad, THE Sistema_Unidades SHALL rechazar la solicitud con código HTTP 409 y un mensaje indicando que el número de unidad ya existe en esta propiedad.
4. WHEN un usuario con WriteAccess envía una actualización con un valor de estado inválido, THE Sistema_Unidades SHALL rechazar la solicitud con código HTTP 422.
5. WHEN un usuario con WriteAccess envía una actualización con un valor de moneda inválido, THE Sistema_Unidades SHALL rechazar la solicitud con código HTTP 422.
6. WHEN un usuario con WriteAccess envía una actualización con un precio negativo, THE Sistema_Unidades SHALL rechazar la solicitud con código HTTP 422.
7. WHEN un usuario con rol visualizador intenta actualizar una unidad, THE Sistema_Unidades SHALL rechazar la solicitud con código HTTP 403.

### Requisito 4: Eliminar unidad

**Historia de usuario:** Como administrador, quiero eliminar unidades erróneas, para mantener los registros limpios.

#### Criterios de aceptación

1. WHEN un usuario con AdminOnly elimina una unidad existente, THE Sistema_Unidades SHALL eliminar la Unidad y devolver código HTTP 204.
2. WHEN un usuario con AdminOnly intenta eliminar una unidad que no existe, THE Sistema_Unidades SHALL devolver código HTTP 404.
3. WHEN un usuario con rol gerente intenta eliminar una unidad, THE Sistema_Unidades SHALL rechazar la solicitud con código HTTP 403.
4. WHEN un usuario con rol visualizador intenta eliminar una unidad, THE Sistema_Unidades SHALL rechazar la solicitud con código HTTP 403.

### Requisito 5: Validación de unicidad de numero_unidad

**Historia de usuario:** Como gerente, quiero que el sistema impida números de unidad duplicados dentro de una propiedad, para evitar confusión en la identificación de espacios.

#### Criterios de aceptación

1. THE Sistema_Unidades SHALL validar que el Numero_Unidad sea único dentro de la misma Propiedad antes de crear o actualizar una Unidad.
2. WHEN un usuario con WriteAccess intenta crear o actualizar una unidad con un Numero_Unidad que ya existe en otra unidad de la misma propiedad, THE Sistema_Unidades SHALL rechazar la operación con código HTTP 409 y un mensaje descriptivo en español.

### Requisito 6: Seguimiento de gastos a nivel de unidad

**Historia de usuario:** Como gerente, quiero filtrar gastos por unidad, para controlar los costos específicos de cada espacio.

#### Criterios de aceptación

1. WHEN un usuario autenticado solicita la lista de gastos con filtro de unidad_id, THE Sistema_Unidades SHALL devolver únicamente los gastos vinculados a esa unidad.
2. WHEN un usuario autenticado consulta el detalle de una unidad, THE Sistema_Unidades SHALL incluir el conteo de gastos asociados a esa unidad en la respuesta.

### Requisito 7: Seguimiento de mantenimiento a nivel de unidad

**Historia de usuario:** Como gerente, quiero filtrar solicitudes de mantenimiento por unidad, para dar seguimiento a las reparaciones de cada espacio.

#### Criterios de aceptación

1. WHEN un usuario autenticado solicita la lista de solicitudes de mantenimiento con filtro de unidad_id, THE Sistema_Unidades SHALL devolver únicamente las solicitudes vinculadas a esa unidad.
2. WHEN un usuario autenticado consulta el detalle de una unidad, THE Sistema_Unidades SHALL incluir el conteo de solicitudes de mantenimiento asociadas a esa unidad en la respuesta.

### Requisito 8: Resumen de ocupación en listado de propiedades

**Historia de usuario:** Como gerente, quiero ver el conteo de unidades y la tasa de ocupación en el listado de propiedades, para evaluar rápidamente el rendimiento de cada edificio.

#### Criterios de aceptación

1. WHEN un usuario autenticado solicita el listado de propiedades, THE Sistema_Unidades SHALL incluir en cada propiedad el conteo total de unidades y el conteo de unidades con estado `ocupada`.
2. WHEN un usuario autenticado solicita el detalle de una propiedad, THE Sistema_Unidades SHALL incluir el conteo total de unidades, el conteo de unidades ocupadas, y la tasa de ocupación como porcentaje.
3. WHEN una propiedad no tiene unidades registradas, THE Sistema_Unidades SHALL devolver conteo total de cero, conteo de ocupadas de cero, y tasa de ocupación de cero.

### Requisito 9: Registro de auditoría

**Historia de usuario:** Como administrador, quiero que todas las operaciones sobre unidades queden registradas en la auditoría, para tener trazabilidad de quién hizo qué cambio.

#### Criterios de aceptación

1. WHEN un usuario crea, actualiza, o elimina una Unidad, THE Sistema_Unidades SHALL registrar la operación en la tabla de registros de auditoría incluyendo el ID del usuario, la acción realizada, y la fecha.

### Requisito 10: Interfaz de usuario para gestión de unidades

**Historia de usuario:** Como gerente, quiero una pestaña de unidades en la vista de detalle de propiedad con todos los textos en español, para gestionar las unidades de forma visual.

#### Criterios de aceptación

1. THE Sistema_Unidades SHALL presentar una pestaña "Unidades" en la vista de detalle de propiedad con tabla paginada de unidades, filtro por estado, y botón para crear nueva unidad.
2. THE Sistema_Unidades SHALL presentar un formulario de creación/edición con campos para numero_unidad, piso (opcional), habitaciones (opcional), baños (opcional), área en m² (opcional), descripción (opcional), precio, moneda (selector DOP/USD), y estado (selector).
3. THE Sistema_Unidades SHALL mostrar todos los textos de la interfaz en español, incluyendo etiquetas, botones, mensajes de error, y placeholders.
4. THE Sistema_Unidades SHALL mostrar los precios con formato de moneda apropiado (DOP o USD) y precisión de dos decimales.
5. THE Sistema_Unidades SHALL mostrar indicadores visuales de estado diferenciados por color (disponible en verde, ocupada en azul, mantenimiento en naranja).
6. WHILE un usuario tiene rol visualizador, THE Sistema_Unidades SHALL ocultar los botones de crear, editar, y eliminar.
7. THE Sistema_Unidades SHALL mostrar el conteo de unidades y la tasa de ocupación en las tarjetas del listado de propiedades.
