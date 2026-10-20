import SwiftUI

/// A retained screen fetches on its first appearance or when its query changes.
/// Returning from a sheet or another tab keeps the existing content.
private struct InitialTask<Key: Equatable & Sendable>: ViewModifier {
    let key: Key
    let operation: @MainActor () async -> Void
    @State private var loadedKey: Key?
    func body(content: Content) -> some View {
        content.task(id: key) {
            guard loadedKey != key else { return }
            await operation()
            if !Task.isCancelled { loadedKey = key }
        }
    }
}

extension View {
    func initialTask<Key: Equatable & Sendable>(id: Key, _ operation: @escaping @MainActor () async -> Void) -> some View {
        modifier(InitialTask(key: id, operation: operation))
    }
    func initialTask(_ operation: @escaping @MainActor () async -> Void) -> some View {
        initialTask(id: true, operation)
    }
}

/// Editors stay above their originating screen, including editors opened inside details.
struct EditorLink<Content: View>: View {
    let title: String
    @ViewBuilder let content: () -> Content
    @State private var presented = false
    init(_ title: String, @ViewBuilder content: @escaping () -> Content) { self.title = title; self.content = content }
    var body: some View {
        Button(title) { presented = true }
            .sheet(isPresented: $presented) { NavigationStack { content() } }
    }
}
