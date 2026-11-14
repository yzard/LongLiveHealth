import SwiftUI

/// The same card and detail hierarchy is used by both native clients.
struct InsightDashboardCard: View {
    let model: AppModel
    let panel: JSONValue
    let response: JSONValue
    let expanded: Bool
    private var kind: String { panel["id"].stringValue }
    private var tint: Color {
        switch kind {
        case "sleep": .indigo
        case "readiness": .teal
        case "vitals": .blue
        case "heart", "cardiovascular": .pink
        default: .orange
        }
    }
    private var evidence: [JSONValue] { panel["evidence"].arrayValue }
    private var first: JSONValue { evidence.first ?? .null }

    var body: some View {
        VStack(alignment: .leading, spacing: 18) {
            HStack {
                InsightSymbol(symbol: panel["symbol"].stringValue, color: tint)
                Text(panel["title"].stringValue).font(.headline)
                Spacer()
                if !expanded { Image(systemName: "chevron.right").font(.caption.bold()) }
            }.foregroundStyle(tint)
            visual
            Text(panel["summary"].stringValue).font(.subheadline).foregroundStyle(.secondary)
                .fixedSize(horizontal: false, vertical: true)
        }
        .frame(maxWidth: .infinity, alignment: .leading)
        .padding(20)
        .background(tint.opacity(0.09), in: RoundedRectangle(cornerRadius: 26))
        .overlay { RoundedRectangle(cornerRadius: 26).stroke(tint.opacity(0.12), lineWidth: 1) }
        .foregroundStyle(.primary)
    }

    @ViewBuilder private var visual: some View {
        switch kind {
        case "sleep":
            let session = panel["session"]
            if let seconds = session["asleep_seconds"].numberValue {
                HStack(spacing: 20) {
                    VStack(alignment: .leading, spacing: 6) {
                        Text(duration(seconds)).font(.system(size: 34, weight: .bold)).minimumScaleFactor(0.7)
                        Text("Recorded sleep").font(.subheadline).foregroundStyle(.secondary)
                        dated(session["end_at"])
                    }
                    Spacer(minLength: 0)
                    if let goal = panel["goal_minutes"].numberValue, goal > 0 {
                        ZStack {
                            Circle().stroke(tint.opacity(0.18), lineWidth: 9)
                            Circle().trim(from: 0, to: min(1, seconds / (goal * 60)))
                                .stroke(tint, style: StrokeStyle(lineWidth: 9, lineCap: .round)).rotationEffect(.degrees(-90))
                            Text("Goal\n\(duration(goal * 60))").font(.caption.weight(.semibold)).multilineTextAlignment(.center)
                        }.frame(width: 88, height: 88).padding(5)
                            .accessibilityLabel("Recorded sleep \(duration(seconds)); personal goal \(duration(goal * 60))")
                    }
                }
                InsightSleepStages(session: session).frame(height: expanded ? 140 : 88)
            } else {
                Text("No complete recent session").font(.title2.bold())
                Text("Sync sleep stages to see recorded sleep here.").font(.subheadline)
            }
        case "readiness":
            HStack(alignment: .top, spacing: 10) {
                readinessFactor("Sleep", kinds: ["sleep"], symbol: "bed.double.fill")
                readinessFactor("Vitals", kinds: ["resting_heart_rate", "hrv_sdnn", "hrv_rmssd"], symbol: "heart.fill")
                readinessFactor("Activity", kinds: ["workout", "active_energy"], symbol: "figure.walk")
            }
            Text("Recovery review").font(.title2.bold())
            Text("Based on changes from your own baseline").font(.caption).foregroundStyle(.secondary)
        case "vitals":
            let values = expanded ? evidence : evidence.enumerated().filter { index, item in !evidence.prefix(index).contains { $0["metric_id"] == item["metric_id"] } }.map { $0.element }
            if values.isEmpty { Text("More readings needed").font(.title2.bold()) }
            else {
                ForEach(Array(values.enumerated()), id: \.offset) { _, value in
                    InsightVitalReading(value: value, tint: tint)
                }
            }
        case "cardiovascular":
            let cardiac = response["cardiovascular"]
            if let risk = cardiac["risk_percent"].numberValue {
                Text("\(risk.formatted(.number.precision(.fractionLength(1))))%")
                    .font(.system(size: 44, weight: .bold)).foregroundStyle(tint)
                Text("10-year total CVD risk").font(.subheadline)
            } else {
                Text("Complete your risk picture").font(.title2.bold())
                Text("\(cardiac["missing"].arrayValue.count) inputs need review").font(.subheadline).foregroundStyle(.secondary)
            }
            let finding = cardiac["findings"].arrayValue.first
            if let finding {
                HStack(alignment: .top) {
                    InsightSymbol(symbol: "exclamationmark.circle.fill", color: .orange)
                    VStack(alignment: .leading, spacing: 4) {
                        Text(finding["title"].stringValue).font(.headline)
                        Text(finding["text"].stringValue).font(.subheadline)
                    }
                }
            }
            HStack(spacing: 16) {
                HStack(spacing: 6) { InsightSymbol(symbol: "heart.fill", color: .secondary); Text("Blood pressure") }
                HStack(spacing: 6) { InsightSymbol(symbol: "drop.fill", color: .secondary); Text("Blood work") }
            }.font(.caption).foregroundStyle(.secondary)
        case "heart":
            if let reading = evidence.first(where: { $0["metric_id"] == .string("heart_rate") }) ?? evidence.first, let value = reading["latest"].numberValue {
                HStack(spacing: 22) {
                    ZStack {
                        Image(systemName: MobileSymbol.name("heart.fill")).resizable().scaledToFit().foregroundStyle(tint)
                        VStack(spacing: 0) {
                            Text(value.formatted(.number.precision(.fractionLength(0)))).font(.system(size: expanded ? 46 : 32, weight: .bold))
                            Text(HistoricalValue.displayUnit(reading["unit"].stringValue, kind: reading["metric_id"].stringValue)).font(.caption.bold())
                        }.foregroundStyle(.white)
                    }.frame(width: expanded ? 180 : 130, height: expanded ? 160 : 116)
                    VStack(alignment: .leading, spacing: 8) {
                        Text(reading["title"].stringValue).font(.headline)
                        dated(reading["latest_at"])
                        Text("Daily reading").font(.caption).foregroundStyle(.secondary)
                    }
                }
                InsightSparkline(series: reading["series"].arrayValue, tint: tint).frame(height: expanded ? 150 : 70)
            } else { Text("More heart readings needed").font(.title2.bold()) }
        default:
            if let value = first["latest"].numberValue {
                HStack(alignment: .firstTextBaseline, spacing: 6) {
                    Text(first["unit"] == .string("seconds") ? duration(value) : value.formatted(.number.precision(.fractionLength(0...1))))
                        .font(.system(size: 38, weight: .bold)).foregroundStyle(tint)
                    if first["unit"] != .string("seconds") {
                        Text(HistoricalValue.displayUnit(first["unit"].stringValue, kind: first["metric_id"].stringValue)).font(.subheadline)
                    }
                }
                Text(first["title"].stringValue).font(.headline)
                dated(first["latest_at"])
                InsightSparkline(series: first["series"].arrayValue, tint: tint).frame(height: expanded ? 160 : 74)
            } else { Text("More readings needed").font(.title2.bold()) }
        }
    }
    @ViewBuilder private func dated(_ value: JSONValue) -> some View {
        if let at = value.numberValue {
            Text(Date(timeIntervalSince1970: at).formatted(date: .abbreviated, time: .omitted))
                .font(.caption).foregroundStyle(.secondary)
        }
    }
    private func readinessFactor(_ title: String, kinds: [String], symbol: String) -> some View {
        let signals = evidence.filter { kinds.contains($0["metric_id"].stringValue) && $0["cue"] != .string("insufficient_comparable_days") }
        let changed = signals.contains { $0["cue"] != .string("no_review_cue") }
        return VStack(spacing: 10) {
            InsightSymbol(symbol: symbol, color: tint)
            Text(title).font(.subheadline.weight(.semibold))
            Text(signals.isEmpty ? "Need data" : changed ? "Review" : "No change cue")
                .font(.caption).foregroundStyle(.secondary).multilineTextAlignment(.center)
        }.frame(maxWidth: .infinity, minHeight: 105).padding(8)
            .background(tint.opacity(0.08), in: RoundedRectangle(cornerRadius: 18))
    }
    private func duration(_ seconds: Double) -> String {
        let minutes = Int(seconds / 60)
        return "\(minutes / 60)h \(minutes % 60)m"
    }
}

struct InsightSparkline: View {
    let series: [JSONValue]
    let tint: Color
    var body: some View {
        GeometryReader { geometry in
            let points = series.flatMap { $0["points"].arrayValue }.filter { $0["at"].numberValue != nil && $0["value"].numberValue != nil }
                .sorted { ($0["at"].numberValue ?? 0) < ($1["at"].numberValue ?? 0) }
            let start = points.first?["at"].numberValue ?? 0
            let span = max(1, (points.last?["at"].numberValue ?? start) - start)
            let domain = ChartValueScale.domain(values: points.compactMap { $0["value"].numberValue })
            ZStack(alignment: .topLeading) {
                ForEach(Array(points.enumerated()), id: \.offset) { _, point in
                    Circle().fill(tint).frame(width: 6, height: 6)
                        .offset(x: (point["at"].numberValue! - start) / span * max(1, geometry.size.width - 6),
                                y: (domain.upperBound - point["value"].numberValue!) / (domain.upperBound - domain.lowerBound) * max(1, geometry.size.height - 6))
                }
            }
        }.accessibilityLabel("Recorded measurements over time; full values and sources in details")
    }
}

struct InsightVitalReading: View {
    let value: JSONValue
    let tint: Color
    var body: some View {
        VStack(alignment: .leading, spacing: 6) {
            HStack(alignment: .firstTextBaseline) {
                Text(value["title"].stringValue).font(.subheadline)
                Spacer()
                if let number = value["latest"].numberValue {
                    Text("\(number.formatted(.number.precision(.fractionLength(0...1)))) \(HistoricalValue.displayUnit(value["unit"].stringValue, kind: value["metric_id"].stringValue))")
                        .font(.headline)
                }
            }
            if let at = value["latest_at"].numberValue {
                Text(Date(timeIntervalSince1970: at).formatted(date: .abbreviated, time: .omitted)).font(.caption2).foregroundStyle(.secondary)
            }
            if let low = value["baseline_low"].numberValue, let high = value["baseline_high"].numberValue, let latest = value["latest"].numberValue {
                GeometryReader { geometry in
                    let domain = ChartValueScale.domain(values: [low, high, latest])
                    let span = domain.upperBound - domain.lowerBound
                    ZStack(alignment: .leading) {
                        Capsule().fill(tint.opacity(0.08))
                        RoundedRectangle(cornerRadius: 5).fill(tint.opacity(0.24))
                            .frame(width: max(3, (high - low) / span * geometry.size.width))
                            .offset(x: (low - domain.lowerBound) / span * geometry.size.width)
                        Circle().fill(tint).frame(width: 12, height: 12)
                            .offset(x: max(0, min(geometry.size.width - 12, (latest - domain.lowerBound) / span * geometry.size.width - 6)))
                    }
                }.frame(height: 18).accessibilityLabel("Observed baseline \(low) to \(high); latest daily value \(latest)")
            } else { Text("More baseline days needed").font(.caption).foregroundStyle(.secondary) }
        }
    }
}

struct InsightSleepStages: View {
    let session: JSONValue
    private let names = ["Awake", "REM", "Core", "Deep", "Asleep"]
    private let codes = [2, 5, 3, 4, 1]
    private let colors: [Color] = [.orange, .cyan, .blue, .indigo, .purple]
    var body: some View {
        GeometryReader { geometry in
            let start = session["start_at"].numberValue ?? 0
            let span = max(1, (session["end_at"].numberValue ?? start) - start)
            let width = max(1, geometry.size.width - 54)
            VStack(spacing: 3) {
                ForEach(0..<5, id: \.self) { index in
                    HStack(spacing: 6) {
                        Text(names[index]).font(.caption2).frame(width: 48, alignment: .leading)
                        ZStack(alignment: .leading) {
                            Capsule().fill(colors[index].opacity(0.08))
                            ForEach(Array(session["timeline"].arrayValue.enumerated()), id: \.offset) { _, interval in
                                if interval["category"].numberValue == Double(codes[index]), let from = interval["start_at"].numberValue, let to = interval["end_at"].numberValue {
                                    RoundedRectangle(cornerRadius: 3).fill(colors[index])
                                        .frame(width: max(1, (to - from) / span * width))
                                        .offset(x: (from - start) / span * width)
                                }
                            }
                        }
                    }
                }
            }
        }.accessibilityLabel("Recorded sleep stages: Awake, REM, Core, Deep and unspecified Asleep")
    }
}

struct InsightDetail: View {
    let model: AppModel
    let panel: JSONValue
    let response: JSONValue
    @State var editingContext = false
    @Environment(\.dismiss) var dismiss
    private var recoveryEvidence: JSONValue {
        var result = response["training"]
        result["method"] = panel["method"]
        result["references"] = panel["references"]
        return result
    }
    var body: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: 24) {
                InsightDashboardCard(model: model, panel: panel, response: response, expanded: true)
                if panel["id"] == .string("cardiovascular") {
                    CardiovascularInsight(model: model, insight: response["cardiovascular"])
                    Button("Risk context") { editingContext = true }.buttonStyle(.borderedProminent)
                } else if panel["id"] == .string("readiness") {
                    RecoveryInsight(model: model, insight: recoveryEvidence)
                } else {
                    InsightHeading(insight: panel)
                    if panel["id"] == .string("sleep"), panel["source"] != .null {
                        Text(model.sourceLabel(panel["source"].stringValue, name: "")).font(.subheadline).foregroundStyle(.secondary)
                        Text("Longest complete session on the latest recorded wake date").font(.caption).foregroundStyle(.secondary)
                    }
                    ForEach(Array(panel["evidence"].arrayValue.enumerated()), id: \.offset) { _, card in
                        InsightEvidencePlot(model: model, card: card)
                    }
                    if panel["evidence"].arrayValue.isEmpty {
                        Text("Sync the corresponding measurements to see your history here.").foregroundStyle(.secondary)
                    }
                }
            }.frame(maxWidth: .infinity, alignment: .leading).padding(20)
        }
        .navigationTitle(panel["title"].stringValue).navigationBarTitleDisplayMode(.inline)
        .toolbar { ToolbarItem(placement: .confirmationAction) { Button("Done") { dismiss() } } }
        .sheet(isPresented: $editingContext) { NavigationStack { CardiovascularContextEditor(model: model) } }
    }
}

struct InsightSymbol: View {
    let symbol: String
    let color: Color
    var body: some View {
        #if os(Android)
        if symbol == "exclamationmark.circle.fill" {
            Text("!").font(.headline.bold()).foregroundStyle(color).frame(width: 24, height: 24)
                .background(color.opacity(0.15), in: Circle())
        } else {
            Image(BrowseCategoryIcon.asset(for: symbol), bundle: .module).renderingMode(.template)
                .resizable().scaledToFit().foregroundStyle(color).frame(width: 24, height: 24)
        }
        #else
        Image(systemName: symbol).foregroundStyle(color)
        #endif
    }
}
