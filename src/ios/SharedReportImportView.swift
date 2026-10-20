import SwiftUI

struct SharedReportImportView: View {
    var model: AppModel
    @State private var sourceNames: [UUID: String] = [:]
    var body: some View {
        NavigationStack {
            List {
                Section("PDF reports") {
                    ForEach(model.sharedReports) { report in
                        VStack(alignment: .leading) {
                            Text(report.filename)
                            TextField("Clinic or laboratory", text: Binding(get: { sourceNames[report.id] ?? "" }, set: { sourceNames[report.id] = $0 }))
                        }
                            .swipeActions { Button("Discard", role: .destructive) { model.discardSharedReport(report) }.disabled(model.isBusy) }
                    }
                }
                if let session = model.session, !model.mustChangePassword {
                    Section("Import to") {
                        Text(session.username)
                        Text(session.server.absoluteString).font(.caption).foregroundStyle(.secondary)
                        Button("Import") { Task { await model.importSharedReports(sourceNames: sourceNames) } }.disabled(model.isBusy || model.sharedReports.contains { (sourceNames[$0.id] ?? "").trimmingCharacters(in: .whitespacesAndNewlines).isEmpty })
                    }
                } else {
                    Text("Sign in to import these reports. Your PDFs are saved on this iPhone.")
                }
                if model.isBusy { ProgressView("Waiting for the current operation…") }
            }
            .navigationTitle("Import Health Report")
            .toolbar { Button("Later") { model.showSharedReportImport = false } }
        }
    }
}
