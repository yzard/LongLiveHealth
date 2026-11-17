import SwiftUI

struct CalendarPlanButton: View {
    let title: String
    let start: Date
    let durationMinutes: Double
    @State var failure: String?
    var body: some View {
        VStack(alignment: .leading) {
            Button(uiText("Review in Calendar")) { Task {
                do { _ = try await AndroidSystem.perform("calendar.review", input: .object([
                    "title": .string(title), "start": .number(start.timeIntervalSince1970), "duration": .number(durationMinutes)
                ])) } catch { failure = error.localizedDescription }
            } }
            if let failure { Text(failure).font(.caption).foregroundStyle(.red) }
        }
    }
}
