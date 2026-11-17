import SwiftUI

struct SharedReportImportView: View {
    var model: AppModel
    var body: some View {
        NavigationStack {
            List {
                Section(uiText("PDF reports")) {
                    InfoHelp(title: uiText("Report recognition"), message: uiText("OCR will identify the report title and source after upload. You can edit them in the report."))
                    ForEach(model.sharedReports) { report in
                        VStack(alignment: .leading) {
                            Text(report.filename)
                        }
                            .swipeActions { Button(uiText("Discard"), role: .destructive) { model.discardSharedReport(report) }.disabled(model.isBusy) }
                    }
                }
                if let session = model.session, !model.mustChangePassword {
                    Section(uiText("Import to")) {
                        Text(session.username)
                        Text(session.server.absoluteString).font(.caption).foregroundStyle(.secondary)
                        Button(uiText("Import")) { Task { await model.importSharedReports() } }.disabled(model.isBusy)
                    }
                } else {
                    Text(uiText("Sign in to import these reports. Your PDFs are saved on this iPhone."))
                }
                if model.isBusy { ProgressView(uiText("Waiting for the current operation…")) }
            }
            .mobileNavigationTitle(uiText("Import Blood Work"))
            .toolbar { Button(uiText("Later")) { model.showSharedReportImport = false } }
        }
    }
}
