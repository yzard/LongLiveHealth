import SwiftUI

struct SharedReportImportView: View {
    var model: AppModel
    var body: some View {
        NavigationStack {
            List {
                Section("PDF reports") {
                    InfoHelp(title: "Report recognition", message: "OCR will identify the report title and source after upload. You can edit them in the report.")
                    ForEach(model.sharedReports) { report in
                        VStack(alignment: .leading) {
                            Text(report.filename)
                        }
                            .swipeActions { Button("Discard", role: .destructive) { model.discardSharedReport(report) }.disabled(model.isBusy) }
                    }
                }
                if let session = model.session, !model.mustChangePassword {
                    Section("Import to") {
                        Text(session.username)
                        Text(session.server.absoluteString).font(.caption).foregroundStyle(.secondary)
                        Button("Import") { Task { await model.importSharedReports() } }.disabled(model.isBusy)
                    }
                } else {
                    Text("Sign in to import these reports. Your PDFs are saved on this iPhone.")
                }
                if model.isBusy { ProgressView("Waiting for the current operation…") }
            }
            .mobileNavigationTitle("Import Blood Work")
            .toolbar { Button("Later") { model.showSharedReportImport = false } }
        }
    }
}
