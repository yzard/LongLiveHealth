import Foundation

/// Current-type work, separate from the server's cumulative unique archive count.
public nonisolated struct HealthSyncActivity: Sendable {
    public var readingSince: Date?
    public private(set) var sendingSince: Date?
    public private(set) var sendingRecords = 0
    public private(set) var confirmedRecords = 0

    public init() {}

    public mutating func beginSending(records: Int, at: Date) {
        sendingRecords = records
        sendingSince = at
    }

    public mutating func confirmSending() {
        confirmedRecords += sendingRecords
    }

    public mutating func endSending() {
        sendingSince = nil
        sendingRecords = 0
    }

    public func lines(at now: Date) -> [String] {
        var lines: [String] = []
        if let readingSince {
            lines.append("Reading from Apple Health · \(seconds(since: readingSince, now: now))s")
        }
        if let sendingSince {
            let action = sendingRecords == 0 ? "Updating sync checkpoint" : "Sending \(sendingRecords.formatted()) records"
            lines.append("\(action) · waiting for server · \(seconds(since: sendingSince, now: now))s")
        }
        lines.append("This type: \(confirmedRecords.formatted()) records acknowledged this sync")
        lines.append("Existing records do not increase the archive total above.")
        return lines
    }

    private func seconds(since start: Date, now: Date) -> Int {
        max(0, Int(now.timeIntervalSince(start)))
    }
}
