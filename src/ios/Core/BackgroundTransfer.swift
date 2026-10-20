#if os(iOS)
import Foundation
import CryptoKit

/// The system owns file uploads while the app is suspended. App checkpoints still
/// own business acknowledgement and retries; the server's stable IDs prevent duplicates.
@MainActor public final class BackgroundTransfer {
    public static let shared = BackgroundTransfer()
    public static let identifier = "dev.helpyourself.ios.transfers"
    private var generation = UUID()
    private var activeSession: URLSession?
    private var eventsFinished = false
    private var waiters: [String: [CheckedContinuation<TransferReceipt, Error>]] = [:]
    private var eventCompletion: (() -> Void)?
    private var deadlines: [String: Task<Void, Never>] = [:]
    private var session: URLSession {
        if let activeSession { return activeSession }
        let configuration = URLSessionConfiguration.background(withIdentifier: Self.identifier)
        configuration.isDiscretionary = false
        configuration.sessionSendsLaunchEvents = true
        configuration.waitsForConnectivity = true
        configuration.timeoutIntervalForRequest = 120
        configuration.timeoutIntervalForResource = 24 * 60 * 60
        configuration.httpShouldSetCookies = false
        let created = URLSession(configuration: configuration, delegate: TransferDelegate(generation: generation), delegateQueue: nil)
        activeSession = created
        return created
    }
    public func cancelAll() throws {
        generation = UUID()
        activeSession?.invalidateAndCancel()
        activeSession = nil
        for key in Array(waiters.keys) { fail(CancellationError(), key: key) }
        try archive().clear()
    }

    public func reconnect() { _ = session }
    public func handleEvents(identifier: String, completion: @escaping () -> Void) {
        guard identifier == Self.identifier else { completion(); return }
        eventCompletion = completion
        reconnect()
        if eventsFinished { finishEvents() }
    }
    fileprivate func finishEvents() {
        guard eventCompletion != nil else { eventsFinished = true; return }
        eventsFinished = false
        let completion = eventCompletion
        eventCompletion = nil
        completion?()
    }
    private func archive() throws -> LocalArchive {
        let root = try FileManager.default.url(for: .applicationSupportDirectory, in: .userDomainMask, appropriateFor: nil, create: true)
        return try LocalArchive(directory: root.appendingPathComponent("background-transfers"))
    }
    private func key(for request: URLRequest, operation: String) throws -> String {
        guard let url = request.url else { throw APIError.invalidResponse }
        let identity = url.absoluteString + "|" + (request.value(forHTTPHeaderField: "Authorization") ?? "") + "|" + operation
        return SHA256.hash(data: Data(identity.utf8)).map { String(format: "%02x", $0) }.joined()
    }
    public func upload(_ request: URLRequest, body: Data, operation: String) async throws -> (Data, URLResponse) {
        let key = try key(for: request, operation: operation)
        let receipt: TransferReceipt = try await withCheckedThrowingContinuation { continuation in
            if waiters[key] != nil { waiters[key]?.append(continuation); return }
            waiters[key] = [continuation]
            deadlines[key] = Task {
                do { try await Task.sleep(for: .seconds(120)) }
                catch { return }
                // Stop blocking the UI; the system upload and durable business draft remain.
                fail(URLError(.timedOut), key: key)
            }
            Task {
                do {
                    let archive = try archive()
                    if let saved = try archive.load(TransferReceipt.self, name: "\(key).json") {
                        deliver(saved, key: key)
                        return
                    }
                    let tasks = await session.allTasks
                    // A system completion may have arrived during task discovery.
                    guard waiters[key] != nil else { return }
                    if let saved = try archive.load(TransferReceipt.self, name: "\(key).json") {
                        deliver(saved, key: key)
                    } else if let existing = tasks.first(where: { $0.taskDescription == key }) {
                        if existing.state == .suspended { existing.resume() }
                    } else {
                        try archive.saveBytes(body, name: "\(key).body")
                        let task = session.uploadTask(with: request, fromFile: try archive.location("\(key).body"))
                        task.taskDescription = key
                        task.resume()
                    }
                } catch { fail(error, key: key) }
            }
        }
        if let code = receipt.errorCode { throw URLError(URLError.Code(rawValue: code)) }
        guard let url = request.url, let status = receipt.status,
              let response = HTTPURLResponse(url: url, statusCode: status, httpVersion: "HTTP/1.1", headerFields: nil) else { throw APIError.invalidResponse }
        return (receipt.bytes, response)
    }
    fileprivate func complete(key: String, receipt: TransferReceipt, generation: UUID) {
        guard generation == self.generation else { return }
        guard key.count == 64, key.allSatisfy({ $0.isHexDigit }) else { return }
        do {
            // Commit the result before acknowledging the system's background events.
            let archive = try archive()
            try archive.save(receipt, name: "\(key).json")
            try archive.remove(name: "\(key).body")
            if waiters[key] != nil { deliver(receipt, key: key) }
        } catch { fail(error, key: key) }
    }
    private func deliver(_ receipt: TransferReceipt, key: String) {
        do {
            let archive = try archive()
            try archive.remove(name: "\(key).body")
            try archive.remove(name: "\(key).json")
            deadlines.removeValue(forKey: key)?.cancel()
            let callbacks = waiters.removeValue(forKey: key) ?? []
            for callback in callbacks { callback.resume(returning: receipt) }
        } catch { fail(error, key: key) }
    }
    private func fail(_ error: Error, key: String) {
        deadlines.removeValue(forKey: key)?.cancel()
        let callbacks = waiters.removeValue(forKey: key) ?? []
        for callback in callbacks { callback.resume(throwing: error) }
    }
}

private nonisolated struct TransferReceipt: Codable, Sendable {
    let status: Int?
    let bytes: Data
    let errorCode: Int?
}

/// URLSession invokes this delegate serially. The lock also protects the event
/// drain barrier while completion receipts are persisted on the main actor.
private nonisolated final class TransferDelegate: NSObject, URLSessionDataDelegate, @unchecked Sendable {
    private let generation: UUID
    init(generation: UUID) { self.generation = generation }
    private let lock = NSLock()
    private var responses: [Int: Data] = [:]
    private var oversized: Set<Int> = []
    private let completions = DispatchGroup()
    func urlSession(_ session: URLSession, dataTask: URLSessionDataTask, didReceive data: Data) {
        lock.lock()
        if (responses[dataTask.taskIdentifier]?.count ?? 0) + data.count > 1024 * 1024 {
            oversized.insert(dataTask.taskIdentifier)
            lock.unlock()
            dataTask.cancel()
            return
        }
        responses[dataTask.taskIdentifier, default: Data()].append(data)
        lock.unlock()
    }
    func urlSession(_ session: URLSession, task: URLSessionTask, didCompleteWithError error: Error?) {
        guard let key = task.taskDescription else { return }
        lock.lock()
        let bytes = responses.removeValue(forKey: task.taskIdentifier) ?? Data()
        let exceeded = oversized.remove(task.taskIdentifier) != nil
        lock.unlock()
        let receipt = TransferReceipt(status: (task.response as? HTTPURLResponse)?.statusCode, bytes: bytes,
            errorCode: exceeded ? URLError.dataLengthExceedsMaximum.rawValue : error.map { ($0 as NSError).code })
        completions.enter()
        Task { @MainActor in
            BackgroundTransfer.shared.complete(key: key, receipt: receipt, generation: generation)
            completions.leave()
        }
    }
    func urlSessionDidFinishEvents(forBackgroundURLSession session: URLSession) {
        completions.notify(queue: .main) {
            Task { @MainActor in BackgroundTransfer.shared.finishEvents() }
        }
    }
    func urlSession(_ session: URLSession, task: URLSessionTask, willPerformHTTPRedirection response: HTTPURLResponse,
                    newRequest request: URLRequest, completionHandler: @escaping (URLRequest?) -> Void) {
        completionHandler(nil)
    }
}
#endif
