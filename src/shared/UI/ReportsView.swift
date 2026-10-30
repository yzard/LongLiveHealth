import SwiftUI

struct ReportsView: View {
    var model: AppModel
    var body: some View {
        List {
            if model.isBusy {
                Section { ProgressView("Finishing the current operation…") }
            }
            if let refresh = model.lastRefresh { Section { Text("Last refreshed \(refresh.formatted())").font(.caption).foregroundStyle(.secondary) } }
            if !model.drafts.isEmpty {
                Section {
                    InfoHelp(title: "Background processing", message: "Uploads can continue in the background. After upload, OCR runs on your server even when this app is closed.")
                }
                Section("Waiting to upload") {
                    ForEach(model.drafts) { draft in Label(draft.filename, systemImage: MobileSymbol.name("icloud.and.arrow.up"))
                        .swipeActions { Button("Discard", role: .destructive) { model.discardDraft(draft.id) }.disabled(model.isBusy) }
                    }
                    Button("Retry uploads") { Task { await model.sendDrafts(automatic: false) } }.disabled(model.isBusy)
                }
            }
            Section("Your reports") {
                if model.reports.isEmpty { EmptyContent(title: "Add your first report", symbol: "doc.badge.plus") }
                ForEach(model.reports.sorted { ($0["created_at"].numberValue ?? 0) > ($1["created_at"].numberValue ?? 0) }, id: \.identifier) { report in
                    NavigationLink { ReportView(model: model, reportID: report["report_id"].stringValue) } label: {
                        VStack(alignment: .leading, spacing: 5) {
                            Text(report["context"]["report_title"].stringValue.isEmpty ? report["original_name"].stringValue : report["context"]["report_title"].stringValue).font(.headline)
                            Text("\(Int(report["page_count"].numberValue ?? 0)) pages · revision \(Int(report["revision"].numberValue ?? 1))").font(.caption).foregroundStyle(.secondary)
                            if let job = model.jobs.first(where: { $0["file_id"] == report["report_id"] }) {
                                Text("Extraction: \(job["status"].stringValue)").font(.caption)
                            }
                        }.padding(.vertical, 4)
                    }
                }
            }
        }
        .navigationTitle("Health Report")
        .refreshable { await model.refresh(automatic: false) }
        .toolbar { ToolbarItem(placement: .primaryAction) { ReportImportControl(model: model) } }
    }
}
