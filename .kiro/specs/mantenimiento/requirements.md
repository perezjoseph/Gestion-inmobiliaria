# Documento de Requisitos — Solicitudes de Mantenimiento

## Introducción

Sistema de solicitudes de mantenimiento para la aplicación de gestión inmobiliaria. Permite a gerentes y administradores registrar solicitudes de reparación vinculadas a una propiedad o unidad, dar seguimiento al estado (pendiente, en progreso, completado), asignar proveedores de servicio, y controlar los costos asociados. Los visualizadores pueden consultar las solicitudes en modo solo lectura.

## Glosario

- **Sistema_Mantenimiento**: Módulo backend y frontend que gestiona las solicitudes de mantenimiento, incluyendo API REST, servicios, entidades y páginas de interfaz.
- **Solicitud_Mantenimiento**: Registro que describe un trabajo de reparación o mantenimiento vinculado a una propiedad y opcionalmente a una unidad e inquilino.
- **Proveedor**: Nombre y datos de contacto de la persona o empresa asignada para ejecutar el trabajo de mantenimiento.
- **Estado_Solicitud**: Valor que indica la fase actual de una solicitud: `pendiente`, `en_progreso`, o `completado`.
- **Prioridad**: Nivel de urgencia de una solicitud: `baja`, `media`, `alta`, o `urgente`.
- **Nota_Mantenimiento**: Comentario de texto libre asociado a una solicitud, con autor y fecha de creación.
- **WriteAccess**: Extractor de Actix-web que permite acceso a usuarios con rol `admin` o `gerente`.
- **AdminOnly**: Extractor de Actix-web que permite acceso exclusivo a usuarios con rol `admin`.
- **Claims**: Datos del usuario autenticado extraídos del token JWT (sub, email, rol).

## Requisitos

### Requisito 1: Crear solicitud de mantenimiento

**Historia de usuario:** Como gerente, quiero registrar una solicitud de mantenimiento vinculada a una propiedad, para que el equipo pueda dar seguimiento a las reparaciones necesarias.

#### Criterios de aceptación

1. WHEN un usuario con WriteAccess envía una solicitud de creación con propiedad_id, titulo, y descripcion válidos, THE Sistema_Mantenimiento SHALL crear una Solicitud_Mantenimiento con estado `pendiente` y devolver el registro creado con código HTTP 201.
2. WHEN un usuario con WriteAccess envía una solicitud de creación con un propiedad_id que no existe en la tabla de propiedades, THE Sistema_Mantenimiento SHALL rechazar la solicitud con código HTTP 404 y un mensaje indicando que la propiedad no fue encontrada.
3. WHEN un usuario con WriteAccess envía una solicitud de creación sin el campo titulo, THE Sistema_Mantenimiento SHALL rechazar la solicitud con código HTTP 422 y un mensaje de validación indicando que el titulo es requerido.
4. WHEN un usuario con WriteAccess envía una solicitud de creación con un campo prioridad que no es `baja`, `media`, `alta`, o `urgente`, THE Sistema_Mantenimiento SHALL rechazar la solicitud con código HTTP 422 y un mensaje indicando los valores válidos de prioridad.
5. WHEN un usuario con WriteAccess envía una solicitud de creación sin especificar prioridad, THE Sistema_Mantenimiento SHALL asignar el valor `media` como prioridad por defecto.
6. WHEN un usuario con rol visualizador intenta crear una solicitud, THE Sistema_Mantenimiento SHALL rechazar la solicitud con código HTTP 403.

### Requisito 2: Listar y consultar solicitudes de mantenimiento

**Historia de usuario:** Como gerente, quiero ver todas las solicitudes de mantenimiento con filtros por estado, prioridad y propiedad, para poder priorizar el trabajo pendiente.

#### Criterios de aceptación

1. WHEN un usuario autenticado solicita la lista de solicitudes sin filtros, THE Sistema_Mantenimiento SHALL devolver una respuesta paginada con todas las solicitudes ordenadas por fecha de creación descendente.
2. WHEN un usuario autenticado solicita la lista con filtro de estado `en_progreso`, THE Sistema_Mantenimiento SHALL devolver únicamente las solicitudes cuyo Estado_Solicitud sea `en_progreso`.
3. WHEN un usuario autenticado solicita la lista con filtro de prioridad `urgente`, THE Sistema_Mantenimiento SHALL devolver únicamente las solicitudes cuya Prioridad sea `urgente`.
4. WHEN un usuario autenticado solicita la lista con filtro de propiedad_id, THE Sistema_Mantenimiento SHALL devolver únicamente las solicitudes vinculadas a esa propiedad.
5. WHEN un usuario autenticado solicita una solicitud por su ID y la solicitud existe, THE Sistema_Mantenimiento SHALL devolver el detalle completo de la Solicitud_Mantenimiento incluyendo las notas asociadas.
6. WHEN un usuario autenticado solicita una solicitud por un ID que no existe, THE Sistema_Mantenimiento SHALL devolver código HTTP 404 con un mensaje indicando que la solicitud no fue encontrada.

### Requisito 3: Actualizar solicitud de mantenimiento

**Historia de usuario:** Como gerente, quiero actualizar los datos de una solicitud de mantenimiento, para reflejar cambios en prioridad, asignación de proveedor, o descripción.

#### Criterios de aceptación

1. WHEN un usuario con WriteAccess envía una actualización con campos válidos para una solicitud existente, THE Sistema_Mantenimiento SHALL actualizar los campos proporcionados y devolver el registro actualizado con código HTTP 200.
2. WHEN un usuario con WriteAccess envía una actualización para una solicitud que no existe, THE Sistema_Mantenimiento SHALL devolver código HTTP 404.
3. WHEN un usuario con WriteAccess envía una actualización con un valor de prioridad inválido, THE Sistema_Mantenimiento SHALL rechazar la solicitud con código HTTP 422.
4. WHEN un usuario con rol visualizador intenta actualizar una solicitud, THE Sistema_Mantenimiento SHALL rechazar la solicitud con código HTTP 403.

### Requisito 4: Gestionar estado de la solicitud

**Historia de usuario:** Como gerente, quiero cambiar el estado de una solicitud de mantenimiento siguiendo un flujo definido, para que el equipo sepa en qué fase se encuentra cada trabajo.

#### Criterios de aceptación

1. WHEN un usuario con WriteAccess cambia el estado de una solicitud de `pendiente` a `en_progreso`, THE Sistema_Mantenimiento SHALL actualizar el Estado_Solicitud y registrar la fecha de inicio del trabajo.
2. WHEN un usuario con WriteAccess cambia el estado de una solicitud de `en_progreso` a `completado`, THE Sistema_Mantenimiento SHALL actualizar el Estado_Solicitud y registrar la fecha de finalización.
3. WHEN un usuario con WriteAccess intenta cambiar el estado de `pendiente` directamente a `completado`, THE Sistema_Mantenimiento SHALL rechazar la transición con código HTTP 422 y un mensaje indicando que la solicitud debe pasar por `en_progreso` antes de completarse.
4. WHEN un usuario con WriteAccess intenta cambiar el estado de `completado` a cualquier otro estado, THE Sistema_Mantenimiento SHALL rechazar la transición con código HTTP 422 y un mensaje indicando que las solicitudes completadas no pueden revertirse.

### Requisito 5: Asignar proveedor de servicio

**Historia de usuario:** Como gerente, quiero asignar un proveedor de servicio a una solicitud de mantenimiento, para saber quién es responsable de ejecutar la reparación.

#### Criterios de aceptación

1. WHEN un usuario con WriteAccess asigna un proveedor proporcionando nombre_proveedor a una solicitud existente, THE Sistema_Mantenimiento SHALL almacenar el nombre del Proveedor, teléfono de contacto opcional, y email opcional en la Solicitud_Mantenimiento.
2. WHEN un usuario con WriteAccess actualiza el proveedor asignado a una solicitud, THE Sistema_Mantenimiento SHALL reemplazar los datos del Proveedor anterior con los nuevos datos proporcionados.
3. WHEN un usuario con WriteAccess asigna un proveedor sin proporcionar nombre_proveedor, THE Sistema_Mantenimiento SHALL rechazar la solicitud con código HTTP 422 indicando que el nombre del proveedor es requerido.

### Requisito 6: Registrar y consultar costos de mantenimiento

**Historia de usuario:** Como gerente, quiero registrar el costo de una reparación con su moneda, para llevar control financiero del mantenimiento de las propiedades.

#### Criterios de aceptación

1. WHEN un usuario con WriteAccess registra un costo proporcionando monto y moneda (`DOP` o `USD`) en una solicitud existente, THE Sistema_Mantenimiento SHALL almacenar el costo con precisión decimal de dos dígitos.
2. WHEN un usuario con WriteAccess registra un costo con un valor de moneda diferente a `DOP` o `USD`, THE Sistema_Mantenimiento SHALL rechazar la solicitud con código HTTP 422 indicando las monedas válidas.
3. WHEN un usuario con WriteAccess registra un costo con un monto negativo, THE Sistema_Mantenimiento SHALL rechazar la solicitud con código HTTP 422 indicando que el monto debe ser mayor o igual a cero.
4. WHEN un usuario autenticado consulta una solicitud con costo registrado, THE Sistema_Mantenimiento SHALL incluir el monto y la moneda en la respuesta.

### Requisito 7: Agregar notas y comentarios

**Historia de usuario:** Como gerente, quiero agregar notas a una solicitud de mantenimiento, para documentar el progreso, comunicaciones con el proveedor, o detalles adicionales.

#### Criterios de aceptación

1. WHEN un usuario con WriteAccess agrega una nota con contenido de texto a una solicitud existente, THE Sistema_Mantenimiento SHALL crear una Nota_Mantenimiento vinculada a la solicitud con el ID del autor y la fecha de creación.
2. WHEN un usuario con WriteAccess intenta agregar una nota con contenido vacío, THE Sistema_Mantenimiento SHALL rechazar la solicitud con código HTTP 422 indicando que el contenido de la nota es requerido.
3. WHEN un usuario autenticado consulta el detalle de una solicitud, THE Sistema_Mantenimiento SHALL incluir todas las notas asociadas ordenadas por fecha de creación descendente.
4. WHEN un usuario con WriteAccess intenta agregar una nota a una solicitud que no existe, THE Sistema_Mantenimiento SHALL devolver código HTTP 404.

### Requisito 8: Eliminar solicitud de mantenimiento

**Historia de usuario:** Como administrador, quiero eliminar solicitudes de mantenimiento erróneas, para mantener los registros limpios.

#### Criterios de aceptación

1. WHEN un usuario con AdminOnly elimina una solicitud existente, THE Sistema_Mantenimiento SHALL eliminar la Solicitud_Mantenimiento y todas las Nota_Mantenimiento asociadas, y devolver código HTTP 204.
2. WHEN un usuario con AdminOnly intenta eliminar una solicitud que no existe, THE Sistema_Mantenimiento SHALL devolver código HTTP 404.
3. WHEN un usuario con rol gerente intenta eliminar una solicitud, THE Sistema_Mantenimiento SHALL rechazar la solicitud con código HTTP 403.
4. WHEN un usuario con rol visualizador intenta eliminar una solicitud, THE Sistema_Mantenimiento SHALL rechazar la solicitud con código HTTP 403.

### Requisito 9: Integración con propiedades e inquilinos

**Historia de usuario:** Como gerente, quiero vincular solicitudes de mantenimiento a propiedades, unidades e inquilinos existentes, para tener trazabilidad completa de cada reparación.

#### Criterios de aceptación

1. THE Sistema_Mantenimiento SHALL almacenar una referencia obligatoria a una propiedad (propiedad_id) en cada Solicitud_Mantenimiento.
2. THE Sistema_Mantenimiento SHALL almacenar una referencia opcional a una unidad (unidad_id) en cada Solicitud_Mantenimiento.
3. THE Sistema_Mantenimiento SHALL almacenar una referencia opcional a un inquilino (inquilino_id) en cada Solicitud_Mantenimiento.
4. WHEN un usuario con WriteAccess crea una solicitud con un unidad_id que no pertenece a la propiedad indicada en propiedad_id, THE Sistema_Mantenimiento SHALL rechazar la solicitud con código HTTP 422 indicando que la unidad no pertenece a la propiedad.
5. WHEN un usuario con WriteAccess crea una solicitud con un inquilino_id que no existe, THE Sistema_Mantenimiento SHALL rechazar la solicitud con código HTTP 404 indicando que el inquilino no fue encontrado.

### Requisito 10: Registro de auditoría

**Historia de usuario:** Como administrador, quiero que todas las operaciones de mantenimiento queden registradas en la auditoría, para tener trazabilidad de quién hizo qué cambio.

#### Criterios de aceptación

1. WHEN un usuario crea, actualiza, cambia estado, o elimina una Solicitud_Mantenimiento, THE Sistema_Mantenimiento SHALL registrar la operación en la tabla de registros de auditoría incluyendo el ID del usuario, la acción realizada, y la fecha.
2. WHEN un usuario agrega una nota a una solicitud, THE Sistema_Mantenimiento SHALL registrar la operación en la tabla de registros de auditoría.

### Requisito 11: Interfaz de usuario para mantenimiento

**Historia de usuario:** Como gerente, quiero una página de mantenimiento en la aplicación web con todos los textos en español, para gestionar las solicitudes de forma visual.

#### Criterios de aceptación

1. THE Sistema_Mantenimiento SHALL presentar una página de listado de solicitudes con tabla paginada, filtros por estado y prioridad, y botón para crear nueva solicitud.
2. THE Sistema_Mantenimiento SHALL presentar un formulario de creación con campos para propiedad (selector), unidad (selector filtrado por propiedad), inquilino (selector), titulo, descripcion, prioridad (selector), datos del proveedor, y costo.
3. THE Sistema_Mantenimiento SHALL presentar una vista de detalle que muestre toda la información de la solicitud, las notas asociadas, y permita cambiar el estado.
4. THE Sistema_Mantenimiento SHALL mostrar todos los textos de la interfaz en español, incluyendo etiquetas, botones, mensajes de error, y placeholders.
5. THE Sistema_Mantenimiento SHALL mostrar los costos con formato de moneda apropiado (DOP o USD) y las fechas en formato DD/MM/YYYY.
6. WHILE un usuario tiene rol visualizador, THE Sistema_Mantenimiento SHALL ocultar los botones de crear, editar, eliminar, y cambiar estado.
7. THE Sistema_Mantenimiento SHALL mostrar indicadores visuales de prioridad diferenciados por color (por ejemplo: urgente en rojo, alta en naranja, media en amarillo, baja en verde).
8. THE Sistema_Mantenimiento SHALL mostrar indicadores visuales de estado diferenciados (pendiente, en progreso, completado) con colores o badges distintivos.
