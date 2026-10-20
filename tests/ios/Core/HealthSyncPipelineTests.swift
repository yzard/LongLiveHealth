import Testing
@testable import LongLiveHealthCore

private enum PipelineFailure: Error { case read, upload }

@MainActor @Test(.timeLimit(.minutes(1))) func pipelineOverlapsReadWithUploadAndKeepsOrder() async throws {
    var readStarted = false
    var releaseRead: CheckedContinuation<Void, Never>?
    var uploadWaiting: CheckedContinuation<Void, Never>?
    var uploaded: [Int] = []
    try await HealthSyncPipeline.run(first: 1, next: { page in
        guard page == 1 else { return nil }
        readStarted = true
        uploadWaiting?.resume()
        uploadWaiting = nil
        await withCheckedContinuation { releaseRead = $0 }
        return 2
    }, consume: { page in
        if page == 1 {
            if !readStarted { await withCheckedContinuation { uploadWaiting = $0 } }
            #expect(readStarted)
            #expect(uploaded.isEmpty)
            releaseRead?.resume()
            releaseRead = nil
        }
        uploaded.append(page)
    })
    #expect(uploaded == [1, 2])
}

@MainActor @Test func pipelineCommitsCurrentPageBeforeReportingPrefetchFailure() async {
    var committed: [Int] = []
    do {
        try await HealthSyncPipeline.run(first: 1, next: { _ in throw PipelineFailure.read }, consume: { committed.append($0) })
        Issue.record("Expected read failure")
    } catch { #expect(error is PipelineFailure) }
    #expect(committed == [1])
}

@MainActor @Test(.timeLimit(.minutes(1))) func failedUploadCancelsPrefetchWithoutConsumingIt() async {
    var cancelled = false
    var consumed: [Int] = []
    do {
        try await HealthSyncPipeline.run(first: 1, next: { _ in
            do {
                try await Task.sleep(for: .seconds(30))
                return 2
            } catch {
                cancelled = Task.isCancelled
                throw error
            }
        }, consume: { page in
            consumed.append(page)
            throw PipelineFailure.upload
        })
        Issue.record("Expected upload failure")
    } catch { #expect(error is PipelineFailure) }
    #expect(cancelled)
    #expect(consumed == [1])
}

@MainActor @Test func cancelledPipelineDoesNotUploadOrRead() async {
    var consumed = false
    var read = false
    let task = Task { @MainActor in
        try await HealthSyncPipeline.run(first: 1, next: { _ in read = true; return nil }, consume: { _ in consumed = true })
    }
    task.cancel()
    do { try await task.value; Issue.record("Expected cancellation") }
    catch { #expect(error is CancellationError) }
    #expect(!consumed && !read)
}
