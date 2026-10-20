import Foundation

public nonisolated enum HealthSyncBatch {
    public static let maximumRecords = 1_000
    public static func requests(connectionID: String, recordType: String, coverageStatus: String, records: [JSONValue], maximumRecords: Int, maximumBytes: Int) throws -> [JSONValue] {
        func request(_ values: [JSONValue]) -> JSONValue {
            .object(["connection_id": .string(connectionID), "batch_id": .string(UUID().uuidString),
                     "record_type": .string(recordType), "coverage_status": .string(coverageStatus), "records": .array(values)])
        }
        guard maximumRecords > 0 else { throw APIError.invalidResponse }
        let encoder = JSONEncoder()
        let envelopeBytes = try encoder.encode(request([])).count
        guard envelopeBytes <= maximumBytes else { throw APIError.status(413, "Health batch metadata exceeds the server limit") }
        var pageBytes = envelopeBytes
        var batches: [JSONValue] = []
        var page: [JSONValue] = []
        for record in records {
            // Encode each record once; re-encoding the growing page is quadratic.
            let recordBytes = try encoder.encode(record).count
            guard recordBytes <= maximumBytes - envelopeBytes else {
                throw APIError.status(413, "A Health record exceeds the server limit")
            }
            let separatorBytes = page.isEmpty ? 0 : 1
            if page.count == maximumRecords || recordBytes + separatorBytes > maximumBytes - pageBytes {
                batches.append(request(page)); page = []; pageBytes = envelopeBytes
            }
            pageBytes += recordBytes + (page.isEmpty ? 0 : 1)
            page.append(record)
        }
        if !page.isEmpty || batches.isEmpty { batches.append(request(page)) }
        return batches
    }
}
