import Foundation
import Observation

@MainActor @Observable final class InterfaceLanguageSettings {
    static let shared = InterfaceLanguageSettings()
    private(set) var language: InterfaceLanguage

    private init() {
        language = InterfaceLanguage.stored ?? .systemDefault(MobilePlatform.systemLanguageIdentifier)
        language.persist()
        MobilePlatform.applyInterfaceLanguage(language)
    }

    func select(_ selection: InterfaceLanguage) {
        guard selection != language else { return }
        selection.persist()
        language = selection
        MobilePlatform.applyInterfaceLanguage(selection)
    }
}

@MainActor func uiText(_ phrase: InterfaceText) -> String {
    InterfaceCatalog.text(phrase, language: InterfaceLanguageSettings.shared.language)
}

@MainActor func uiLabel(_ english: String) -> String {
    InterfaceCatalog.label(english, language: InterfaceLanguageSettings.shared.language)
}

@MainActor func uiMessage(_ english: String) -> String {
    InterfaceCatalog.message(english, language: InterfaceLanguageSettings.shared.language)
}

@MainActor func uiState(_ code: String) -> String {
    uiLabel(code.replacingOccurrences(of: "_", with: " ").capitalized)
}

@MainActor func uiDate(_ date: Date, dateStyle: Date.FormatStyle.DateStyle, timeStyle: Date.FormatStyle.TimeStyle) -> String {
    date.formatted(Date.FormatStyle(date: dateStyle, time: timeStyle)
        .locale(InterfaceLanguageSettings.shared.language.locale))
}
