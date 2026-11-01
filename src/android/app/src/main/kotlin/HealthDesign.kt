package health.app

import androidx.compose.foundation.background
import androidx.compose.foundation.BorderStroke
import androidx.compose.foundation.Canvas
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.NavigationBarItemDefaults
import androidx.compose.material3.OutlinedTextFieldDefaults
import androidx.compose.material3.SegmentedButtonDefaults
import androidx.compose.material3.TopAppBarDefaults
import androidx.compose.material3.Typography
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.geometry.CornerRadius
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.Path
import androidx.compose.ui.graphics.StrokeCap
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import skip.ui.Material3ColorScheme
import skip.ui.View

/** Android presentation only. Shared Swift views retain navigation, data and form state. */
@Composable
internal fun HealthDesignTheme(content: @Composable () -> Unit) {
    MaterialTheme(typography = Typography(
        titleLarge = TextStyle(fontSize = 24.sp, lineHeight = 30.sp, fontWeight = FontWeight.Bold),
        titleMedium = TextStyle(fontSize = 17.sp, lineHeight = 22.sp, fontWeight = FontWeight.SemiBold),
        bodyLarge = TextStyle(fontSize = 17.sp, lineHeight = 24.sp),
        bodyMedium = TextStyle(fontSize = 15.sp, lineHeight = 21.sp),
        labelLarge = TextStyle(fontSize = 15.sp, lineHeight = 20.sp, fontWeight = FontWeight.Medium),
        labelMedium = TextStyle(fontSize = 12.sp, lineHeight = 16.sp, fontWeight = FontWeight.Medium),
        labelSmall = TextStyle(fontSize = 11.sp, lineHeight = 14.sp, fontWeight = FontWeight.Medium)
    )) {
        Material3ColorScheme(scheme = { base, dark ->
            val background = if (dark) Color.Black else Color(0xFFF2F2F7)
            val card = if (dark) Color(0xFF1C1C1E) else Color.White
            val text = if (dark) Color(0xFFF5F5F7) else Color(0xFF1C1C1E)
            val secondary = if (dark) Color(0xFF98989F) else Color(0xFF63636A)
            base.copy(
                primary = if (dark) Color(0xFF64D2C9) else Color(0xFF007F79),
                onPrimary = if (dark) Color.Black else Color.White,
                background = background, onBackground = text,
                surface = background, onSurface = text, surfaceTint = Color.Transparent,
                surfaceVariant = card, onSurfaceVariant = secondary,
                surfaceContainer = card, surfaceContainerLow = card,
                surfaceContainerHigh = card, surfaceContainerHighest = card,
                surfaceContainerLowest = background,
                outline = secondary.copy(alpha = 0.4f), outlineVariant = secondary.copy(alpha = 0.18f)
            )
        }, content = content)
    }
}

@OptIn(androidx.compose.material3.ExperimentalMaterial3Api::class)
internal fun View.healthDesign(): View = this
    .material3TopAppBar { options ->
        options.copy(
            // Apply to every NavigationStack, including sheets and foldable panes.
            // Inline titles otherwise use a pinned bar, permanently reducing the
            // scroll viewport. Search remains in the content below this moving bar.
            scrollBehavior = TopAppBarDefaults.enterAlwaysScrollBehavior(),
            preferLargeStyle = false,
            preferCenterAlignedStyle = true,
            colors = TopAppBarDefaults.topAppBarColors(
                containerColor = MaterialTheme.colorScheme.background,
                scrolledContainerColor = MaterialTheme.colorScheme.background,
                titleContentColor = MaterialTheme.colorScheme.onBackground,
                navigationIconContentColor = MaterialTheme.colorScheme.primary
            )
        )
    }
    .material3NavigationBar { options ->
        options.copy(
            modifier = options.modifier.padding(horizontal = 12.dp, vertical = 6.dp)
                .clip(RoundedCornerShape(28.dp)),
            containerColor = MaterialTheme.colorScheme.surfaceContainer,
            tonalElevation = 0.dp,
            itemIcon = { index -> HealthTabIcon(index) },
            itemColors = NavigationBarItemDefaults.colors(
                selectedIconColor = MaterialTheme.colorScheme.primary,
                selectedTextColor = MaterialTheme.colorScheme.primary,
                indicatorColor = MaterialTheme.colorScheme.primary.copy(alpha = 0.10f),
                unselectedIconColor = MaterialTheme.colorScheme.onSurfaceVariant,
                unselectedTextColor = MaterialTheme.colorScheme.onSurfaceVariant
            )
        )
    }
    .material3SegmentedButton { options ->
        options.copy(
            // Adjacent segments paint one continuous track. Only its outside ends
            // are rounded; the selected indicator is inset and independently rounded.
            modifier = options.modifier.background(
                MaterialTheme.colorScheme.onSurface.copy(alpha = 0.10f),
                RoundedCornerShape(
                    topStart = if (options.index == 0) 11.dp else 0.dp,
                    bottomStart = if (options.index == 0) 11.dp else 0.dp,
                    topEnd = if (options.index == options.count - 1) 11.dp else 0.dp,
                    bottomEnd = if (options.index == options.count - 1) 11.dp else 0.dp
                )
            ).padding(2.dp),
            shape = RoundedCornerShape(8.dp),
            border = BorderStroke(0.dp, Color.Transparent),
            icon = {},
            colors = SegmentedButtonDefaults.colors(
                activeContainerColor = MaterialTheme.colorScheme.onSurface.copy(alpha = 0.20f),
                activeContentColor = MaterialTheme.colorScheme.onSurface,
                inactiveContainerColor = Color.Transparent,
                inactiveContentColor = MaterialTheme.colorScheme.onSurfaceVariant
            )
        )
    }
    .material3TextField { options ->
        options.copy(
            shape = RoundedCornerShape(14.dp),
            colors = OutlinedTextFieldDefaults.colors(
                focusedContainerColor = MaterialTheme.colorScheme.surfaceContainer,
                unfocusedContainerColor = MaterialTheme.colorScheme.surfaceContainer,
                focusedBorderColor = MaterialTheme.colorScheme.primary,
                unfocusedBorderColor = Color.Transparent
            )
        )
    }

/** A coherent, bundled set avoids SF Symbol fallbacks to unrelated Android icons. */
@Composable
private fun HealthTabIcon(index: Int) {
    val color = androidx.compose.material3.LocalContentColor.current
    Canvas(Modifier.size(24.dp)) {
        val unit = size.width / 24f
        val stroke = 1.8f * unit
        fun line(x1: Float, y1: Float, x2: Float, y2: Float) = drawLine(
            color, Offset(x1 * unit, y1 * unit), Offset(x2 * unit, y2 * unit), stroke, StrokeCap.Round)
        when (index) {
            0 -> for (x in listOf(3f, 14f)) for (y in listOf(3f, 14f))
                drawRoundRect(color, Offset(x * unit, y * unit), Size(7 * unit, 7 * unit), CornerRadius(1.5f * unit))
            1 -> for (y in listOf(5f, 12f, 19f)) {
                drawCircle(color, 1.5f * unit, Offset(4 * unit, y * unit)); line(9f, y, 21f, y)
            }
            2 -> { line(3f, 3f, 3f, 21f); line(3f, 21f, 22f, 21f)
                line(6f, 16f, 11f, 10f); line(11f, 10f, 15f, 13f); line(15f, 13f, 21f, 5f) }
            3 -> {
                val path = Path().apply {
                    moveTo(12 * unit, 2 * unit); quadraticTo(13 * unit, 10 * unit, 21 * unit, 12 * unit)
                    quadraticTo(13 * unit, 14 * unit, 12 * unit, 22 * unit)
                    quadraticTo(11 * unit, 14 * unit, 3 * unit, 12 * unit)
                    quadraticTo(11 * unit, 10 * unit, 12 * unit, 2 * unit); close()
                }; drawPath(path, color)
            }
            else -> {
                drawRoundRect(color, Offset(3 * unit, 7 * unit), Size(18 * unit, 14 * unit), CornerRadius(3 * unit), style = Stroke(stroke))
                line(7f, 3f, 17f, 3f); line(5f, 5f, 19f, 5f); line(9f, 12f, 15f, 12f)
            }
        }
    }
}
