/// One upload and at most one prefetched page. Only the consumer commits checkpoints.
@MainActor public enum HealthSyncPipeline {
    public static func run<Page: Sendable>(
        first: Page?,
        next: @escaping @MainActor @Sendable (Page) async throws -> Page?,
        consume: @escaping @MainActor @Sendable (Page) async throws -> Void
    ) async throws {
        var current = first
        while let page = current {
            try Task.checkCancellation()
            // Structured lifetime cancels and joins the read if upload fails or is paused.
            async let prefetched = next(page)
            try await consume(page)
            // A read error must not discard the successful current upload/checkpoint.
            current = try await prefetched
        }
    }
}
