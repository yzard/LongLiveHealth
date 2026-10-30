/// One upload and at most one prefetched page. Only the consumer commits checkpoints.
@MainActor public enum HealthSyncPipeline {
    public static func run<Page: Sendable>(
        first: Page?,
        prefetch: @escaping @MainActor @Sendable (Page) -> Bool,
        next: @escaping @MainActor @Sendable (Page) async throws -> Page?,
        consume: @escaping @MainActor @Sendable (Page) async throws -> Void
    ) async throws {
        var current = first
        while let page = current {
            try Task.checkCancellation()
            // Structured lifetime cancels and joins the read if upload fails or is paused.
            if prefetch(page) {
                async let prefetched = next(page)
                try await consume(page)
                current = try await prefetched
            } else {
                try await consume(page)
                current = try await next(page)
            }
        }
    }
}
