import Foundation
import Testing
@testable import LongLiveHealthCore

@Test func firstLaunchLanguageMatchesSupportedSystemLanguage() {
    #expect(InterfaceLanguage.systemDefault("zh-Hans-CN") == .chinese)
    #expect(InterfaceLanguage.systemDefault("zh_TW") == .chinese)
    #expect(InterfaceLanguage.systemDefault("en-CN") == .english)
    #expect(InterfaceLanguage.systemDefault("ja-JP") == .english)
}

@Test func reportInterfaceUsesOneLanguageAndPreservesInsertedValues() {
    #expect(InterfaceCatalog.text("Review result", language: .english) == "Review result")
    #expect(InterfaceCatalog.text("Review result", language: .chinese) == "审核结果")
    #expect(InterfaceCatalog.text("Cancel", language: .english) == "Cancel")
    #expect(InterfaceCatalog.text("Save", language: .chinese) == "保存")
    #expect(InterfaceCatalog.text("Submit", language: .chinese) == "提交")
    let raw = "Example Laboratory {1}"
    #expect(InterfaceCatalog.render("{1} | {0}", values: [raw, "2026-11-17"]) == "2026-11-17 | Example Laboratory {1}")
    #expect(InterfaceCatalog.message("Page 2 of 5", language: .chinese) == InterfaceCatalog.text("Page \(2) of \(5)", language: .chinese))
}
