package com.propmanager.feature.scanner

import com.google.mlkit.vision.common.InputImage
import com.google.mlkit.vision.text.Text
import com.google.mlkit.vision.text.TextRecognition
import com.google.mlkit.vision.text.latin.TextRecognizerOptions
import kotlinx.coroutines.tasks.await
import javax.inject.Inject
import javax.inject.Singleton

data class CedulaOcrResult(
    val nombre: String?,
    val apellido: String?,
    val cedula: String?,
    val confidence: Float,
)

@Singleton
class CedulaOcrExtractor
    @Inject
    constructor() {
        private val recognizer = TextRecognition.getClient(TextRecognizerOptions.DEFAULT_OPTIONS)

        private val cedulaPattern = Regex("""(\d{3})-?(\d{7})-?(\d)""")
        private val nameLinePattern = Regex("""^[A-ZÁÉÍÓÚÑ][A-ZÁÉÍÓÚÑa-záéíóúñ\s]+$""")

        suspend fun extractFromImage(image: InputImage): CedulaOcrResult {
            val visionText = recognizer.process(image).await()
            return parseCedulaText(visionText)
        }

        internal fun parseCedulaText(text: Text): CedulaOcrResult {
            val allLines =
                text.textBlocks.flatMap { block ->
                    block.lines.map { it.text.trim() }
                }

            val cedula =
                allLines.firstNotNullOfOrNull { line ->
                    cedulaPattern.find(line)?.let { match ->
                        "${match.groupValues[1]}-${match.groupValues[2]}-${match.groupValues[3]}"
                    }
                }

            val nameLines =
                allLines.filter { line ->
                    nameLinePattern.matches(line) &&
                        !line.contains("REPUBLICA", ignoreCase = true) &&
                        !line.contains("DOMINICANA", ignoreCase = true) &&
                        !line.contains("CEDULA", ignoreCase = true) &&
                        !line.contains("IDENTIDAD", ignoreCase = true) &&
                        !line.contains("ELECTORAL", ignoreCase = true) &&
                        line.length in 2..50
                }

            val apellido = nameLines.getOrNull(0)
            val nombre = nameLines.getOrNull(1)

            val fieldsFound = listOfNotNull(cedula, nombre, apellido).size
            val confidence = fieldsFound / 3f

            return CedulaOcrResult(
                nombre = nombre,
                apellido = apellido,
                cedula = cedula,
                confidence = confidence,
            )
        }
    }
