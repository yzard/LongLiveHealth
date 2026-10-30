import SwiftUI

struct CalendarPlanButton: View {
    let title: String
    let start: Date
    let durationMinutes: Double
    @State var failure: String?
    var body: some View {
        VStack(alignment: .leading) {
            Button("Review in Calendar") { Task {
                do { _ = try await AndroidSystem.perform("calendar.review", input: .object([
                    "title": .string(title), "start": .number(start.timeIntervalSince1970), "duration": .number(durationMinutes)
                ])) } catch { failure = error.localizedDescription }
            } }
            if let failure { Text(failure).font(.caption).foregroundStyle(.red) }
        }
    }
}

struct VoiceQuestionButton: View {
    @Binding var question: String
    @State var busy = false
    @State var failure: String?
    var body: some View {
        VStack(alignment: .leading) {
            Button("Dictate on this device") { Task {
                busy = true; defer { busy = false }
                do {
                    let result = try await AndroidSystem.perform("speech.transcribe", input: .object([:]))
                    if !result["text"].stringValue.isEmpty { question = result["text"].stringValue }
                } catch { failure = error.localizedDescription }
            } }.disabled(busy)
            if busy { ProgressView("Listening") }
            if let failure { Text(failure).font(.caption).foregroundStyle(.secondary) }
        }
    }
}
