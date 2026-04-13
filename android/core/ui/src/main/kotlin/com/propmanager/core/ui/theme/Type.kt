package com.propmanager.core.ui.theme

import androidx.compose.material3.Typography
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.googlefonts.Font
import androidx.compose.ui.text.googlefonts.GoogleFont
import androidx.compose.ui.unit.TextUnit
import androidx.compose.ui.unit.sp
import com.propmanager.core.ui.R

val provider =
    GoogleFont.Provider(
        providerAuthority = "com.google.android.gms.fonts",
        providerPackage = "com.google.android.gms",
        certificates = R.array.com_google_android_gms_fonts_certs,
    )

val BitterFont =
    FontFamily(
        Font(
            googleFont = GoogleFont("Bitter"),
            fontProvider = provider,
            weight = FontWeight.SemiBold,
        ),
        Font(googleFont = GoogleFont("Bitter"), fontProvider = provider, weight = FontWeight.Bold),
    )

val SourceSans3Font =
    FontFamily(
        Font(
            googleFont = GoogleFont("Source Sans 3"),
            fontProvider = provider,
            weight = FontWeight.Normal,
        ),
        Font(
            googleFont = GoogleFont("Source Sans 3"),
            fontProvider = provider,
            weight = FontWeight.Medium,
        ),
        Font(
            googleFont = GoogleFont("Source Sans 3"),
            fontProvider = provider,
            weight = FontWeight.SemiBold,
        ),
        Font(
            googleFont = GoogleFont("Source Sans 3"),
            fontProvider = provider,
            weight = FontWeight.Bold,
        ),
    )

private fun textStyle(
    family: FontFamily,
    weight: FontWeight,
    size: TextUnit,
    lineHeight: TextUnit,
    letterSpacing: TextUnit,
) =
    TextStyle(
        fontFamily = family,
        fontWeight = weight,
        fontSize = size,
        lineHeight = lineHeight,
        letterSpacing = letterSpacing,
    )

val PropManagerTypography =
    Typography(
        displayLarge = textStyle(BitterFont, FontWeight.Normal, 57.sp, 64.sp, (-0.25).sp),
        displayMedium = textStyle(BitterFont, FontWeight.Normal, 45.sp, 52.sp, 0.sp),
        displaySmall = textStyle(BitterFont, FontWeight.Normal, 36.sp, 44.sp, 0.sp),
        headlineLarge = textStyle(BitterFont, FontWeight.SemiBold, 32.sp, 40.sp, 0.sp),
        headlineMedium = textStyle(BitterFont, FontWeight.SemiBold, 28.sp, 36.sp, 0.sp),
        headlineSmall = textStyle(BitterFont, FontWeight.SemiBold, 24.sp, 32.sp, 0.sp),
        titleLarge = textStyle(SourceSans3Font, FontWeight.Medium, 22.sp, 28.sp, 0.sp),
        titleMedium = textStyle(SourceSans3Font, FontWeight.Medium, 16.sp, 24.sp, 0.15.sp),
        titleSmall = textStyle(SourceSans3Font, FontWeight.Medium, 14.sp, 20.sp, 0.1.sp),
        bodyLarge = textStyle(SourceSans3Font, FontWeight.Normal, 16.sp, 24.sp, 0.5.sp),
        bodyMedium = textStyle(SourceSans3Font, FontWeight.Normal, 14.sp, 20.sp, 0.25.sp),
        bodySmall = textStyle(SourceSans3Font, FontWeight.Normal, 12.sp, 16.sp, 0.4.sp),
        labelLarge = textStyle(SourceSans3Font, FontWeight.Medium, 14.sp, 20.sp, 0.1.sp),
        labelMedium = textStyle(SourceSans3Font, FontWeight.Medium, 12.sp, 16.sp, 0.5.sp),
        labelSmall = textStyle(SourceSans3Font, FontWeight.Medium, 11.sp, 16.sp, 0.5.sp),
    )
