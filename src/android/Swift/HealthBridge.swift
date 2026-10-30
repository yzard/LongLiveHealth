import Foundation
import Observation
import SkipFuse

@MainActor @Observable final class HealthBridge {
    var status = "Not connected"
    var isSyncing = false
    var activity: HealthSyncActivity?
    var coverage: [JSONValue] = []
    var totalRecords: Int?

    static func isProtectedDataError(_ error: Error) -> Bool { !MobilePlatform.protectedDataAvailable }

    func writeReviewedGlucose(_ observation: JSONValue, server: URL, userID: String, sampledAt: Date) async throws {
        guard observation["status"] == .string("confirmed"), observation["payload"]["metric_id"] == .string("glucose"),
              let value = Double(observation["payload"]["raw_result"].stringValue), value.isFinite, value >= 0 else { throw APIError.invalidResponse }
        _ = try await AndroidSystem.perform("health.writeGlucose", input: .object([
            "value": .number(value), "unit": observation["payload"]["raw_unit"], "at": .number(sampledAt.timeIntervalSince1970),
            "identity": .string("\(server.absoluteString)|\(userID)|\(observation["observation_id"].stringValue)"),
            "version": observation["revision"]
        ]))
    }

    func synchronize(client: APIClient, archive: LocalArchive, requestAccess: Bool, requestSelectedAccess: Bool) async throws {
        guard !isSyncing else { return }
        isSyncing = true
        defer { isSyncing = false; activity = nil }
        if requestAccess {
            _ = try await AndroidSystem.perform("health.authorize", input: .object([:]))
        }
        let available = try await AndroidSystem.perform("health.permissions", input: .object([:]))["types"].arrayValue.map(\.stringValue)
        guard !available.isEmpty else { throw APIError.status(403, "Allow health data access in Health Connect before syncing") }
        try archive.save(true, name: "health-enabled.json")
        let installation = try archive.load(UUID.self, name: "installation.json") ?? UUID()
        try archive.save(installation, name: "installation.json")
        let connection = try await client.post("health/connect", body: .object([
            "platform": .string("health_connect"), "installation_id": .string(installation.uuidString)
        ]))
        let connectionID = connection["connection_id"].stringValue
        guard !connectionID.isEmpty else { throw APIError.invalidResponse }
        activity = HealthSyncActivity()
        let pendingName = "pending-health-connect.json"
        func send(_ block: HealthSyncBlock) async throws {
            try archive.save(block, name: pendingName)
            activity?.beginSending(records: block.request["records"].arrayValue.count, at: Date())
            defer { activity?.endSending() }
            _ = try await client.post("health/sync", body: block.request)
            try await block.commit(to: archive)
            try archive.remove(name: pendingName)
            activity?.confirmSending()
        }
        if let pending = try archive.load(HealthSyncBlock.self, name: pendingName) { try await send(pending) }
        var batch = HealthSyncBatch(connectionID: connectionID)
        for kind in available {
            status = "Syncing — \(kind.replacingOccurrences(of: "_", with: " "))"
            let checkpoint = "health-connect-\(kind).json"
            var cursor = try archive.load(JSONValue.self, name: checkpoint) ?? .object([:])
            if cursor["token"].stringValue.isEmpty && cursor["initial_token"].stringValue.isEmpty {
                // Capture changes before the historical scan so inserts during it cannot be lost.
                let token = try await AndroidSystem.perform("health.token", input: .object(["kind": .string(kind)]))["token"]
                cursor = .object(["initial_token": token, "until": .string(Date().ISO8601Format()), "page": .string("")])
                try archive.save(cursor, name: checkpoint)
            }
            var more = true
            while more {
                try Task.checkCancellation()
                activity?.readingSince = Date()
                let initial = cursor["token"].stringValue.isEmpty
                var input = cursor
                input["kind"] = .string(kind)
                let page = try await AndroidSystem.perform(initial ? "health.page" : "health.changes", input: input)
                activity?.readingSince = nil
                if page["expired"].boolValue {
                    // Persist all acknowledged progress before replacing an expired provider cursor.
                    if let block = batch.finish() { try await send(block) }
                    let token = try await AndroidSystem.perform("health.token", input: .object(["kind": .string(kind)]))["token"]
                    cursor = .object(["initial_token": token, "until": .string(Date().ISO8601Format()), "page": .string("")])
                    try archive.save(cursor, name: checkpoint)
                    continue
                }
                if initial {
                    more = !page["page"].stringValue.isEmpty
                    cursor["page"] = page["page"]
                    if !more { cursor = .object(["token": cursor["initial_token"]]); more = true }
                } else {
                    cursor = .object(["token": page["token"]])
                    more = page["more"].boolValue
                }
                let blocks = try await batch.append(recordType: kind, status: page["records"].arrayValue.isEmpty ? "no_visible_samples" : "observed", records: page["records"].arrayValue, writes: [checkpoint: cursor])
                for block in blocks { try await send(block) }
                if initial && page["page"].stringValue.isEmpty, let block = batch.finish() { try await send(block) }
            }
        }
        if let block = batch.finish() { try await send(block) }
        let response = try await client.post("health/coverage", body: .object([:]))
        totalRecords = try HealthSyncProgress.totalRecords(in: response)
        coverage = response["coverage"].arrayValue
        status = "Up to date"
    }
}

@MainActor final class HealthBackgroundDelivery {
    var registrationError: String?
    func enable() async {
        do {
            _ = try AndroidSystem.invoke("background.schedule", "")
            registrationError = nil
        } catch { registrationError = error.localizedDescription }
    }
}
