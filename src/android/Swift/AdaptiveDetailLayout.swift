import SwiftUI

/// Both panes keep their identity when the display folds or rotates.
struct AdaptiveDetailLayout<Sidebar: View, Detail: View>: View {
    let selected: Bool
    @ViewBuilder let sidebar: Sidebar
    @ViewBuilder let detail: Detail
    var body: some View {
        GeometryReader { geometry in
            let wide = geometry.size.width >= 700
            let sideWidth = wide ? min(340, geometry.size.width * 0.38) : selected ? 0 : geometry.size.width
            HStack(spacing: 0) {
                sidebar.frame(width: sideWidth).clipped().allowsHitTesting(wide || !selected).accessibilityHidden(!wide && selected)
                detail.frame(width: max(0, geometry.size.width - sideWidth))
                    .clipped().allowsHitTesting(wide || selected)
                    .accessibilityHidden(!wide && !selected)
            }
        }
    }
}
