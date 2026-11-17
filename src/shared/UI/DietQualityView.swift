import SwiftUI

struct DietQualityView: View {
    var model: AppModel
    @State var date = Date()
    @State var result: JSONValue = .null
    @State var failure: String?
    @State var editing = false
    var body: some View {
        List {
            Section(uiText("Confirmed diet assessment")) {
                DatePicker(uiText("Ending date"), selection: $date, displayedComponents: .date)
                InfoHelp(title: uiText("Diet quality"), message: uiText("HEI-2020 describes diet composition. It requires full-day food-pattern equivalents, not ordinary portion weights or photo estimates."))
                Button(uiText("Record full-day equivalents")) { editing = true }
                NavigationLink(uiText("Edit or delete assessments"), destination: DailyLogsView(model: model))
                Link(uiText("NCI definitions and scoring"), destination: URL(string: "https://epi.grants.cancer.gov/hei/hei-scoring-method.html")!)
                Group {
                    InfoHelp(title: uiText("Required inputs"), message: uiText("energy_kcal; total_fruit_cup_eq; whole_fruit_cup_eq; vegetables_cup_eq; greens_beans_cup_eq; whole_grains_oz_eq; dairy_cup_eq; protein_foods_oz_eq; seafood_plant_oz_eq; unsaturated_fat_g; saturated_fat_g; refined_grains_oz_eq; sodium_mg; added_sugars_tsp_eq"))
                    InfoHelp(title: uiText("Food equivalents"), message: uiText("Include legumes in both vegetable and protein totals. Unsaturated fat means monounsaturated plus polyunsaturated fat. Added sugar uses teaspoon equivalents."))
                }
                if let failure { Text(uiMessage(failure)).foregroundStyle(.secondary) }
            }
            ForEach(Array(result["windows"].arrayValue.enumerated()), id: \.offset) { _, window in
                Section(uiText("\(Int(window["days"].numberValue ?? 0)) days")) {
                    Text(uiText("\(Int(window["complete_days"].numberValue ?? 0)) complete days"))
                    Text(uiText("HEI: \(window["result"]["total"].numberValue?.formatted() ?? uiText("Insufficient confirmed inputs"))"))
                    ForEach(Array(window["result"]["components"].arrayValue.enumerated()), id: \.offset) { _, component in
                        LabeledContent(component["component"].stringValue.replacingOccurrences(of: "_", with: " "), value: (component["score"].numberValue?.formatted() ?? uiText("Unknown")) + " / " + (component["maximum"].numberValue?.formatted() ?? ""))
                    }
                }
            }
            Section { if !result["notes"].arrayValue.isEmpty { InfoHelp(title: uiText("Method"), message: result["notes"].arrayValue.map(\.stringValue).joined(separator: "\n\n")) } }
        }.mobileNavigationTitle(uiText("Diet quality")).initialTask(id: date) { await load() }
            .sheet(isPresented: $editing) { NavigationStack { LogEditor(model: model, existing: nil, initialKind: "diet_quality", initialValues: [:], initialDate: date) } }
    }
    private func load() async {
        guard let client = model.client, let token = model.session?.token else { return }
        let formatter = DateFormatter(); formatter.dateFormat = "yyyy-MM-dd"; formatter.locale = Locale(identifier: "en_US_POSIX"); formatter.timeZone = .current
        do {
            let response = try await model.request("wellness/diet", body: .object(["date": .string(formatter.string(from: date)), "timezone": .string(TimeZone.current.identifier)]))
            guard !Task.isCancelled, model.session?.token == token else { return }
            result = response; failure = nil
        } catch { if !Task.isCancelled, model.session?.token == token { failure = error.localizedDescription } }
    }
}
