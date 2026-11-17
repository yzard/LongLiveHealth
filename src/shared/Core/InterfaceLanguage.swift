import Foundation

public nonisolated enum InterfaceLanguage: String, CaseIterable, Sendable {
    case english = "en"
    case chinese = "zh-Hans"

    private static let preferenceKey = "interfaceLanguage"

    public static func systemDefault(_ language: String) -> Self {
        let base = language.split(whereSeparator: { $0 == "-" || $0 == "_" }).first
        return base?.lowercased() == "zh" ? .chinese : .english
    }

    public static var stored: Self? {
        UserDefaults.standard.string(forKey: preferenceKey).flatMap(Self.init(rawValue:))
    }

    public static var current: Self {
        stored ?? systemDefault(Locale.preferredLanguages.first ?? "en")
    }

    public func persist() {
        UserDefaults.standard.set(rawValue, forKey: Self.preferenceKey)
    }

    public var locale: Locale { Locale(identifier: self == .chinese ? "zh_CN" : "en_US") }
    public var displayName: String { self == .chinese ? "中文" : "English" }
}

/// Values that come from a report or the user's archive occupy slots in the translated
/// phrase and are inserted verbatim after translation.
public nonisolated struct InterfaceText: ExpressibleByStringLiteral, ExpressibleByStringInterpolation, Sendable {
    let key: String
    let values: [String]

    public init(stringLiteral value: String) {
        key = value
        values = []
    }

    public init(stringInterpolation: StringInterpolation) {
        key = stringInterpolation.key
        values = stringInterpolation.values
    }

    public struct StringInterpolation: StringInterpolationProtocol, Sendable {
        var key: String
        var values: [String]

        public init(literalCapacity: Int, interpolationCount: Int) {
            key = ""
            key.reserveCapacity(literalCapacity)
            values = []
            values.reserveCapacity(interpolationCount)
        }

        public mutating func appendLiteral(_ literal: String) { key.append(literal) }

        public mutating func appendInterpolation<T>(_ value: T) {
            key.append("{\(values.count)}")
            values.append(String(describing: value))
        }
    }
}

public nonisolated enum InterfaceCatalog {
    private static let placeholders = try? NSRegularExpression(pattern: #"\{([0-9]+)\}"#)
    private static let chinese: [String: String] = {
        #if SWIFT_PACKAGE
        let bundle = Bundle.module
        #else
        let bundle = Bundle.main
        #endif
        guard let url = bundle.url(forResource: "InterfaceChinese", withExtension: "json"),
              let data = try? Data(contentsOf: url),
              let catalog = try? JSONDecoder().decode([String: String].self, from: data)
        else { return [:] }
        return catalog
    }()

    public static func label(_ key: String, language: InterfaceLanguage) -> String {
        language == .chinese ? chinese[key] ?? key : key
    }

    public static func text(_ phrase: InterfaceText, language: InterfaceLanguage) -> String {
        render(label(phrase.key, language: language), values: phrase.values)
    }

    /// Localize application-authored diagnostics already formatted by a Health bridge
    /// or by an older server. Only call this for interface copy, never archived values.
    public static func message(_ english: String, language: InterfaceLanguage) -> String {
        guard language == .chinese else { return english }
        if let direct = chinese[english] { return direct }
        let range = NSRange(english.startIndex..., in: english)
        for entry in messageTemplates {
            guard english.hasPrefix(entry.prefix),
                  let match = entry.pattern.firstMatch(in: english, range: range) else { continue }
            let values = (1..<match.numberOfRanges).map { index in
                Range(match.range(at: index), in: english).map { String(english[$0]) } ?? ""
            }
            return render(entry.translation, values: values)
        }
        return english
    }

    private struct MessageTemplate: Sendable {
        let prefix: String
        let pattern: NSRegularExpression
        let translation: String
    }

    private static let messageTemplates: [MessageTemplate] = {
        guard let placeholders else { return [] }
        return chinese.compactMap { key, translation -> MessageTemplate? in
            let matches = placeholders.matches(in: key, range: NSRange(key.startIndex..., in: key))
            guard let first = matches.first.flatMap({ Range($0.range, in: key) }),
                  key[..<first.lowerBound].count >= 5 else { return nil }
            let prefix = String(key[..<first.lowerBound])
            var pattern = "\\A"
            var start = key.startIndex
            for match in matches {
                guard let range = Range(match.range, in: key) else { return nil }
                pattern += NSRegularExpression.escapedPattern(for: String(key[start..<range.lowerBound])) + "(.*?)"
                start = range.upperBound
            }
            pattern += NSRegularExpression.escapedPattern(for: String(key[start...])) + "\\z"
            guard let regex = try? NSRegularExpression(pattern: pattern, options: [.dotMatchesLineSeparators]) else { return nil }
            return MessageTemplate(prefix: prefix, pattern: regex, translation: translation)
        }.sorted { $0.prefix.count > $1.prefix.count }
    }()

    static func render(_ template: String, values: [String]) -> String {
        guard !values.isEmpty, let placeholders else { return template }
        var result = ""
        var start = template.startIndex
        for match in placeholders.matches(in: template, range: NSRange(template.startIndex..., in: template)) {
            guard let range = Range(match.range, in: template),
                  let number = Range(match.range(at: 1), in: template),
                  let index = Int(template[number]), values.indices.contains(index) else { continue }
            result.append(contentsOf: template[start..<range.lowerBound])
            result.append(values[index])
            start = range.upperBound
        }
        result.append(contentsOf: template[start...])
        return result
    }
}
