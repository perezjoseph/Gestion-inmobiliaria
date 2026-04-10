package com.propmanager.feature.scanner

import com.google.mlkit.vision.common.InputImage
import com.google.mlkit.vision.text.Text
import com.google.mlkit.vision.text.TextRecognition
import com.google.mlkit.vision.text.latin.TextRecognizerOptions
import java.math.BigDecimal
import java.time.LocalDate
import javax.inject.Inject
import javax.inject.Singleton
import kotlinx.coroutines.tasks.await

data class ReceiptOcrResult(
    val monto: BigDecimal?,
    val fecha: LocalDate?,
    val proveedor: String?,
    val numeroFactura: String?,
    val confidence: Float,
)

@Singleton
class ReceiptOcrExtractor @Inject constructor() {
    private val recognizer by lazy {
        TextRecognition.getClient(TextRecognizerOptions.DEFAULT_OPTIONS)
    }

    private val montoPattern = Regex("""(?:RD\$|US\$|\$)\s*([\d,]+\.?\d*)""")
    private val montoPlainPattern =
        Regex("""(?:total|monto|amount)\s*:?\s*\$?\s*([\d,]+\.?\d*)""", RegexOption.IGNORE_CASE)
    private val datePatterns =
        listOf(
            Regex("""(\d{2})/(\d{2})/(\d{4})"""),
            Regex("""(\d{2})-(\d{2})-(\d{4})"""),
            Regex("""(\d{4})-(\d{2})-(\d{2})"""),
        )
    private val facturaPattern =
        Regex("""(?:factura|invoice|no\.?|num\.?|#)\s*:?\s*([A-Z0-9-]+)""", RegexOption.IGNORE_CASE)

    suspend fun extractFromImage(image: InputImage): ReceiptOcrResult {
        val visionText = recognizer.process(image).await()
        return parseReceiptText(visionText)
    }

    internal fun parseReceiptText(text: Text): ReceiptOcrResult {
        val allLines = text.textBlocks.flatMap { block -> block.lines.map { it.text.trim() } }
        return parseReceiptLines(allLines)
    }

    internal fun parseReceiptLines(allLines: List<String>): ReceiptOcrResult {
        val fullText = allLines.joinToString("\n")

        val monto = extractMonto(fullText)
        val fecha = extractFecha(fullText)
        val proveedor = extractProveedor(allLines)
        val numeroFactura = extractNumeroFactura(fullText)

        val fieldsFound = listOfNotNull(monto, fecha, proveedor, numeroFactura).size
        val confidence = fieldsFound / 4f

        return ReceiptOcrResult(
            monto = monto,
            fecha = fecha,
            proveedor = proveedor,
            numeroFactura = numeroFactura,
            confidence = confidence,
        )
    }

    private fun extractMonto(text: String): BigDecimal? {
        val match = montoPattern.find(text) ?: montoPlainPattern.find(text) ?: return null
        val raw = match.groupValues[1].replace(",", "")
        return runCatching { BigDecimal(raw) }.getOrNull()
    }

    private fun extractFecha(text: String): LocalDate? {
        for (pattern in datePatterns) {
            val match = pattern.find(text) ?: continue
            return runCatching {
                    val groups = match.groupValues
                    if (groups[1].length == 4) {
                        LocalDate.of(groups[1].toInt(), groups[2].toInt(), groups[3].toInt())
                    } else {
                        LocalDate.of(groups[3].toInt(), groups[2].toInt(), groups[1].toInt())
                    }
                }
                .getOrNull()
        }
        return null
    }

    private fun extractProveedor(lines: List<String>): String? =
        lines.firstOrNull { line ->
            line.length in 3..60 &&
                !line.contains("$") &&
                !line.matches(Regex(""".*\d{2}/\d{2}/\d{4}.*""")) &&
                !line.matches(Regex("""^\d+.*""")) &&
                !line.contains("factura", ignoreCase = true) &&
                !line.contains("total", ignoreCase = true) &&
                !line.contains("rnc", ignoreCase = true)
        }

    private fun extractNumeroFactura(text: String): String? =
        facturaPattern.find(text)?.groupValues?.get(1)
}
