import Foundation

func dateScope(days: Int) -> JSONValue {
    let calendar = Calendar.current
    let end = Date(); let start = calendar.date(byAdding: .day, value: -days, to: end) ?? end
    let format = DateFormatter(); format.calendar = Calendar(identifier: .gregorian); format.locale = Locale(identifier: "en_US_POSIX"); format.timeZone = .current; format.dateFormat = "yyyy-MM-dd"
    return .object(["start_date": .string(format.string(from: start)), "end_date": .string(format.string(from: end)), "timezone": .string(TimeZone.current.identifier)])
}
