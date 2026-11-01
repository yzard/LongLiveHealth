import SwiftUI

struct SleepGoalEditor: View {
    var model: AppModel
    var body: some View {
        AutoSavedPreferencesForm(model: model, title: "Sleep Goal") { preferences, update in
            let total = preferences["sleep_target_minutes"].numberValue.map(Int.init)
            let hours = (total ?? 480) / 60
            let minutes = (total ?? 480) % 60
            Section {
                Toggle("Set a sleep goal", isOn: Binding(get: { total != nil }, set: { enabled in
                    update(["sleep_target_minutes": enabled ? .number(480) : .null])
                }))
                if total != nil {
                    Picker("Hours", selection: Binding(get: { hours }, set: { value in
                        update(["sleep_target_minutes": .number(Double(max(1, value * 60 + (value == 24 ? 0 : minutes))))])
                    })) {
                        ForEach(Array(0...24), id: \.self) { Text("\($0) hr").tag($0) }
                    }.pickerStyle(.menu).accessibilityIdentifier("sleep-goal-hours")
                    Picker("Minutes", selection: Binding(get: { minutes }, set: { value in
                        update(["sleep_target_minutes": .number(Double(hours * 60 + value))])
                    })) {
                        ForEach(Array((hours == 0 ? 1 : 0)...59), id: \.self) { Text("\($0) min").tag($0) }
                    }.pickerStyle(.menu).disabled(hours == 24).accessibilityIdentifier("sleep-goal-minutes")
                }
            }
        }
    }
}
