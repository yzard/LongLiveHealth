import SwiftUI

struct ReportView: View {
    var model: AppModel
    let reportID: String
    @State var document: JSONValue = .null
    @State var ready: [JSONValue] = []
    @State var needsReview: [JSONValue] = []
    @State var submitted: [JSONValue] = []
    @State var editor: EditTarget?
    @State var healthWriter: EditTarget?
    @State var preview: DocumentLocation?
    @State var contextVisible = false
    @State var deleting = false
    @State var rescanning = false
    @State var rescanConfirmation = false
    @State var submittingAll = false
    @State var savingResult = false
    @State var submissionError: String?
    @State var readyVisible = false
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
    }
    var body: some View {
        reportList
        .disabled(submittingAll || rescanning || savingResult)
        .mobileNavigationTitle(document["context"]["report_title"].stringValue.isEmpty ? uiText("Blood Work") : document["context"]["report_title"].stringValue)
        .navigationBarTitleDisplayMode(.inline)
        .initialTask { await reload() }
        .task(id: model.jobs.first { $0["file_id"].stringValue == reportID }?["job_id"].stringValue) { await followExtraction() }
        #if !os(Android)
        .refreshable { await reload() }
        #endif
        .originalReportPreview($preview)
        .sheet(item: $editor) { target in reviewEditor(target) }
        .sheet(item: $healthWriter) { target in ReviewedGlucoseWriter(model: model, observation: target.observation) }
        .sheet(isPresented: $contextVisible) { ContextEditor(context: document["context"]) { context in try await save(observations: [], context: context) } }
        #if os(Android)
        .navigationDestination(isPresented: $readyVisible) {
            ReadyToSubmitView(model: model, reportID: reportID, document: $document,
                reviewEditor: reviewEditor, submitReadyResult: submitReadyResult)
        }
        #endif
        .alert(uiText("Rescan report?"), isPresented: $rescanConfirmation) {
            Button(uiText("Cancel"), role: .cancel) {}
            Button(uiText("Rescan"), role: .destructive) { Task { await rescan() } }
        } message: { Text(uiText("Rescan deletes all existing results and runs OCR again on the original report. New results will be pending review.")) }
        .alert(uiText("Delete report?"), isPresented: $deleting) {
            Button(uiText("Cancel"), role: .cancel) {}
            Button(uiText("Delete report"), role: .destructive) { Task { await model.perform {
                guard let client = model.client else { return }
                _ = try await model.request("reports/delete", body: .object(["report_id": .string(reportID), "expected_revision": document["report"]["revision"]]))
                try model.archive?.remove(name: "report-\(reportID).json"); try model.archive?.remove(name: "source-\(reportID)")
                model.reports.removeAll { $0["report_id"].stringValue == reportID }
                model.jobs.removeAll { $0["file_id"].stringValue == reportID }
                try model.archive?.save(model.reports, name: "reports.json")
                dismiss()
            } } }
        } message: { Text(uiText("This deletes the report, its submitted and draft results, and related analysis from your archive. The server then removes stored files. Offline backups you made are separate.")) }
    }
    private enum ReportRow: Identifiable {
        case info, markerCount, actions, retry, relationship, unlink, review, error, empty, ocrOutput
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
            case .error: "submission-error"
            case .empty: "empty-results"
            case .ocrOutput: "report-ocr-output"
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
        if submissionError != nil { review.append(.error) }
        if observations.isEmpty && validation.isEmpty { review.append(.empty) }

        let needsReviewRows = validation.map { ReportRow.validation($0) }
            + needsReview.map { ReportRow.observation($0) }
        var result = [
            ReportSection(id: "info", title: nil, rows: information),
            ReportSection(id: "review", title: uiText("Review Results"), rows: review)
        ]
        if !needsReviewRows.isEmpty {
            result.append(ReportSection(id: "manual", title: uiText("Needs Review"), rows: needsReviewRows))
        }
        if !submitted.isEmpty {
            result.append(ReportSection(id: "submitted", title: uiText("Submitted Results"),
                rows: submitted.map { ReportRow.observation($0) }))
        }
        if ["pages", "extraction_inputs", "extraction_outputs"].contains(where: { !document[$0].arrayValue.isEmpty }) {
            result.append(ReportSection(id: "ocr-output", title: nil, rows: [.ocrOutput]))
        }
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
                    case .summary:
                        VStack(alignment: .leading, spacing: 8) {
                            reportRow(.info)
                            reportRow(.markerCount)
                            reportRow(.actions)
                            reportSectionHeader("Review Results")
                            reportRow(.review)
                            if failedJob != nil { reportRow(.retry) }
                            if !document["relation"]["preferred_report_id"].stringValue.isEmpty {
                                reportRow(.relationship)
                                reportRow(.unlink)
                            }
                            if submissionError != nil { reportRow(.error) }
                            if document["observations"].arrayValue.isEmpty
                                && document["validation_issues"].arrayValue.isEmpty { reportRow(.empty) }
                        }
                        .padding(.horizontal, 16).padding(.vertical, 8)
                    case .header(_, let title):
                        reportSectionHeader(title)
                    case .row(let row, let last):
                        reportListRow(row, last: last)
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
        case summary
        case header(String, String?)
        case row(ReportRow, last: Bool)

        var id: String {
            switch self {
            case .summary: "summary"
            case .header(let id, _): "section-\(id)"
            case .row(let row, _): row.id
            }
        }
    }

    private var scrollItems: [ReportScrollItem] {
        var items: [ReportScrollItem] = [.summary]
        for section in sections where section.id != "info" && section.id != "review" {
            items.append(.header(section.id, section.title))
            for (index, row) in section.rows.enumerated() {
                items.append(.row(row, last: index == section.rows.count - 1))
            }
        }
        return items
    }

    @ViewBuilder private func reportSectionHeader(_ title: String?) -> some View {
        if let title {
            Text(uiLabel(title).uppercased()).font(.footnote).foregroundStyle(.secondary)
                .accessibilityAddTraits(.isHeader)
                .padding(.horizontal, 16).padding(.top, 24).padding(.bottom, 8)
        } else {
            Color.clear.frame(height: 24)
        }
    }

    private func reportListRow(_ row: ReportRow, last: Bool) -> some View {
        VStack(spacing: 0) {
            reportRow(row)
                .frame(maxWidth: .infinity, minHeight: 32, alignment: .leading)
                .padding(.horizontal, 16).padding(.vertical, 8)
            if !last { Divider() }
        }
        .background(Color(.systemBackground))
    }
    #endif

    @ViewBuilder private func reportRow(_ row: ReportRow) -> some View {
        switch row {
        case .info:
            Button { contextVisible = true } label: {
                HStack {
                    Text(uiText("Report Info"))
                    Spacer()
                    let source = document["context"]["source_name"].stringValue
                    Text(source.isEmpty ? uiText("Set source name") : source).foregroundStyle(.secondary)
                    Image(systemName: MobileSymbol.name("chevron.right")).font(.caption).foregroundStyle(.secondary)
                }
            }
            #if os(Android)
            .buttonStyle(.plain)
            #endif
            .accessibilityIdentifier("report-info")
        case .markerCount:
            let count = document["observations"].arrayValue.count
            HStack {
                Text(uiText("Total Markers"))
                Spacer()
                Text("\(count)").foregroundStyle(.secondary)
            }
                .accessibilityIdentifier("report-marker-count").accessibilityValue("\(count)")
        case .actions:
            reportActions
        case .retry:
            Button(uiText("Retry failed extraction")) { Task { await retryExtraction() } }
        case .relationship:
            Text(uiText("Excluded from trends: \(document["relation"]["kind"].stringValue). Original data is retained.")).font(.caption)
        case .unlink:
            Button(uiText("Treat as independent report")) { Task { await unlink() } }
        case .review:
            reviewActions
        case .error:
            if let submissionError { Text(uiMessage(submissionError)).foregroundStyle(.red) }
        case .empty:
            Text(uiText("No extracted results yet.")).foregroundStyle(.secondary)
        case .validation(let entry):
            validationRow(entry.value)
        case .observation(let observation):
            observationRow(observation)
        case .ocrOutput:
            NavigationLink {
                List { ocrDiagnostics }.mobileNavigationTitle(uiText("Raw OCR Output"))
            } label: {
                #if os(Android)
                HStack {
                    Text(uiText("Raw OCR Output"))
                    Spacer()
                    Image(systemName: MobileSymbol.name("chevron.right")).font(.caption).foregroundStyle(.secondary)
                }
                #else
                Text(uiText("Raw OCR Output"))
                #endif
            }.accessibilityIdentifier("report-ocr-output")
        }
    }

    private var reportActions: some View {
        HStack(spacing: 12) {
            Button { showSource(page: 1) } label: { reportAction(uiText("View Original"), icon: "doc.text.magnifyingglass") }
                .accessibilityLabel(Text(uiText("View original report"))).accessibilityIdentifier("report-original")
            Button { editor = EditTarget(observation: .null) } label: { reportAction(uiText("Add Result"), icon: "plus") }
                .accessibilityLabel(Text(uiText("Add result manually"))).accessibilityIdentifier("report-add-result")
            Button { rescanConfirmation = true } label: { reportAction(uiText("Rescan"), icon: "arrow.clockwise") }
                .accessibilityLabel(Text(uiText("Rescan report"))).accessibilityIdentifier("report-rescan")
                .disabled(extracting || rescanning)
            Button(role: .destructive) { deleting = true } label: { reportAction(uiText("Delete"), icon: "trash") }
                .accessibilityLabel(Text(uiText("Delete report"))).accessibilityIdentifier("report-delete")
        }.buttonStyle(.borderless)
        .disabled(document == .null || model.isBusy)
    }

    private var reviewActions: some View {
        let observations = ready
        return HStack {
            #if os(Android)
            Button { readyVisible = true } label: {
                Text(observations.count == 1 ? uiText("1 result ready to submit") : uiText("\(observations.count) results ready to submit"))
                    .foregroundStyle(.primary).frame(maxWidth: .infinity, alignment: .leading)
            }.buttonStyle(.borderless)
            .accessibilityIdentifier("ready-to-submit-results")
            .disabled(observations.isEmpty)
            #else
            NavigationLink {
                ReadyToSubmitView(model: model, reportID: reportID, document: $document,
                    reviewEditor: reviewEditor, submitReadyResult: submitReadyResult)
            } label: {
                Text(observations.count == 1 ? uiText("1 result ready to submit") : uiText("\(observations.count) results ready to submit"))
                    .foregroundStyle(.primary).frame(maxWidth: .infinity, alignment: .leading)
            }.buttonStyle(.borderless)
            .accessibilityIdentifier("ready-to-submit-results")
            .disabled(observations.isEmpty)
            #endif
            Spacer(minLength: 8)
            Button(submittingAll ? uiText("Submitting…") : uiText("Submit")) { Task { await submitAll() } }
                .buttonStyle(.borderless)
                .accessibilityIdentifier("submit-all-results")
                .disabled(submittingAll || extracting || observations.isEmpty)
        }
    }

    private func validationRow(_ issue: JSONValue) -> some View {
        let name = issue["item_name"].stringValue
        let field = issue["field"].stringValue.replacingOccurrences(of: "_", with: " ")
        let reason = issue["reason"] == .string("invalid_date") ? "Invalid" : "Missing"
        return VStack(alignment: .leading, spacing: 4) {
            Text(name.isEmpty ? uiText("Item \(Int(issue["item_index"].numberValue ?? 0))") : name).font(.headline)
            Text(uiText("Page \(Int(issue["page"].numberValue ?? 0)) · \(reason) \(field)")).foregroundStyle(.red)
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
            pageCount: Int(document["report"]["page_count"].numberValue ?? 1), source: { page in
                DocumentLocation(url: try await model.source(reportID), page: page)
            }, lookupLOINCName: { code in
                try await model.request("metrics/loinc/get", body: .object(["code": .string(code)]))["official_name"].stringValue
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
                if ReportReview.canBulkSubmit(observation) {
                    Task { await submitReadyResult(id) }
                } else {
                    editor = EditTarget(observation: observation)
                }
            },
            edit: { editor = EditTarget(observation: observation) },
            showSource: { showSource(page: $0) },
            writeHealth: { healthWriter = EditTarget(observation: observation) })
            .id("\(id.stringValue)-\(revision)-\(renderedStatus)")
    }

    @ViewBuilder private var ocrDiagnostics: some View {
        if !document["pages"].arrayValue.isEmpty {
            Section(uiText("Page processing")) {
                ForEach(indexedValues("pages")) { entry in
                    let page = entry.value
                    NavigationLink(uiText("Page \(Int(page["page"].numberValue ?? 0)) · \(page["status"].stringValue)")) {
                        ScrollView { CopyableText(page["content"].stringValue).font(.caption).padding() }
                            .mobileNavigationTitle(uiText("Page processing"))
                    }
                }
            }
        }
        if !document["extraction_inputs"].arrayValue.isEmpty {
            Section(uiText("Document evidence")) {
                ForEach(indexedValues("extraction_inputs")) { entry in
                    let input = entry.value
                    NavigationLink(uiText("Page \(Int(input["page"].numberValue ?? 0)) · text layer: \(input["text_status"].stringValue.isEmpty ? "image only" : input["text_status"].stringValue)")) {
                        DocumentEvidenceView(model: model, reportID: reportID, input: input)
                    }
                }
            }
        }
        if !document["extraction_outputs"].arrayValue.isEmpty {
            Section(uiText("Original OCR output")) {
                ForEach(indexedValues("extraction_outputs")) { entry in
                    let output = entry.value
                    NavigationLink(uiText("Page \(Int(output["page"].numberValue ?? 0)) · \(output["stage"].stringValue)")) {
                        ExtractionOutputView(model: model, reportID: reportID, output: output)
                    }
                }
            }
        }
    }
    private func reportAction(_ title: String, icon: String) -> some View {
        VStack(spacing: 5) {
            Image(systemName: MobileSymbol.name(icon)).font(.body)
            Text(uiLabel(title)).font(.caption).lineLimit(1)
            #if !os(Android)
                .minimumScaleFactor(0.8)
            #endif
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
            _ = adoptReport(try await model.report(reportID))
        }
    }
    private func submitAll() async {
        guard !submittingAll, !savingResult, !rescanning, !extracting else { return }
        submittingAll = true; submissionError = nil
        defer { submittingAll = false }
        do {
            // Use the existing atomic report submission endpoint; larger reports resume from remaining pending items.
            while true {
                let changes = document["observations"].arrayValue
                    .filter { ReportReview.canBulkSubmit($0) }.prefix(2_000)
                    .map { ReportReview.change($0, to: "submitted") }
                guard !changes.isEmpty else { return }
                try Task.checkCancellation()
                guard try await saveStatusChanges(changes) else { return }
            }
        } catch { submissionError = error.localizedDescription }
    }
    private func submitReadyResult(_ id: JSONValue) async {
        guard !submittingAll, !savingResult, !rescanning, !extracting,
              let current = document["observations"].arrayValue.first(where: { $0["observation_id"] == id }),
              ReportReview.canBulkSubmit(current) else { return }
        await model.perform {
            _ = try await saveStatusChanges([ReportReview.change(current, to: "submitted")])
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
                guard scenePhase == .active, model.selectedTab == "Data", !submittingAll, !savingResult, !contextVisible, editor == nil, preview == nil, !model.isLocked else { continue }
                guard let token = model.session?.token else { return }
                let response = try await model.request("jobs/list", body: .object(["limit": .number(100)]))
                guard !Task.isCancelled, model.session?.token == token else { return }
                model.jobs = response["jobs"].arrayValue
                let latest = try await model.report(reportID)
                guard !Task.isCancelled, model.session?.token == token, !submittingAll, !savingResult, !contextVisible, editor == nil else { continue }
                _ = adoptReport(latest)
            } catch { return }
        }
    }
    private func reload() async {
        do { _ = adoptReport(try await model.report(reportID)) }
        catch { model.errorMessage = error.localizedDescription }
    }
    private func save(observations: [JSONValue], context: JSONValue) async throws {
        guard model.client != nil, !savingResult else { throw APIError.invalidResponse }
        savingResult = true
        defer { savingResult = false }
        let request: JSONValue = .object(["report_id": .string(reportID), "expected_revision": document["report"]["revision"], "context": context, "observations": .array(observations)])
        let response = try await model.request("reports/submit", body: request)
        for metric in response["created_metrics"].arrayValue
            where !model.metrics.contains(where: { $0["metric_id"] == metric["metric_id"] }) {
            model.metrics.append(metric)
        }
        let updatedCodes = response["updated_metric_loinc"].objectValue
        if !updatedCodes.isEmpty {
            for index in model.metrics.indices {
                let id = model.metrics[index]["metric_id"].stringValue
                if let codes = updatedCodes[id] { model.metrics[index]["loinc"] = codes }
            }
        }
        try await applySavedReport(response)
    }

    private func adoptReport(_ snapshot: JSONValue) -> Bool {
        guard let revision = snapshot["report"]["revision"].numberValue,
              revision >= (document["report"]["revision"].numberValue ?? 0) else { return false }
        if document != snapshot {
            let observations = snapshot["observations"].arrayValue
            ready = observations.filter(ReportReview.canBulkSubmit)
            needsReview = observations.filter {
                $0["status"] == .string("pending") && !ReportReview.canBulkSubmit($0)
            }
            submitted = observations.filter { $0["status"] == .string("submitted") }
            document = snapshot
        }
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

struct ReadyToSubmitView: View {
    let model: AppModel
    let reportID: String
    @Binding var document: JSONValue
    let reviewEditor: (ReportView.EditTarget) -> ObservationEditor
    let submitReadyResult: (JSONValue) async -> Void
    @State var editor: ReportView.EditTarget?
    @State var healthWriter: ReportView.EditTarget?
    @State var preview: DocumentLocation?

    var body: some View {
        List {
            ForEach(document["observations"].arrayValue.filter(ReportReview.canBulkSubmit), id: \.identifier) { observation in
                ReportObservationRow(model: model, observation: observation,
                    changeStatus: { _ = Task { await submitReadyResult(observation["observation_id"]) } },
                    edit: { editor = ReportView.EditTarget(observation: observation) },
                    showSource: { page in _ = Task { await openSource(page) } },
                    writeHealth: { healthWriter = ReportView.EditTarget(observation: observation) })
            }
        }
        .mobileNavigationTitle(uiText("Ready to Submit"))
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
            DisclosureGroup(uiText("Complete provider response")) { CopyableText(original["response_body"].stringValue).font(.caption.monospaced()) }
        }.padding() }.mobileNavigationTitle(uiText("Original OCR output")).initialTask {
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
            Text(uiText("Report collection date: \(observation["payload"]["sampled_at"].stringValue)")).font(.caption)
            DatePicker(uiText("Collection date and time"), selection: $sampledAt)
            Toggle(uiText("I verified the collection date and time"), isOn: $verifiedTime)
            InfoHelp(title: uiText("Collection time"), message: uiText("\(MobilePlatform.healthProviderName) requires a timestamp. Check the original report before saving. The complete report and its revisions remain on your server."))
            if let error { Text(uiMessage(error)).foregroundStyle(.red) }
            Button(uiText("Save reviewed glucose")) { Task {
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
                            report = try await model.request("reports/submit", body: .object(["report_id": observation["report_id"], "expected_revision": report["report"]["revision"], "context": .null,
                                "observations": .array([.object(["observation_id": observation["observation_id"], "expected_revision": current["revision"], "status": .string("submitted"), "payload": desired])])]))
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
        }.mobileNavigationTitle(uiText("Save to \(uiLabel(MobilePlatform.healthProviderName))")).toolbar { Button(uiText("Cancel")) { dismiss() }.disabled(model.isBusy) } }
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
            if let error { Text(uiMessage(error)).foregroundStyle(.red) }
            if evidence == .null && error == nil { ProgressView() }
            else {
                Text(uiText("Text layer: \(evidence["text_layer"]["status"].stringValue.isEmpty ? "not applicable to image" : evidence["text_layer"]["status"].stringValue)"))
                if !evidence["error_code"].stringValue.isEmpty { Text(uiText("Text extraction issue: \(evidence["error_code"].stringValue)")).foregroundStyle(.orange) }
                InfoHelp(title: uiText("Recognition evidence"), message: uiText("The page image is always used for recognition. PDF text may be incomplete or disagree with the image."))
                CopyableText(evidence["text_layer"]["text"].stringValue)
                Text(uiText("Positioned words: \(evidence["text_layer"]["words"].arrayValue.count)")).font(.caption)
            }
        }.mobileNavigationTitle(uiText("Document evidence")).initialTask {
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
    let showSource: (Int) -> Void
    let writeHealth: () -> Void
    private var needsManualReview: Bool {
        observation["status"] == .string("pending")
            && !ReportReview.canBulkSubmit(observation)
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
        let status = observation["status"].stringValue
        let mappedMetricID = ReportReview.defaultMetricID(observation)
        let hasPossibleMatch = status == "pending" && mappedMetricID != nil
        let proposedNew = observation["payload"]["unmapped_choice"]["kind"] == .string("new")
        let isNew = mappedMetricID == nil
            && (proposedNew || observation["browse_issues"].arrayValue.contains { $0["code"] == .string("unmapped_metric") })
        VStack(alignment: .leading, spacing: 7) {
            HStack(spacing: 8) {
                Button { edit() } label: {
                    HStack(alignment: .firstTextBaseline, spacing: 8) {
                        Text(LaboratoryPresentation.resultTitle(observation, metrics: model.metrics, renamedMetrics: model.renamedLabMetrics))
                            .font(.headline)
                            .fixedSize(horizontal: false, vertical: true)
                        let code = observation["lab_test_code"].stringValue
                        if !code.isEmpty {
                            Text(uiText("LOINC \(code)"))
                                .font(.caption)
                                .foregroundStyle(.secondary)
                                .fixedSize()
                                .accessibilityIdentifier("report-result-loinc")
                        }
                    }
                }.buttonStyle(.plain)
                Spacer(minLength: 0)
                if hasPossibleMatch || isNew {
                    Button(action: edit) {
                        Text(hasPossibleMatch ? uiText("Possible match") : uiText("New"))
                            .font(.caption.weight(.semibold))
                            .foregroundStyle(Color.yellow)
                            .padding(.horizontal, 8)
                            .padding(.vertical, 4)
                            .background(Color.yellow.opacity(0.12), in: Capsule())
                            .frame(minHeight: 44)
                    }
                    .accessibilityIdentifier(hasPossibleMatch ? "report-result-possible-match" : "report-result-new")
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
                .accessibilityLabel(Text(uiText("View source · page \(sourcePage)")))
                #if !os(Android)
                .accessibilityHint(uiText("Opens the original report page"))
                #endif
                if let changeStatus, status == "pending" {
                    Button(action: changeStatus) {
                        Text(needsManualReview ? uiText("Review") : uiText("Submit"))
                            .font(.caption.weight(.semibold))
                            .foregroundStyle(needsManualReview ? Color.yellow : Color.accentColor)
                            .padding(.horizontal, 12)
                            .padding(.vertical, 6)
                            .background((needsManualReview ? Color.yellow : Color.accentColor).opacity(0.12), in: Capsule())
                            .frame(minHeight: 44)
                    }
                    .accessibilityIdentifier("report-result-status-action")
                    .disabled(model.isBusy)
                }
            }
            let suggestion = ReportReview.suggestionNote(observation, includeMetricName: !hasPossibleMatch)
            if !suggestion.isEmpty {
                Text(uiText("OCR suggestion: \(suggestion)")).font(.caption).foregroundStyle(.secondary)
            }
            let name = observation["lab_test_name"].stringValue
            if !name.isEmpty {
                Text(uiText("Recommended name: \(name)")).font(.caption).foregroundStyle(.secondary)
                    .accessibilityIdentifier("report-result-loinc-name")
            }
            ForEach(Array(observation["browse_issues"].arrayValue.enumerated()), id: \.offset) { _, issue in
                if issue["code"] == .string("ocr_mapping_uncertain")
                    || (hasPossibleMatch && issue["code"] == .string("metric_identity_uncertain"))
                    || issue["code"] == .string("unmapped_metric") {
                    EmptyView()
                } else if issue["scope"] == .string("browse") {
                    Button { edit() } label: {
                        HStack(alignment: .firstTextBaseline) {
                            issueLabel(issue)
                            Image(systemName: MobileSymbol.name("chevron.right"))
                        }.font(.caption).foregroundStyle(.orange)
                    }.buttonStyle(.plain).accessibilityIdentifier("report-result-issue")
                } else {
                    issueLabel(issue).accessibilityIdentifier("report-result-issue")
                }
            }
            if observation["status"] == .string("submitted"), observation["payload"]["metric_id"] == .string("glucose") {
                Button(uiText("Save to \(uiLabel(MobilePlatform.healthProviderName))")) { writeHealth() }.font(.caption).disabled(model.isBusy)
            }
        }.buttonStyle(.borderless).padding(.vertical, 5)
    }

    private func issueLabel(_ issue: JSONValue) -> some View {
        Label(uiMessage(issue["message"].stringValue), systemImage: MobileSymbol.name("exclamationmark.triangle"))
            .font(.caption).foregroundStyle(Color.yellow)
            .fixedSize(horizontal: false, vertical: true)
    }

}
