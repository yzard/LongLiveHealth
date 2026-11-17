import SwiftUI

struct ReportView: View {
    var model: AppModel
    let reportID: String
    @State var document: JSONValue = .null
    @State var editor: EditTarget?
    @State var healthWriter: EditTarget?
    @State var preview: DocumentLocation?
    @State var contextVisible = false
    @State var deleting = false
    @State var rescanning = false
    @State var rescanConfirmation = false
    @State var confirmingAll = false
    @State var savingReview = false
    @State var confirmationError: String?
    @Environment(\.scenePhase) var scenePhase
    @Environment(\.dismiss) var dismiss

    private struct IndexedValue: Identifiable {
        let id: String
        let value: JSONValue
    }
    private func indexedValues(_ section: String) -> [IndexedValue] {
        document[section].arrayValue.enumerated().map {
            IndexedValue(id: "\(section)-\($0.offset)", value: $0.element)
        }
    }

    struct EditTarget: Identifiable {
        let id = UUID()
        let observation: JSONValue
        let choosingMetric: Bool
    }
    var body: some View {
        reportList
        .disabled(confirmingAll || rescanning || savingReview)
        .mobileNavigationTitle(document["context"]["report_title"].stringValue.isEmpty ? "Blood Work" : document["context"]["report_title"].stringValue)
        .navigationBarTitleDisplayMode(.inline)
        .initialTask { await reload() }
        .task(id: model.jobs.first { $0["file_id"].stringValue == reportID }?["job_id"].stringValue) { await followExtraction() }
        .refreshable { await reload() }
        .originalReportPreview($preview)
        .sheet(item: $editor) { target in reviewEditor(target) }
        .sheet(item: $healthWriter) { target in ReviewedGlucoseWriter(model: model, observation: target.observation) }
        .sheet(isPresented: $contextVisible) { ContextEditor(context: document["context"]) { context in try await save(observations: [], context: context) } }
        .alert("Rescan report?", isPresented: $rescanConfirmation) {
            Button("Cancel", role: .cancel) {}
            Button("Rescan", role: .destructive) { Task { await rescan() } }
        } message: { Text("Rescan deletes all existing results and runs OCR again on the original report. New results will be pending review.") }
        .alert("Delete report?", isPresented: $deleting) {
            Button("Cancel", role: .cancel) {}
            Button("Delete report", role: .destructive) { Task { await model.perform {
                guard let client = model.client else { return }
                _ = try await model.request("reports/delete", body: .object(["report_id": .string(reportID), "expected_revision": document["report"]["revision"]]))
                try model.archive?.remove(name: "report-\(reportID).json"); try model.archive?.remove(name: "source-\(reportID)")
                model.reports.removeAll { $0["report_id"].stringValue == reportID }
                model.jobs.removeAll { $0["file_id"].stringValue == reportID }
                try model.archive?.save(model.reports, name: "reports.json")
                dismiss()
            } } }
        } message: { Text("This deletes the report, its confirmed and draft results, and related analysis from your archive. The server then removes stored files. Offline backups you made are separate.") }
    }
    private enum ReportRow: Identifiable {
        case info, markerCount, actions, retry, relationship, unlink, review, error, empty, detail
        case validation(IndexedValue)
        case observation(JSONValue)

        var id: String {
            switch self {
            case .info: "report-info"
            case .markerCount: "marker-count"
            case .actions: "report-actions"
            case .retry: "retry-extraction"
            case .relationship: "relationship"
            case .unlink: "unlink-report"
            case .review: "review-actions"
            case .error: "confirmation-error"
            case .empty: "empty-results"
            case .detail: "report-detail"
            case .validation(let issue): issue.id
            case .observation(let value): "observation-\(value.identifier)"
            }
        }
    }

    private struct ReportSection: Identifiable {
        let id: String
        let title: String?
        let rows: [ReportRow]
    }

    private var sections: [ReportSection] {
        let observations = document["observations"].arrayValue
        let validation = indexedValues("validation_issues")
        var information: [ReportRow] = [.info, .markerCount, .actions]
        if failedJob != nil { information.append(.retry) }
        if !document["relation"]["preferred_report_id"].stringValue.isEmpty {
            information.append(.relationship)
            information.append(.unlink)
        }
        var review: [ReportRow] = [.review]
        if confirmationError != nil { review.append(.error) }
        if observations.isEmpty && validation.isEmpty { review.append(.empty) }

        let needsReview = validation.map { ReportRow.validation($0) } + observations.filter {
            $0["status"] == .string("pending") && !ReportReview.canBulkConfirm($0)
        }.map { ReportRow.observation($0) }
        let confirmed = observations.filter { $0["status"] == .string("confirmed") }
            .map { ReportRow.observation($0) }
        var result = [
            ReportSection(id: "info", title: nil, rows: information),
            ReportSection(id: "review", title: "Review Results", rows: review)
        ]
        if !needsReview.isEmpty {
            result.append(ReportSection(id: "manual", title: "Needs Review", rows: needsReview))
        }
        if !confirmed.isEmpty {
            result.append(ReportSection(id: "confirmed", title: "Confirmed Results", rows: confirmed))
        }
        result.append(ReportSection(id: "detail", title: nil, rows: [.detail]))
        return result
    }

    @ViewBuilder private var reportList: some View {
        #if os(Android)
        // Skip renders the lazy stack as the sole vertical scroll owner. Keep
        // headings and rows there so the native list can't reorder or animate them.
        ScrollView {
            LazyVStack(alignment: .leading, spacing: 0) {
                ForEach(scrollItems) { item in
                    switch item {
                    case .header(_, let title):
                        reportSectionHeader(title)
                    case .row(let row, let first, let last):
                        reportListRow(row, first: first, last: last)
                    }
                }
            }
            .padding(16)
        }
        #else
        List {
            ForEach(sections) { section in
                Section {
                    ForEach(section.rows) { row in reportRow(row) }
                } header: {
                    if let title = section.title { Text(title) }
                }
            }
        }
        #endif
    }

    #if os(Android)
    private enum ReportScrollItem: Identifiable {
        case header(String, String?)
        case row(ReportRow, first: Bool, last: Bool)

        var id: String {
            switch self {
            case .header(let id, _): "section-\(id)"
            case .row(let row, _, _): row.id
            }
        }
    }

    private var scrollItems: [ReportScrollItem] {
        var items: [ReportScrollItem] = []
        for section in sections {
            if section.id != "info" {
                items.append(.header(section.id, section.title))
            }
            for (index, row) in section.rows.enumerated() {
                items.append(.row(row, first: index == 0, last: index == section.rows.count - 1))
            }
        }
        return items
    }

    @ViewBuilder private func reportSectionHeader(_ title: String?) -> some View {
        if let title {
            Text(title.uppercased()).font(.footnote).foregroundStyle(.secondary)
                .accessibilityAddTraits(.isHeader)
                .padding(.horizontal, 16).padding(.top, 24).padding(.bottom, 8)
        } else {
            Color.clear.frame(height: 24)
        }
    }

    private func reportListRow(_ row: ReportRow, first: Bool, last: Bool) -> some View {
        VStack(spacing: 0) {
            reportRow(row)
                .frame(maxWidth: .infinity, minHeight: 32, alignment: .leading)
                .padding(.horizontal, 16).padding(.vertical, 8)
            if !last { Divider() }
        }
        .background(Color(.systemBackground))
        .clipShape(UnevenRoundedRectangle(topLeadingRadius: first ? 8 : 0,
            bottomLeadingRadius: last ? 8 : 0, bottomTrailingRadius: last ? 8 : 0,
            topTrailingRadius: first ? 8 : 0))
    }
    #endif

    @ViewBuilder private func reportRow(_ row: ReportRow) -> some View {
        switch row {
        case .info:
            Button { contextVisible = true } label: {
                HStack {
                    Text("Report Info")
                    Spacer()
                    let source = document["context"]["source_name"].stringValue
                    Text(source.isEmpty ? "Set source name" : source).foregroundStyle(.secondary)
                    Image(systemName: MobileSymbol.name("chevron.right")).font(.caption).foregroundStyle(.secondary)
                }
            }
            #if os(Android)
            .buttonStyle(.plain)
            #endif
            .accessibilityIdentifier("report-info")
        case .markerCount:
            let count = document["observations"].arrayValue.count
            LabeledContent("Total Markers", value: "\(count)")
                .accessibilityIdentifier("report-marker-count").accessibilityValue("\(count)")
        case .actions:
            reportActions
        case .retry:
            Button("Retry failed extraction") { Task { await retryExtraction() } }
        case .relationship:
            Text("Excluded from trends: \(document["relation"]["kind"].stringValue). Original data is retained.").font(.caption)
        case .unlink:
            Button("Treat as independent report") { Task { await unlink() } }
        case .review:
            reviewActions
        case .error:
            if let confirmationError { Text(confirmationError).foregroundStyle(.red) }
        case .empty:
            Text("No extracted results yet.").foregroundStyle(.secondary)
        case .validation(let entry):
            validationRow(entry.value)
        case .observation(let observation):
            observationRow(observation)
        case .detail:
            NavigationLink {
                List { reportDetails }.mobileNavigationTitle("Detail")
            } label: {
                #if os(Android)
                HStack {
                    Text("Detail")
                    Spacer()
                    Image(systemName: MobileSymbol.name("chevron.right")).font(.caption).foregroundStyle(.secondary)
                }
                #else
                Text("Detail")
                #endif
            }.accessibilityIdentifier("report-detail")
        }
    }

    private var reportActions: some View {
        HStack(spacing: 12) {
            Button { showSource(page: 1) } label: { reportAction("View Original", icon: "doc.text.magnifyingglass") }
                .accessibilityLabel("View original report").accessibilityIdentifier("report-original")
            Button { editor = EditTarget(observation: .null, choosingMetric: false) } label: { reportAction("Add Result", icon: "plus") }
                .accessibilityLabel("Add result manually").accessibilityIdentifier("report-add-result")
            Button { rescanConfirmation = true } label: { reportAction("Rescan", icon: "arrow.clockwise") }
                .accessibilityLabel("Rescan report").accessibilityIdentifier("report-rescan")
                .disabled(extracting || rescanning)
            Button(role: .destructive) { deleting = true } label: { reportAction("Delete", icon: "trash") }
                .accessibilityLabel("Delete report").accessibilityIdentifier("report-delete")
        }.buttonStyle(.borderless)
        .disabled(document == .null || model.isBusy)
    }

    private var ready: [JSONValue] {
        document["observations"].arrayValue.filter { ReportReview.canBulkConfirm($0) }
    }

    private var reviewActions: some View {
        let observations = ready
        return HStack {
            NavigationLink {
                ReadyToConfirmView(model: model, reportID: reportID, document: $document,
                    reviewEditor: reviewEditor, confirmReadyResult: confirmReadyResult)
            } label: {
                Text(observations.count == 1 ? "1 result ready to confirm" : "\(observations.count) results ready to confirm")
                    .foregroundStyle(.primary).frame(maxWidth: .infinity, alignment: .leading)
            }.buttonStyle(.borderless)
            .accessibilityIdentifier("ready-to-confirm-results")
            .disabled(observations.isEmpty)
            Spacer(minLength: 8)
            Button(confirmingAll ? "Confirming…" : "Confirm") { Task { await confirmAll() } }
                .buttonStyle(.borderless)
                .accessibilityIdentifier("confirm-all-results")
                .disabled(confirmingAll || extracting || observations.isEmpty)
        }
    }

    private func validationRow(_ issue: JSONValue) -> some View {
        let name = issue["item_name"].stringValue
        let field = issue["field"].stringValue.replacingOccurrences(of: "_", with: " ")
        let reason = issue["reason"] == .string("invalid_date") ? "Invalid" : "Missing"
        return VStack(alignment: .leading, spacing: 4) {
            Text(name.isEmpty ? "Item \(Int(issue["item_index"].numberValue ?? 0))" : name).font(.headline)
            Text("Page \(Int(issue["page"].numberValue ?? 0)) · \(reason) \(field)").foregroundStyle(.red)
        }
    }

    private var failedJob: JSONValue? {
        guard let job = model.jobs.first(where: { $0["file_id"].stringValue == reportID }),
              job["status"] == .string("failed") else { return nil }
        return job
    }

    private func retryExtraction() async {
        guard let job = failedJob else { return }
        await model.perform {
            guard model.client != nil else { return }
            _ = try await model.request("jobs/retry", body: .object(["job_id": job["job_id"]]))
        }
        await model.refresh(automatic: false)
        await reload()
    }

    private func reviewEditor(_ target: EditTarget) -> ObservationEditor {
        ObservationEditor(observation: target.observation, metrics: model.metrics,
            pageCount: Int(document["report"]["page_count"].numberValue ?? 1), choosingMetric: target.choosingMetric, source: { page in
                DocumentLocation(url: try await model.source(reportID), page: page)
            }) { change in
                do {
                    try await save(observations: [change], context: .null)
                } catch APIError.status(409, let message)
                    where message == "Report changed; reload before reviewing" || message == "Observation changed" {
                    let latest = try await model.refreshReportAfterReviewConflict(reportID)
                    try await applySavedReport(latest)
                    let id = change["observation_id"]
                    if id != .null && latest["observations"].arrayValue.first(where: { $0["observation_id"] == id })?["revision"] != change["expected_revision"] {
                        throw APIError.status(409, "This result changed. Check the refreshed result before reapplying your edits.")
                    }
                    throw APIError.status(409, "The report was refreshed. Your edits are still open. Tap Save to submit them again.")
                }
        }
    }

    private func observationRow(_ observation: JSONValue) -> some View {
        let id = observation["observation_id"]
        let renderedStatus = observation["status"].stringValue
        let revision = observation["revision"].numberValue ?? 0
        return ReportObservationRow(model: model, observation: observation,
            changeStatus: {
                if ReportReview.canBulkConfirm(observation) {
                    Task { await confirmReadyResult(id) }
                } else {
                    let needsMapping = observation["browse_issues"].arrayValue.contains { $0["scope"] == .string("browse") }
                    editor = EditTarget(observation: observation, choosingMetric: needsMapping)
                }
            },
            edit: { editor = EditTarget(observation: observation, choosingMetric: false) },
            chooseMetric: { editor = EditTarget(observation: observation, choosingMetric: true) },
            showSource: { showSource(page: $0) },
            writeHealth: { healthWriter = EditTarget(observation: observation, choosingMetric: false) })
            .id("\(id.stringValue)-\(revision)-\(renderedStatus)")
    }

    @ViewBuilder private var reportDetails: some View {

        if !document["pages"].arrayValue.isEmpty {
            Section("Page processing") {
                ForEach(indexedValues("pages")) { entry in
                    let page = entry.value
                    NavigationLink("Page \(Int(page["page"].numberValue ?? 0)) · \(page["status"].stringValue)") {
                        ScrollView { CopyableText(page["content"].stringValue).font(.caption).padding() }
                            .mobileNavigationTitle("Page processing")
                    }
                }
            }
        }
        if !document["extraction_inputs"].arrayValue.isEmpty {
            Section("Document evidence") {
                ForEach(indexedValues("extraction_inputs")) { entry in
                    let input = entry.value
                    NavigationLink("Page \(Int(input["page"].numberValue ?? 0)) · text layer: \(input["text_status"].stringValue.isEmpty ? "image only" : input["text_status"].stringValue)") {
                        DocumentEvidenceView(model: model, reportID: reportID, input: input)
                    }
                }
            }
        }
        if !document["extraction_outputs"].arrayValue.isEmpty {
            Section("Original OCR output") {
                ForEach(indexedValues("extraction_outputs")) { entry in
                    let output = entry.value
                    NavigationLink("Page \(Int(output["page"].numberValue ?? 0)) · \(output["stage"].stringValue)") {
                        ExtractionOutputView(model: model, reportID: reportID, output: output)
                    }
                }
            }
        }
    }
    private func reportAction(_ title: String, icon: String) -> some View {
        VStack(spacing: 5) {
            Image(systemName: MobileSymbol.name(icon)).font(.body)
            Text(title).font(.caption).lineLimit(1).minimumScaleFactor(0.8)
        }.frame(maxWidth: .infinity, minHeight: 44)
    }
    private var extracting: Bool {
        model.jobs.contains { $0["file_id"].stringValue == reportID && ["queued", "running"].contains($0["status"].stringValue) }
    }
    private func rescan() async {
        guard !rescanning, !extracting else { return }
        rescanning = true
        defer { rescanning = false }
        await model.perform {
            let response = try await model.request("reports/rescan", body: .object([
                "report_id": .string(reportID), "expected_revision": document["report"]["revision"],
                "replace_results": .bool(true)
            ]))
            model.jobs.removeAll { $0["file_id"].stringValue == reportID }
            model.jobs.insert(response["job"], at: 0)
            try model.archive?.remove(name: "report-\(reportID).json")
            document = try await model.report(reportID)
        }
    }
    private func confirmAll() async {
        guard !confirmingAll, !savingReview, !rescanning, !extracting else { return }
        confirmingAll = true; confirmationError = nil
        defer { confirmingAll = false }
        do {
            // Use the existing atomic review endpoint; larger reports resume from remaining pending items.
            while true {
                let changes = document["observations"].arrayValue
                    .filter { ReportReview.canBulkConfirm($0) }.prefix(2_000)
                    .map { ReportReview.change($0, to: "confirmed") }
                guard !changes.isEmpty else { return }
                try Task.checkCancellation()
                guard try await saveStatusChanges(changes) else { return }
            }
        } catch { confirmationError = error.localizedDescription }
    }
    private func confirmReadyResult(_ id: JSONValue) async {
        guard !confirmingAll, !savingReview, !rescanning, !extracting,
              let current = document["observations"].arrayValue.first(where: { $0["observation_id"] == id }),
              ReportReview.canBulkConfirm(current) else { return }
        await model.perform {
            _ = try await saveStatusChanges([ReportReview.change(current, to: "confirmed")])
        }
    }

    private func saveStatusChanges(_ changes: [JSONValue]) async throws -> Bool {
        do {
            try await save(observations: changes, context: .null)
            return true
        } catch APIError.status(409, _) {
            let latest = try await model.refreshReportAfterReviewConflict(reportID)
            try await applySavedReport(latest)
            return false
        }
    }

    private func followExtraction() async {
        while !Task.isCancelled {
            guard model.jobs.contains(where: { $0["file_id"].stringValue == reportID && ["queued", "running"].contains($0["status"].stringValue) }) else { return }
            do {
                try await Task.sleep(for: .seconds(4))
                guard scenePhase == .active, model.selectedTab == "Data", !confirmingAll, !savingReview, !contextVisible, editor == nil, preview == nil, !model.isLocked else { continue }
                guard let token = model.session?.token else { return }
                let response = try await model.request("jobs/list", body: .object(["limit": .number(100)]))
                guard !Task.isCancelled, model.session?.token == token else { return }
                model.jobs = response["jobs"].arrayValue
                let latest = try await model.report(reportID)
                guard !Task.isCancelled, model.session?.token == token, !confirmingAll, !savingReview, !contextVisible, editor == nil else { continue }
                _ = adoptReport(latest)
            } catch { return }
        }
    }
    private func reload() async {
        do { _ = adoptReport(try await model.report(reportID)) }
        catch { model.errorMessage = error.localizedDescription }
    }
    private func save(observations: [JSONValue], context: JSONValue) async throws {
        guard model.client != nil, !savingReview else { throw APIError.invalidResponse }
        savingReview = true
        defer { savingReview = false }
        let request: JSONValue = .object(["report_id": .string(reportID), "expected_revision": document["report"]["revision"], "context": context, "observations": .array(observations)])
        let response = try await model.request("reports/review", body: request)
        for metric in response["created_metrics"].arrayValue
            where !model.metrics.contains(where: { $0["metric_id"] == metric["metric_id"] }) {
            model.metrics.append(metric)
        }
        try await applySavedReport(response)
    }

    private func adoptReport(_ snapshot: JSONValue) -> Bool {
        guard let revision = snapshot["report"]["revision"].numberValue,
              revision >= (document["report"]["revision"].numberValue ?? 0) else { return false }
        if document != snapshot { document = snapshot }
        if let index = model.reports.firstIndex(where: { $0["report_id"].stringValue == reportID }) {
            model.reports[index] = snapshot["report"]
            model.reports[index]["context"] = snapshot["context"]
        }
        return true
    }

    private func applySavedReport(_ snapshot: JSONValue) async throws {
        guard adoptReport(snapshot) else { return }
        let savedToken = model.session?.token
        let cachedBytes = try await Task.detached(priority: .utility) { try JSONEncoder().encode(snapshot) }.value
        guard model.session?.token == savedToken, !Task.isCancelled else { throw CancellationError() }
        if document["report"]["revision"] == snapshot["report"]["revision"] {
            try model.archive?.saveBytes(cachedBytes, name: "report-\(reportID).json")
            try model.archive?.save(model.reports, name: "reports.json")
        }
    }
    private func showSource(page: Int) { Task { do { preview = DocumentLocation(url: try await model.source(reportID), page: page) } catch { model.errorMessage = error.localizedDescription } } }
    private func unlink() async {
        await model.perform { guard let client = model.client else { return }
            _ = try await model.request("reports/relate", body: .object(["report_id": .string(reportID), "preferred_report_id": .null, "kind": .string("duplicate"), "expected_revision": document["report"]["revision"]]))
        }; await reload()
    }
}

struct ReadyToConfirmView: View {
    let model: AppModel
    let reportID: String
    @Binding var document: JSONValue
    let reviewEditor: (ReportView.EditTarget) -> ObservationEditor
    let confirmReadyResult: (JSONValue) async -> Void
    @State var editor: ReportView.EditTarget?
    @State var healthWriter: ReportView.EditTarget?
    @State var preview: DocumentLocation?

    var body: some View {
        List {
            ForEach(document["observations"].arrayValue.filter(ReportReview.canBulkConfirm), id: \.identifier) { observation in
                ReportObservationRow(model: model, observation: observation,
                    changeStatus: { _ = Task { await confirmReadyResult(observation["observation_id"]) } },
                    edit: { editor = ReportView.EditTarget(observation: observation, choosingMetric: false) },
                    chooseMetric: { editor = ReportView.EditTarget(observation: observation, choosingMetric: true) },
                    showSource: { page in _ = Task { await openSource(page) } },
                    writeHealth: { healthWriter = ReportView.EditTarget(observation: observation, choosingMetric: false) })
            }
        }
        .mobileNavigationTitle("Ready to Confirm")
        .sheet(item: $editor) { target in reviewEditor(target) }
        .sheet(item: $healthWriter) { target in ReviewedGlucoseWriter(model: model, observation: target.observation) }
        .originalReportPreview($preview)
    }

    private func openSource(_ page: Int) async {
        do { preview = DocumentLocation(url: try await model.source(reportID), page: page) }
        catch { model.errorMessage = error.localizedDescription }
    }
}

struct ExtractionOutputView: View {
    var model: AppModel
    let reportID: String
    let output: JSONValue
    @State var original: JSONValue = .null
    var body: some View {
        ScrollView { VStack(alignment: .leading, spacing: 16) {
            Text(output["model"].stringValue).font(.caption)
            CopyableText(original["content"].stringValue)
            DisclosureGroup("Complete provider response") { CopyableText(original["response_body"].stringValue).font(.caption.monospaced()) }
        }.padding() }.mobileNavigationTitle("Original OCR output").initialTask {
            do {
                guard let client = model.client else { return }
                original = try await model.request("reports/extraction/get", body: .object(["report_id": .string(reportID), "run_id": output["run_id"], "page": output["page"], "stage": output["stage"]]))
            } catch { model.errorMessage = error.localizedDescription }
        }
    }
}

struct ReviewedGlucoseWriter: View {
    var model: AppModel
    let observation: JSONValue
    @State var sampledAt = Date()
    @State var verifiedTime = false
    @State var error: String?
    @Environment(\.dismiss) var dismiss
    var body: some View {
        NavigationStack { Form {
            Text("\(observation["payload"]["raw_result"].stringValue) \(observation["payload"]["raw_unit"].stringValue)")
            Text("Report collection date: \(observation["payload"]["sampled_at"].stringValue)").font(.caption)
            DatePicker("Collection date and time", selection: $sampledAt)
            Toggle("I verified the collection date and time", isOn: $verifiedTime)
            InfoHelp(title: "Collection time", message: "\(MobilePlatform.healthProviderName) requires a timestamp. Check the original report before saving. The complete report and its revisions remain on your server.")
            if let error { Text(error).foregroundStyle(.red) }
            Button("Save reviewed glucose") { Task {
                guard let session = model.session, !model.isBusy else { return }
                var saved = false
                await model.perform {
                    do {
                        guard let client = model.client else { throw APIError.invalidResponse }
                        let timestamp = ISO8601DateFormatter().string(from: sampledAt)
                        var desired = observation["payload"]; desired["sampled_at"] = .string(timestamp)
                        var report = try await model.report(observation["report_id"].stringValue)
                        guard var current = report["observations"].arrayValue.first(where: { $0["observation_id"] == observation["observation_id"] }) else { throw APIError.invalidResponse }
                        if current["payload"] != desired {
                            guard current["revision"] == observation["revision"] else { throw APIError.status(409, "This result changed. Reload it before saving to Apple Health") }
                            report = try await model.request("reports/review", body: .object(["report_id": observation["report_id"], "expected_revision": report["report"]["revision"], "context": .null,
                                "observations": .array([.object(["observation_id": observation["observation_id"], "expected_revision": current["revision"], "status": .string("confirmed"), "payload": desired])])]))
                            guard let updated = report["observations"].arrayValue.first(where: { $0["observation_id"] == observation["observation_id"] }) else { throw APIError.invalidResponse }
                            current = updated
                        }
                        try model.archive?.save(report, name: "report-\(observation["report_id"].stringValue).json")
                        try await model.health.writeReviewedGlucose(current, server: session.server, userID: session.userID, sampledAt: sampledAt)
                        saved = true
                    }
                    catch { self.error = error.localizedDescription }
                }
                if saved { dismiss(); await model.synchronizeHealth(requestAccess: false, requestSelectedAccess: false, automatic: false) }
            } }.disabled(!verifiedTime || model.isBusy)
        }.mobileNavigationTitle("Save to \(MobilePlatform.healthProviderName)").toolbar { Button("Cancel") { dismiss() }.disabled(model.isBusy) } }
    }
}

struct DocumentEvidenceView: View {
    var model: AppModel
    let reportID: String
    let input: JSONValue
    @State var evidence: JSONValue = .null
    @State var error: String?
    var body: some View {
        List {
            if let error { Text(error).foregroundStyle(.red) }
            if evidence == .null && error == nil { ProgressView() }
            else {
                Text("Text layer: \(evidence["text_layer"]["status"].stringValue.isEmpty ? "not applicable to image" : evidence["text_layer"]["status"].stringValue)")
                if !evidence["error_code"].stringValue.isEmpty { Text("Text extraction issue: \(evidence["error_code"].stringValue)").foregroundStyle(.orange) }
                InfoHelp(title: "Recognition evidence", message: "The page image is always used for recognition. PDF text may be incomplete or disagree with the image.")
                CopyableText(evidence["text_layer"]["text"].stringValue)
                Text("Positioned words: \(evidence["text_layer"]["words"].arrayValue.count)").font(.caption)
            }
        }.mobileNavigationTitle("Document evidence").initialTask {
            do {
                guard let client = model.client else { return }
                evidence = try await model.request("reports/input/get", body: .object(["report_id": .string(reportID), "run_id": input["run_id"], "page": input["page"]]))
            } catch { self.error = error.localizedDescription }
        }
    }
}

struct ReportObservationRow: View {
    let model: AppModel
    let observation: JSONValue
    let changeStatus: (() -> Void)?
    let edit: () -> Void
    let chooseMetric: () -> Void
    let showSource: (Int) -> Void
    let writeHealth: () -> Void
    private var needsManualConfirmation: Bool {
        observation["status"] == .string("pending")
            && !ReportReview.canBulkConfirm(observation)
    }

    private var resultLabel: String {
        let printed = [observation["payload"]["raw_result"].stringValue,
                       observation["payload"]["raw_unit"].stringValue]
            .filter { !$0.isEmpty }.joined(separator: " ")
        let bound = observation["reported_negative_bound"].stringValue
        if !bound.isEmpty { return "\(bound) (\(printed))" }
        let standardized = observation["interpretation"]["result"]["display"].stringValue
        return standardized.isEmpty || standardized == printed ? printed : "\(standardized) (\(printed))"
    }
    private var sourcePage: Int { Int(observation["payload"]["source"]["page"].numberValue ?? 1) }

    var body: some View {
        VStack(alignment: .leading, spacing: 7) {
            HStack(spacing: 8) {
                Button { edit() } label: {
                    HStack(alignment: .firstTextBaseline, spacing: 8) {
                        Text(LaboratoryPresentation.resultTitle(observation, renamedMetrics: model.renamedLabMetrics))
                            .font(.headline)
                            .fixedSize(horizontal: false, vertical: true)
                        let code = observation["lab_test_code"].stringValue
                        if !code.isEmpty {
                            Text("LOINC \(code)")
                                .font(.caption)
                                .foregroundStyle(.secondary)
                                .fixedSize()
                                .accessibilityIdentifier("report-result-loinc")
                        }
                    }
                }.buttonStyle(.plain)
                Spacer(minLength: 0)
                let hasSuggestion = ReportReview.suggestedMetricID(observation) != nil
                if hasSuggestion || observation["browse_issues"].arrayValue.contains(where: { $0["code"] == .string("unmapped_metric") }) {
                    Button(action: hasSuggestion ? edit : chooseMetric) {
                        Text(hasSuggestion ? "Suggested match" : "New")
                            .font(.caption.weight(.semibold))
                            .foregroundStyle(Color.yellow)
                            .padding(.horizontal, 8)
                            .padding(.vertical, 4)
                            .background(Color.yellow.opacity(0.12), in: Capsule())
                            .frame(minHeight: 44)
                    }
                    .accessibilityIdentifier(hasSuggestion ? "report-result-suggested" : "report-result-new")
                    .disabled(model.isBusy)
                }
            }.frame(minHeight: 44)
            HStack(spacing: 8) {
                Button { edit() } label: {
                    Text(resultLabel)
                        .foregroundStyle(observation["browse_issues"].arrayValue.contains {
                            $0["code"] == .string("negative_with_unit")
                        } ? Color.yellow : Color.primary)
                        .fixedSize(horizontal: false, vertical: true)
                        .frame(maxWidth: .infinity, alignment: .leading)
                }.buttonStyle(.plain)
                Button { showSource(sourcePage) } label: {
                    Image(systemName: MobileSymbol.name("doc.text.magnifyingglass"))
                        .frame(minWidth: 44, minHeight: 44)
                }
                .accessibilityLabel(Text("View source · page \(sourcePage)"))
                #if !os(Android)
                .accessibilityHint("Opens the original report page")
                #endif
                let status = observation["status"].stringValue
                if let changeStatus, status == "pending" {
                    Button(action: changeStatus) {
                        Text(needsManualConfirmation ? "Review" : "Confirm")
                            .font(.caption.weight(.semibold))
                            .foregroundStyle(needsManualConfirmation ? Color.yellow : Color.accentColor)
                            .padding(.horizontal, 12)
                            .padding(.vertical, 6)
                            .background((needsManualConfirmation ? Color.yellow : Color.accentColor).opacity(0.12), in: Capsule())
                            .frame(minHeight: 44)
                    }
                    .accessibilityIdentifier("report-result-status-action")
                    .disabled(model.isBusy)
                }
            }
            let suggestion = ReportReview.suggestionNote(observation)
            if !suggestion.isEmpty {
                Text("OCR suggestion: \(suggestion)").font(.caption).foregroundStyle(.secondary)
            }
            let name = observation["lab_test_name"].stringValue
            if !name.isEmpty {
                Text("Recommended name: \(name)").font(.caption).foregroundStyle(.secondary)
                    .accessibilityIdentifier("report-result-loinc-name")
            }
            ForEach(Array(observation["browse_issues"].arrayValue.enumerated()), id: \.offset) { _, issue in
                if issue["code"] == .string("ocr_mapping_uncertain")
                    || issue["code"] == .string("unmapped_metric") {
                    EmptyView()
                } else if issue["scope"] == .string("browse") {
                    Button { chooseMetric() } label: {
                        HStack(alignment: .firstTextBaseline) {
                            issueLabel(issue)
                            Image(systemName: MobileSymbol.name("chevron.right"))
                        }.font(.caption).foregroundStyle(.orange)
                    }.buttonStyle(.plain).accessibilityIdentifier("report-result-issue")
                } else {
                    issueLabel(issue).accessibilityIdentifier("report-result-issue")
                }
            }
            if observation["status"] == .string("confirmed"), observation["payload"]["metric_id"] == .string("glucose") {
                Button("Save to \(MobilePlatform.healthProviderName)") { writeHealth() }.font(.caption).disabled(model.isBusy)
            }
        }.buttonStyle(.borderless).padding(.vertical, 5)
    }

    private func issueLabel(_ issue: JSONValue) -> some View {
        Label(issue["message"].stringValue, systemImage: MobileSymbol.name("exclamationmark.triangle"))
            .font(.caption).foregroundStyle(Color.yellow)
            .fixedSize(horizontal: false, vertical: true)
    }

}
