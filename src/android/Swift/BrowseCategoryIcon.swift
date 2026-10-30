import SwiftUI

/// Bundled vector glyphs: the Android symbol bridge does not include health SF Symbols.
struct BrowseCategoryIcon: View {
    let category: BrowseCategory
    let color: Color
    var body: some View {
        Path { path in
            func line(_ points: [(Double, Double)]) {
                guard let first = points.first else { return }
                path.move(to: CGPoint(x: first.0, y: first.1))
                for point in points.dropFirst() { path.addLine(to: CGPoint(x: point.0, y: point.1)) }
            }
            func circle(_ x: Double, _ y: Double, _ radius: Double) {
                path.addEllipse(in: CGRect(x: x-radius, y: y-radius, width: radius*2, height: radius*2))
            }
            switch category {
            case .activity:
                line([(12,2),(18,10),(20,15),(18,20),(12,22),(6,20),(4,15),(8,7),(8,13),(12,8),(12,2)])
            case .body:
                circle(12,4,2); line([(4,9),(20,9)]); line([(12,7),(12,15),(7,22)]); line([(12,15),(17,22)])
            case .cycle:
                for index in 0..<8 {
                    let angle = Double(index) * .pi / 4
                    circle(12 + cos(angle)*8,12 + sin(angle)*8,1.3)
                }
            case .hearing:
                line([(5,10),(5,7),(8,3),(14,2),(19,6),(19,12),(15,16),(14,20),(11,22),(8,21)])
                line([(9,10),(9,7),(12,6),(15,8),(15,11),(11,14)])
            case .heart:
                line([(12,21),(3,12),(2,8),(4,4),(8,3),(12,7),(16,3),(20,4),(22,8),(21,12),(12,21)])
            case .medications:
                line([(3,12),(12,3),(16,2),(20,4),(22,8),(21,12),(12,21),(8,22),(4,20),(2,16),(3,12)])
                line([(7,8),(16,17)])
            case .wellbeing:
                line([(9,22),(9,18),(5,18),(5,13),(2,12),(5,7),(7,3),(14,2),(20,6),(21,12),(18,17),(18,22)])
                circle(12,9,3); line([(12,6),(12,12)]); line([(9,9),(15,9)])
            case .mobility:
                circle(14,3,2); line([(6,11),(10,8),(14,8),(18,12),(21,12)])
                line([(13,8),(10,15),(7,22)]); line([(10,15),(16,16),(17,22)])
            case .nutrition:
                line([(12,7),(6,5),(3,8),(2,13),(4,19),(8,22),(12,21),(16,22),(20,18),(22,12),(20,7),(16,5),(12,7),(12,3),(17,1)])
            case .respiratory:
                line([(12,2),(12,10),(8,14)]); line([(12,10),(16,14)])
                line([(9,6),(5,8),(2,18),(4,21),(9,19),(9,6)])
                line([(15,6),(19,8),(22,18),(20,21),(15,19),(15,6)])
            case .sleep:
                line([(2,6),(2,22)]); line([(22,12),(22,22)]); line([(2,18),(22,18),(22,12),(10,12),(10,18)])
                circle(6,13,2)
            case .symptoms:
                circle(9,4,2); line([(3,10),(9,8),(14,10)]); line([(9,8),(9,15),(5,22)]); line([(9,15),(13,22)])
                line([(20,7),(20,14)]); circle(20,18,0.6)
            case .vitals:
                line([(1,12),(6,12),(9,4),(13,21),(17,8),(19,12),(23,12)])
            case .bloodWork:
                line([(12,2),(4,13),(3,17),(5,21),(12,23),(19,21),(21,17),(20,13),(12,2)])
            case .vision:
                line([(1,12),(6,6),(12,4),(18,6),(23,12),(18,18),(12,20),(6,18),(1,12)]); circle(12,12,4)
            case .other:
                line([(9,2),(15,2),(15,9),(22,9),(22,15),(15,15),(15,22),(9,22),(9,15),(2,15),(2,9),(9,9),(9,2)])
            case .allergies:
                circle(12,12,5)
                for index in 0..<8 {
                    let angle = Double(index) * .pi / 4
                    line([(12+cos(angle)*7,12+sin(angle)*7),(12+cos(angle)*10,12+sin(angle)*10)])
                }
            case .notes:
                line([(4,2),(16,2),(21,7),(21,22),(4,22),(4,2)]); line([(16,2),(16,7),(21,7)])
                line([(8,11),(17,11)]); line([(8,15),(17,15)]); line([(8,19),(14,19)])
            case .conditions:
                line([(3,2),(3,10),(6,13),(10,13),(13,10),(13,2)])
                line([(8,13),(8,18),(11,22),(17,22),(20,18),(20,12)]); circle(20,9,3)
            case .immunizations:
                line([(4,20),(8,16),(6,14),(14,6),(18,10),(10,18),(8,16)])
                line([(12,4),(20,12)]); line([(16,8),(20,4)]); line([(18,2),(22,6)])
            case .procedures:
                line([(3,7),(21,7),(21,22),(3,22),(3,7)]); line([(8,7),(8,3),(16,3),(16,7)])
                line([(12,10),(12,19)]); line([(8,14),(16,14)])
            }
        }.stroke(color, style: StrokeStyle(lineWidth: 2, lineCap: .round, lineJoin: .round))
            .frame(width: 24, height: 24).scaleEffect(1.25).frame(width: 32, height: 32)
            .accessibilityLabel(Text(category.rawValue))
    }
}
