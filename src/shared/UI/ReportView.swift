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
    @State var confirmingAll = false
    @State var confirmationError: String?
    @Environment(\.scenePhase) var scenePhase
    @Environment(\.dismiss) var dismiss

    struct EditTarget: Identifiable { let id = UUID(); let observation: JSONValue }
    var body: some View {
        List {
            Section {
                Button { contextVisible = true } label: {
                    HStack {
                        Text("Report Info")
                        Spacer()
                        Text(document["context"]["source_name"].stringValue.isEmpty ? "Set source name" : document["context"]["source_name"].stringValue)
                            .foregroundStyle(.secondary)
                        Image(systemName: MobileSymbol.name("chevron.right")).font(.caption).foregroundStyle(.secondary)
                    }
                }.accessibilityIdentifier("report-info")
                LabeledContent("Total Markers", value: "\(document["observations"].arrayValue.count)")
                    .accessibilityIdentifier("report-marker-count")
                    .accessibilityValue("\(document["observations"].arrayValue.count)")
                Button { Task { await confirmAll() } } label: {
                    HStack {
                        Text(confirmingAll ? "Confirming…" : "Confirm All")
                        Spacer()
                        if confirmingAll { ProgressView() }
                        else { Text("\(ReportReview.pendingChanges(in: document).count) pending").foregroundStyle(.secondary) }
                    }
                }
                .accessibilityIdentifier("confirm-all-results")
                .disabled(confirmingAll || extracting || document == .null || ReportReview.pendingChanges(in: document).isEmpty)
                if let confirmationError { Text(confirmationError).foregroundStyle(.red) }
                HStack(spacing: 12) {
                    Button { showSource(page: 1) } label: { reportAction("View Original", icon: "doc.text.magnifyingglass") }
                        .accessibilityLabel("View original report").accessibilityIdentifier("report-original")
                    Button { editor = EditTarget(observation: .null) } label: { reportAction("Add Result", icon: "plus") }
                        .accessibilityLabel("Add result manually").accessibilityIdentifier("report-add-result")
                    Button(role: .destructive) { deleting = true } label: { reportAction("Delete", icon: "trash") }
                        .accessibilityLabel("Delete report").accessibilityIdentifier("report-delete")
                }.buttonStyle(.borderless)
                .disabled(document == .null || model.isBusy)
                if let job = model.jobs.first(where: { $0["file_id"].stringValue == reportID }), job["status"] == .string("failed") {
                    Button("Retry failed extraction") { Task { await model.perform { if let client = model.client { _ = try await model.request("jobs/retry", body: .object(["job_id": job["job_id"]])) } }; await model.refresh(automatic: false); await reload() } }
                }
                if !document["relation"]["preferred_report_id"].stringValue.isEmpty {
                    Text("Excluded from trends: \(document["relation"]["kind"].stringValue). Original data is retained.").font(.caption)
                    Button("Treat as independent report") { Task { await unlink() } }
                }
            }
            if !document["validation_issues"].arrayValue.isEmpty {
                Section("OCR incomplete") {
                    ForEach(Array(document["validation_issues"].arrayValue.enumerated()), id: \.offset) { _, issue in
                        VStack(alignment: .leading, spacing: 4) {
                            Text(issue["item_name"].stringValue.isEmpty ? "Item \(Int(issue["item_index"].numberValue ?? 0))" : issue["item_name"].stringValue).font(.headline)
                            Text("Page \(Int(issue["page"].numberValue ?? 0)) · \(issue["reason"].stringValue == "invalid_date" ? "Invalid" : "Missing") \(issue["field"].stringValue.replacingOccurrences(of: "_", with: " "))")
                                .foregroundStyle(.red)
                        }
                    }
                }
            }
            Section("Review results") {
                if document["observations"].arrayValue.isEmpty { Text("No extracted results yet.").foregroundStyle(.secondary)
                    InfoHelp(title: "Manual results", message: "You can add results manually while OCR is disabled or processing.") }
                ForEach(document["observations"].arrayValue, id: \.identifier) { observation in
                    ReportObservationRow(observation: observation, isBusy: model.isBusy,
                        edit: { editor = EditTarget(observation: observation) },
                        showSource: { showSource(page: $0) },
                        writeHealth: { healthWriter = EditTarget(observation: observation) })
                }
            }
            if !document["pages"].arrayValue.isEmpty {
                Section("Page processing") {
                    ForEach(Array(document["pages"].arrayValue.enumerated()), id: \.offset) { _, page in
                        NavigationLink("Page \(Int(page["page"].numberValue ?? 0)) · \(page["status"].stringValue)") {
                            ScrollView { CopyableText(page["content"].stringValue).font(.caption).padding() }
                                .navigationTitle("Page processing")
                        }
                    }
                }
            }
            if !document["extraction_inputs"].arrayValue.isEmpty {
                Section("Document evidence") {
                    ForEach(Array(document["extraction_inputs"].arrayValue.enumerated()), id: \.offset) { _, input in
                        NavigationLink("Page \(Int(input["page"].numberValue ?? 0)) · text layer: \(input["text_status"].stringValue.isEmpty ? "image only" : input["text_status"].stringValue)") {
                            DocumentEvidenceView(model: model, reportID: reportID, input: input)
                        }
                    }
                }
            }
            if !document["extraction_outputs"].arrayValue.isEmpty {
                Section("Original OCR output") {
                    ForEach(Array(document["extraction_outputs"].arrayValue.enumerated()), id: \.offset) { _, output in
                        NavigationLink("Page \(Int(output["page"].numberValue ?? 0)) · \(output["stage"].stringValue)") {
                            ExtractionOutputView(model: model, reportID: reportID, output: output)
                        }
                    }
                }
            }
        }
        .disabled(confirmingAll)
        .navigationTitle(document["context"]["report_title"].stringValue.isEmpty ? (document["report"]["original_name"].stringValue.isEmpty ? "Report" : document["report"]["original_name"].stringValue) : document["context"]["report_title"].stringValue)
        .navigationBarTitleDisplayMode(.inline)
        .initialTask { await reload() }
        .task { await followExtraction() }
        .refreshable { await reload() }
        .sheet(item: $preview) { DocumentPreview(location: $0) }
        .sheet(item: $editor) { target in
            ObservationEditor(observation: target.observation, metrics: model.metrics, pageCount: Int(document["report"]["page_count"].numberValue ?? 1)) { change in
                try await save(observations: [change], context: .null)
            }
        }
        .sheet(item: $healthWriter) { target in ReviewedGlucoseWriter(model: model, observation: target.observation) }
        .sheet(isPresented: $contextVisible) { ContextEditor(context: document["context"]) { context in try await save(observations: [], context: context) } }
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
    private func reportAction(_ title: String, icon: String) -> some View {
        VStack(spacing: 5) {
            Image(systemName: MobileSymbol.name(icon)).font(.body)
            Text(title).font(.caption).lineLimit(1).minimumScaleFactor(0.8)
        }.frame(maxWidth: .infinity, minHeight: 44)
    }
    private var extracting: Bool {
        model.jobs.contains { $0["file_id"].stringValue == reportID && ["queued", "running"].contains($0["status"].stringValue) }
    }
    private func confirmAll() async {
        guard !confirmingAll, !extracting else { return }
        confirmingAll = true; confirmationError = nil
        defer { confirmingAll = false }
        do {
            // Use the existing atomic review endpoint; larger reports resume from remaining pending items.
            while true {
                let changes = Array(ReportReview.pendingChanges(in: document).prefix(2_000))
                guard !changes.isEmpty else { return }
                try Task.checkCancellation()
                try await save(observations: changes, context: .null)
            }
        } catch { confirmationError = error.localizedDescription }
    }
    private func followExtraction() async {
        while !Task.isCancelled {
            guard model.jobs.contains(where: { $0["file_id"].stringValue == reportID && ["queued", "running"].contains($0["status"].stringValue) }) else { return }
            do {
                try await Task.sleep(for: .seconds(4))
                guard scenePhase == .active, model.selectedTab == "Data", !confirmingAll, !contextVisible, editor == nil, preview == nil, !model.isLocked else { continue }
                guard let token = model.session?.token else { return }
                let response = try await model.request("jobs/list", body: .object(["limit": .number(100)]))
                guard !Task.isCancelled, model.session?.token == token else { return }
                model.jobs = response["jobs"].arrayValue
                let latest = try await model.report(reportID)
                guard !Task.isCancelled, model.session?.token == token, !confirmingAll, !contextVisible, editor == nil else { continue }
                if document != latest { document = latest }
                if let index = model.reports.firstIndex(where: { $0["report_id"].stringValue == reportID }) {
                    model.reports[index] = latest["report"]; model.reports[index]["context"] = latest["context"]
                }
            } catch { return }
        }
    }
    private func reload() async { do { document = try await model.report(reportID) } catch { model.errorMessage = error.localizedDescription } }
    private func save(observations: [JSONValue], context: JSONValue) async throws {
        guard let client = model.client else { throw APIError.invalidResponse }
        document = try await model.request("reports/review", body: .object(["report_id": .string(reportID), "expected_revision": document["report"]["revision"], "context": context, "observations": .array(observations)]))
        let savedDocument = document
        let savedToken = model.session?.token
        let cachedBytes = try await Task.detached(priority: .utility) { try JSONEncoder().encode(savedDocument) }.value
        guard model.session?.token == savedToken, !Task.isCancelled else { throw CancellationError() }
        try model.archive?.saveBytes(cachedBytes, name: "report-\(reportID).json")
        if let index = model.reports.firstIndex(where: { $0["report_id"].stringValue == reportID }) { model.reports[index] = document["report"]; model.reports[index]["context"] = document["context"] }
        try model.archive?.save(model.reports, name: "reports.json")
    }
    private func showSource(page: Int) { Task { do { preview = DocumentLocation(url: try await model.source(reportID), page: page) } catch { model.errorMessage = error.localizedDescription } } }
    private func unlink() async {
        await model.perform { guard let client = model.client else { return }
            _ = try await model.request("reports/relate", body: .object(["report_id": .string(reportID), "preferred_report_id": .null, "kind": .string("duplicate"), "expected_revision": document["report"]["revision"]]))
        }; await reload()
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
        }.padding() }.navigationTitle("Original OCR output").initialTask {
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
        }.navigationTitle("Save to \(MobilePlatform.healthProviderName)").toolbar { Button("Cancel") { dismiss() }.disabled(model.isBusy) } }
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
        }.navigationTitle("Document evidence").initialTask {
            do {
                guard let client = model.client else { return }
                evidence = try await model.request("reports/input/get", body: .object(["report_id": .string(reportID), "run_id": input["run_id"], "page": input["page"]]))
            } catch { self.error = error.localizedDescription }
        }
    }
}

struct ReportObservationRow: View {
    let observation: JSONValue
    let isBusy: Bool
    let edit: () -> Void
    let showSource: (Int) -> Void
    let writeHealth: () -> Void
    private var resultLabel: String {
        let printed = [observation["payload"]["raw_result"].stringValue,
                       observation["payload"]["raw_unit"].stringValue]
            .filter { !$0.isEmpty }.joined(separator: " ")
        let standardized = observation["interpretation"]["result"]["display"].stringValue
        return standardized.isEmpty ? printed : "\(printed) (\(standardized))"
    }
    private var sourcePage: Int { Int(observation["payload"]["source"]["page"].numberValue ?? 1) }

    var body: some View {
        VStack(alignment: .leading, spacing: 7) {
            HStack(alignment: .center, spacing: 8) {
                Button { edit() } label: {
                    VStack(alignment: .leading, spacing: 5) {
                        HStack {
                            Text(observation["payload"]["raw_name"].stringValue).font(.headline)
                            Spacer()
                            Text(observation["status"].stringValue.capitalized).font(.caption)
                        }
                        Text(resultLabel).foregroundStyle(.primary)
                            .fixedSize(horizontal: false, vertical: true)
                    }.frame(maxWidth: .infinity, alignment: .leading)
                }.buttonStyle(.plain)
                Button { showSource(sourcePage) } label: {
                    Image(systemName: MobileSymbol.name("doc.text.magnifyingglass"))
                        .frame(minWidth: 44, minHeight: 44)
                }
                .accessibilityLabel(Text("View source · page \(sourcePage)"))
                #if !os(Android)
                .accessibilityHint("Opens the original report page")
                #endif
            }
            if observation["status"] == .string("confirmed"), observation["payload"]["metric_id"] == .string("glucose") {
                Button("Save to \(MobilePlatform.healthProviderName)") { writeHealth() }.font(.caption).disabled(isBusy)
            }
        }.buttonStyle(.borderless).padding(.vertical, 5)
    }
}
