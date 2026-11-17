import SwiftUI

struct ReportsView: View {
    var model: AppModel
    var body: some View {
        List {
            if model.isBusy {
                Section { ProgressView(uiText("Finishing the current operation…")) }
            }
            if !model.drafts.isEmpty {
                Section {
                    InfoHelp(title: uiText("Background processing"), message: uiText("Uploads can continue in the background. After upload, OCR runs on your server even when this app is closed."))
                }
                Section(uiText("Waiting to upload")) {
                    ForEach(model.drafts) { draft in Label(draft.filename, systemImage: MobileSymbol.name("icloud.and.arrow.up"))
                        .swipeActions { Button(uiText("Discard"), role: .destructive) { model.discardDraft(draft.id) }.disabled(model.isBusy) }
                    }
                    Button(uiText("Retry uploads")) { Task { await model.sendDrafts(automatic: false) } }.disabled(model.isBusy)
                }
            }
            Section {
                if model.reports.isEmpty { EmptyContent(title: uiText("Add your first report"), symbol: "doc.badge.plus") }
                ForEach(model.reports.sorted { ($0["created_at"].numberValue ?? 0) > ($1["created_at"].numberValue ?? 0) }, id: \.identifier) { report in
                    NavigationLink { ReportView(model: model, reportID: report["report_id"].stringValue) } label: {
                        HStack(alignment: .firstTextBaseline, spacing: 12) {
                            let source = report["context"]["source_name"].stringValue.trimmingCharacters(in: .whitespacesAndNewlines)
                            Text(source.isEmpty ? uiText("Source not set") : source).font(.headline)
                                .frame(maxWidth: .infinity, alignment: .leading)
                            let collectionDate = report["collection_date"].stringValue
                            Text(collectionDate.isEmpty ? "—" : collectionDate)
                                .font(.caption).foregroundStyle(.secondary)
                                .fixedSize(horizontal: true, vertical: false)
                                .accessibilityLabel(Text(collectionDate.isEmpty ? uiText("Collection date unavailable") : uiText("Collection date \(collectionDate)")))
                            Text(report["created_at"].numberValue.map { uiDate(Date(timeIntervalSince1970: $0), dateStyle: .abbreviated, timeStyle: .shortened) } ?? "—")
                                .font(.caption).foregroundStyle(.secondary)
                                .multilineTextAlignment(.trailing).fixedSize(horizontal: false, vertical: true)
                        }.padding(.vertical, 4)
                    }.accessibilityIdentifier("report-row-" + report["report_id"].stringValue)
                }
            }
        }
        .mobileNavigationTitle(uiText("Blood Work"))
        .refreshable { await model.refresh(automatic: false) }
        .toolbar { ToolbarItem(placement: .primaryAction) { ReportImportControl(model: model) } }
    }
}
