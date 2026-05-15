# Documento de Requisitos — Sistema de Notificaciones

## Introducción

El sistema actual de notificaciones es mínimo: un solo endpoint (GET /api/v1/notificaciones/pagos-vencidos) que consulta pagos vencidos directamente de la tabla de pagos. No existe persistencia de notificaciones, no hay indicador visual de notificaciones no leídas, y no se cubren otros eventos del sistema como contratos por vencer, documentos vencidos, o cambios en solicitudes de mantenimiento.

Este módulo introduce un sistema de notificaciones completo con una tabla `notificaciones` para persistir las notificaciones generadas, una campana de notificaciones en la barra de navegación con contador de no leídas, una página de listado con funcionalidad de marcar como leída, y una función de servicio que genera notificaciones a partir de condiciones del negocio (pagos vencidos, contratos por vencer, documentos por vencer, cambios de estado en mantenimiento). Todos los textos están en español.

## Glosario

- **Sistema_Notificaciones**: Módulo backend y frontend que gestiona la creación, almacenamiento, consulta y marcado de notificaciones, incluyendo API REST, servicios, entidades y componentes de interfaz.
- **Notificacion**: Registro persistido en la tabla `notificaciones` que representa un aviso generado por el sistema para un usuario específico, con tipo, título, mensaje, estado de lectura, y referencia a la entidad que lo originó.
- **Tipo_Notificacion**: Categoría de la notificación. Valores válidos: `pago_vencido`, `contrato_por_vencer`, `documento_vencido`, `mantenimiento_actualizado`.
- **Campana_Notificaciones**: Componente visual en la barra de navegación que muestra un ícono de campana con un badge indicando la cantidad de notificaciones no leídas del usuario autenticado.
- **Generador_Notificaciones**: Función de servicio que evalúa las condiciones del negocio y crea registros de Notificacion para los usuarios de la organización. Se invoca desde endpoints o manualmente, sin scheduler de fondo.
- **WriteAccess**: Extractor de Actix-web que permite acceso a usuarios con rol `admin` o `gerente`.
- **Claims**: Datos del usuario autenticado extraídos del token JWT (sub, email, rol, organizacion_id).

## Requisitos

### Requisito 1: Entidad y almacenamiento de notificaciones

**Historia de usuario:** Como gerente, quiero que las notificaciones del sistema se almacenen de forma persistente, para poder consultarlas en cualquier momento y no perder avisos importantes.

#### Criterios de aceptación

1. THE Sistema_Notificaciones SHALL almacenar cada Notificacion con los campos: id (UUID), tipo (Tipo_Notificacion), titulo (texto), mensaje (texto), leida (booleano, por defecto falso), entity_type (texto), entity_id (UUID), usuario_id (UUID referencia a usuarios), organizacion_id (UUID referencia a organizaciones), y created_at (timestamp con zona horaria).
2. THE Sistema_Notificaciones SHALL validar que el campo tipo contenga exclusivamente uno de los valores: `pago_vencido`, `contrato_por_vencer`, `documento_vencido`, `mantenimiento_actualizado`.
3. THE Sistema_Notificaciones SHALL crear cada Notificacion con el campo leida en valor falso por defecto.
4. THE Sistema_Notificaciones SHALL almacenar en entity_type el nombre de la entidad relacionada (pago, contrato, documento, solicitud_mantenimiento) y en entity_id el identificador UUID de esa entidad.

### Requisito 2: Listar notificaciones del usuario

**Historia de usuario:** Como usuario autenticado, quiero ver mis notificaciones ordenadas por fecha, para estar al tanto de los eventos importantes del sistema.

#### Criterios de aceptación

1. WHEN un usuario autenticado solicita la lista de notificaciones, THE Sistema_Notificaciones SHALL devolver una respuesta paginada con las notificaciones del usuario ordenadas por created_at descendente.
2. WHEN un usuario autenticado solicita la lista con filtro de leida igual a falso, THE Sistema_Notificaciones SHALL devolver únicamente las notificaciones no leídas del usuario.
3. WHEN un usuario autenticado solicita la lista con filtro de tipo, THE Sistema_Notificaciones SHALL devolver únicamente las notificaciones del usuario cuyo Tipo_Notificacion coincida con el filtro.
4. THE Sistema_Notificaciones SHALL devolver únicamente las notificaciones que pertenecen al usuario autenticado, identificado por el campo sub del token JWT.

### Requisito 3: Obtener conteo de notificaciones no leídas

**Historia de usuario:** Como usuario autenticado, quiero ver cuántas notificaciones no leídas tengo, para saber si hay avisos pendientes sin necesidad de abrir la lista completa.

#### Criterios de aceptación

1. WHEN un usuario autenticado solicita el conteo de notificaciones no leídas, THE Sistema_Notificaciones SHALL devolver un número entero representando la cantidad de notificaciones del usuario donde leida es falso.
2. THE Sistema_Notificaciones SHALL contar únicamente las notificaciones que pertenecen al usuario autenticado.

### Requisito 4: Marcar notificación como leída

**Historia de usuario:** Como usuario autenticado, quiero marcar una notificación individual como leída, para indicar que ya la revisé.

#### Criterios de aceptación

1. WHEN un usuario autenticado marca una notificación existente como leída, THE Sistema_Notificaciones SHALL actualizar el campo leida a verdadero y devolver la notificación actualizada.
2. WHEN un usuario autenticado intenta marcar como leída una notificación que no existe, THE Sistema_Notificaciones SHALL devolver código HTTP 404 con un mensaje indicando que la notificación no fue encontrada.
3. WHEN un usuario autenticado intenta marcar como leída una notificación que pertenece a otro usuario, THE Sistema_Notificaciones SHALL devolver código HTTP 404 con un mensaje indicando que la notificación no fue encontrada.
4. WHEN un usuario autenticado marca como leída una notificación que ya está marcada como leída, THE Sistema_Notificaciones SHALL devolver la notificación sin cambios con código HTTP 200.

### Requisito 5: Marcar todas las notificaciones como leídas

**Historia de usuario:** Como usuario autenticado, quiero marcar todas mis notificaciones como leídas de una sola vez, para limpiar el contador de pendientes rápidamente.

#### Criterios de aceptación

1. WHEN un usuario autenticado solicita marcar todas las notificaciones como leídas, THE Sistema_Notificaciones SHALL actualizar el campo leida a verdadero en todas las notificaciones del usuario donde leida es falso.
2. THE Sistema_Notificaciones SHALL devolver la cantidad de notificaciones actualizadas.
3. WHEN un usuario autenticado solicita marcar todas como leídas y no tiene notificaciones no leídas, THE Sistema_Notificaciones SHALL devolver cero como cantidad de notificaciones actualizadas.

### Requisito 6: Generación de notificaciones por pagos vencidos

**Historia de usuario:** Como gerente, quiero recibir notificaciones cuando hay pagos vencidos, para tomar acción de cobro oportuna.

#### Criterios de aceptación

1. WHEN el Generador_Notificaciones es invocado, THE Sistema_Notificaciones SHALL crear una Notificacion de tipo `pago_vencido` por cada pago con estado `pendiente` y fecha_vencimiento anterior a la fecha actual, para cada usuario de la organización correspondiente.
2. THE Sistema_Notificaciones SHALL incluir en el titulo de la notificación el nombre de la propiedad y en el mensaje el monto, moneda, y días de vencimiento del pago.
3. THE Sistema_Notificaciones SHALL evitar crear notificaciones duplicadas verificando que no exista una Notificacion con el mismo tipo, entity_type, entity_id, y usuario_id antes de crear una nueva.

### Requisito 7: Generación de notificaciones por contratos por vencer

**Historia de usuario:** Como gerente, quiero recibir notificaciones cuando un contrato está próximo a vencer, para gestionar renovaciones a tiempo.

#### Criterios de aceptación

1. WHEN el Generador_Notificaciones es invocado, THE Sistema_Notificaciones SHALL crear una Notificacion de tipo `contrato_por_vencer` por cada contrato con estado `activo` y fecha_fin dentro de los próximos 30 días, para cada usuario de la organización correspondiente.
2. THE Sistema_Notificaciones SHALL incluir en el titulo de la notificación la referencia a la propiedad y en el mensaje la fecha de vencimiento del contrato y los días restantes.
3. THE Sistema_Notificaciones SHALL evitar crear notificaciones duplicadas verificando que no exista una Notificacion con el mismo tipo, entity_type, entity_id, y usuario_id antes de crear una nueva.

### Requisito 8: Generación de notificaciones por documentos vencidos

**Historia de usuario:** Como gerente, quiero recibir notificaciones cuando un documento está próximo a vencer o ya venció, para mantener la documentación al día.

#### Criterios de aceptación

1. WHEN el Generador_Notificaciones es invocado, THE Sistema_Notificaciones SHALL crear una Notificacion de tipo `documento_vencido` por cada documento con fecha_vencimiento dentro de los próximos 30 días o ya vencido, para cada usuario de la organización del documento.
2. THE Sistema_Notificaciones SHALL incluir en el titulo de la notificación el nombre del archivo y en el mensaje la fecha de vencimiento del documento.
3. THE Sistema_Notificaciones SHALL evitar crear notificaciones duplicadas verificando que no exista una Notificacion con el mismo tipo, entity_type, entity_id, y usuario_id antes de crear una nueva.

### Requisito 9: Generación de notificaciones por cambios en mantenimiento

**Historia de usuario:** Como gerente, quiero recibir notificaciones cuando cambia el estado de una solicitud de mantenimiento, para estar al tanto del progreso de las reparaciones.

#### Criterios de aceptación

1. WHEN el estado de una Solicitud_Mantenimiento cambia, THE Sistema_Notificaciones SHALL crear una Notificacion de tipo `mantenimiento_actualizado` para cada usuario de la organización correspondiente.
2. THE Sistema_Notificaciones SHALL incluir en el titulo de la notificación el título de la solicitud de mantenimiento y en el mensaje el estado anterior y el nuevo estado.
3. THE Sistema_Notificaciones SHALL registrar en entity_type el valor `solicitud_mantenimiento` y en entity_id el identificador de la solicitud.

### Requisito 10: Endpoint para disparar generación de notificaciones

**Historia de usuario:** Como administrador, quiero poder disparar manualmente la generación de notificaciones, para asegurar que el sistema detecte todas las condiciones pendientes.

#### Criterios de aceptación

1. WHEN un usuario con WriteAccess invoca el endpoint de generación, THE Sistema_Notificaciones SHALL ejecutar el Generador_Notificaciones para los tipos pago_vencido, contrato_por_vencer, y documento_vencido.
2. THE Sistema_Notificaciones SHALL devolver la cantidad de notificaciones generadas por cada tipo.
3. WHEN un usuario con rol visualizador intenta invocar el endpoint de generación, THE Sistema_Notificaciones SHALL rechazar la solicitud con código HTTP 403.

### Requisito 11: Campana de notificaciones en la barra de navegación

**Historia de usuario:** Como usuario autenticado, quiero ver un ícono de campana en la barra de navegación con el conteo de notificaciones no leídas, para saber de un vistazo si tengo avisos pendientes.

#### Criterios de aceptación

1. THE Sistema_Notificaciones SHALL mostrar un ícono de campana en la barra de navegación para usuarios autenticados.
2. WHILE el usuario tiene notificaciones no leídas, THE Sistema_Notificaciones SHALL mostrar un badge numérico sobre el ícono de campana indicando la cantidad de notificaciones no leídas.
3. WHILE el usuario no tiene notificaciones no leídas, THE Sistema_Notificaciones SHALL mostrar el ícono de campana sin badge.
4. WHEN el usuario hace clic en el ícono de campana, THE Sistema_Notificaciones SHALL navegar a la página de notificaciones.
5. THE Sistema_Notificaciones SHALL consultar el conteo de notificaciones no leídas al montar el componente de la barra de navegación.

### Requisito 12: Página de listado de notificaciones

**Historia de usuario:** Como usuario autenticado, quiero una página donde pueda ver todas mis notificaciones con opciones para filtrar y marcar como leídas, para gestionar mis avisos de forma organizada.

#### Criterios de aceptación

1. THE Sistema_Notificaciones SHALL presentar una página de listado de notificaciones con tabla paginada mostrando tipo, título, mensaje, estado de lectura, y fecha de creación.
2. THE Sistema_Notificaciones SHALL mostrar un botón "Marcar todas como leídas" que invoque el endpoint correspondiente y actualice la vista.
3. THE Sistema_Notificaciones SHALL permitir marcar una notificación individual como leída haciendo clic en un botón o enlace en la fila correspondiente.
4. THE Sistema_Notificaciones SHALL diferenciar visualmente las notificaciones no leídas de las leídas mediante estilo de fondo o tipografía.
5. THE Sistema_Notificaciones SHALL mostrar filtros por tipo de notificación y por estado de lectura.
6. THE Sistema_Notificaciones SHALL mostrar todos los textos de la interfaz en español.
7. THE Sistema_Notificaciones SHALL mostrar las fechas en formato DD/MM/YYYY.
8. THE Sistema_Notificaciones SHALL mostrar indicadores visuales diferenciados por tipo de notificación con íconos o colores distintivos.
