import SwiftUI

/// Recorded GPS positions; no interpolation or invented map background.
struct RouteTraceView: View {
    let samples: [JSONValue]
    var body: some View {
        let points = samples.filter {
            guard let latitude = $0["latitude"].numberValue, let longitude = $0["longitude"].numberValue else { return false }
            return (-90...90).contains(latitude) && (-180...180).contains(longitude) && ($0["horizontal_accuracy"].numberValue ?? -1) >= 0
        }
        let latitudes = points.compactMap { $0["latitude"].numberValue }
        let longitudes = points.compactMap { $0["longitude"].numberValue }
        let latitude = ((latitudes.min() ?? 0) + (latitudes.max() ?? 0))/2
        let longitude = ((longitudes.min() ?? 0) + (longitudes.max() ?? 0))/2
        let correction = max(0.001, cos(latitude * .pi / 180))
        let spanX = max(0.0001, ((longitudes.max() ?? 0)-(longitudes.min() ?? 0))*correction)
        let spanY = max(0.0001, (latitudes.max() ?? 0)-(latitudes.min() ?? 0))
        GeometryReader { geometry in
            let scale = min((geometry.size.width-20)/spanX, (geometry.size.height-35)/spanY)
            ZStack {
                Path { path in
                    // Separate dots avoid connecting missing fixes or crossing the date line.
                    for point in points {
                        let x = geometry.size.width/2 + ((point["longitude"].numberValue ?? longitude)-longitude)*correction*scale
                        let y = geometry.size.height/2 - ((point["latitude"].numberValue ?? latitude)-latitude)*scale
                        path.addEllipse(in: CGRect(x: x-1.5, y: y-1.5, width: 3, height: 3))
                    }
                }.fill(Color.teal)
                VStack { Text("N ↑").font(.caption); Spacer(); Text("Recorded GPS positions").font(.caption).foregroundStyle(.secondary) }
            }
        }.accessibilityLabel("Recorded workout route, north up")
    }
}
