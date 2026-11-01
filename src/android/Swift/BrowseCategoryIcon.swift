import SwiftUI

/// Existing Google Material Icons PNG assets. See bundled MaterialIcons provenance and license.
struct BrowseCategoryIcon: View {
    let category: BrowseCategory
    let color: Color
    private var asset: String {
        switch category.symbol {
        case "flame.fill": "health_category_local_fire_department"
        case "figure.stand": "health_category_accessibility_new"
        case "circle.dotted": "health_category_autorenew"
        case "ear.fill": "health_category_hearing"
        case "heart.fill": "health_category_favorite"
        case "pills.fill": "health_category_medication"
        case "brain.head.profile": "health_category_psychology"
        case "figure.walk": "health_category_directions_walk"
        case "carrot.fill": "health_category_restaurant"
        case "lungs.fill": "health_category_air"
        case "bed.double.fill": "health_category_bed"
        case "figure": "health_category_sick"
        case "waveform.path.ecg": "health_category_monitor_heart"
        case "drop.fill": "health_category_bloodtype"
        case "eye.fill": "health_category_visibility"
        case "cross.fill": "health_category_add_circle"
        case "allergens": "health_category_flare"
        case "doc.text.fill": "health_category_description"
        case "stethoscope": "health_category_medical_services"
        case "syringe.fill": "health_category_vaccines"
        case "cross.case.fill": "health_category_local_hospital"
        default: "health_category_add_circle"
        }
    }
    var body: some View {
        Image(asset, bundle: .module).renderingMode(.template).resizable()
            .scaledToFit().foregroundStyle(color).frame(width: 32, height: 32)
            .accessibilityLabel(Text(category.title))
    }
}
