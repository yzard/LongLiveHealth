import UIKit
import Foundation
@preconcurrency import HealthKit

/// HealthKit supplies a non-Sendable callback. Transfer it through a locked,
/// one-shot owner so delivery and cancellation can never acknowledge it twice.
nonisolated final class HealthDeliveryCompletion: @unchecked Sendable {
    private let lock = NSLock()
    private var callback: (() -> Void)?
    init(_ callback: @escaping () -> Void) { self.callback = callback }
    func finish() {
        lock.lock()
        let action = callback
        callback = nil
        lock.unlock()
        action?()
    }
}

/// Owns the finite background allowance, including its expiration path.
@MainActor final class AppExecutionLease {
    private var identifier: UIBackgroundTaskIdentifier = .invalid
    func begin(name: String, expiration: @escaping @MainActor () -> Void) {
        identifier = UIApplication.shared.beginBackgroundTask(withName: name) { [weak self] in
            expiration()
            self?.end()
        }
    }
    func end() {
        guard identifier != .invalid else { return }
        let previous = identifier
        identifier = .invalid
        UIApplication.shared.endBackgroundTask(previous)
    }
}

@MainActor final class HealthBackgroundDelivery {
    private let store = HKHealthStore()
    private var queries: [HKObserverQuery] = []
    private var completions: [HealthDeliveryCompletion] = []
    private var deliveryTask: Task<Void, Never>?
    private var enabledTypes: Set<String> = []
    private(set) var registrationError: String?

    func register(model: AppModel) {
        guard queries.isEmpty, HKHealthStore.isHealthDataAvailable() else { return }
        // Install observers synchronously at application launch, before any delivery.
        for definition in HealthCatalog.definitions where definition.kind != "medication_dose_event" && !(definition.type is HKCorrelationType) && !(definition.type is HKClinicalType) && !definition.type.requiresPerObjectAuthorization() {
            let query = HKObserverQuery(sampleType: definition.type, predicate: nil) { [weak self, weak model] _, completion, error in
                let acknowledgement = HealthDeliveryCompletion(completion)
                Task { @MainActor in
                    guard let self, let model, error == nil else { acknowledgement.finish(); return }
                    self.completions.append(acknowledgement)
                    guard self.deliveryTask == nil else { return }
                    self.deliveryTask = Task {
                        await model.automaticHealthSync()
                        let callbacks = self.completions
                        self.completions = []
                        self.deliveryTask = nil
                        for callback in callbacks { callback.finish() }
                    }
                }
            }
            queries.append(query)
            store.execute(query)
        }
    }

    func enable() async {
        guard HKHealthStore.isHealthDataAvailable() else { return }
        for definition in HealthCatalog.definitions where definition.kind != "medication_dose_event" && !(definition.type is HKCorrelationType) && !(definition.type is HKClinicalType) && !definition.type.requiresPerObjectAuthorization() && !enabledTypes.contains(definition.type.identifier) {
            guard !Task.isCancelled else { return }
            do {
                try await store.enableBackgroundDelivery(for: definition.type, frequency: .hourly)
                enabledTypes.insert(definition.type.identifier)
            } catch {
                guard !Task.isCancelled else { return }
                registrationError = "Some Health types cannot wake the app: \(error.localizedDescription)"
            }
        }
    }
}

@MainActor final class HealthAppDelegate: NSObject, UIApplicationDelegate {
    let model = AppModel()
    func application(_ application: UIApplication, didFinishLaunchingWithOptions launchOptions: [UIApplication.LaunchOptionsKey: Any]?) -> Bool {
        BackgroundTransfer.shared.reconnect()
        model.backgroundHealth.register(model: model)
        return true
    }
    func application(_ application: UIApplication, handleEventsForBackgroundURLSession identifier: String, completionHandler: @escaping () -> Void) {
        BackgroundTransfer.shared.handleEvents(identifier: identifier) {
            Task { @MainActor in await self.model.sendDrafts(automatic: true) }
            completionHandler()
        }
    }
    func applicationProtectedDataWillBecomeUnavailable(_ application: UIApplication) {
        model.suspendHealthSync()
    }
    func applicationProtectedDataDidBecomeAvailable(_ application: UIApplication) {
        Task { await model.automaticHealthSync() }
    }
}
