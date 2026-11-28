import Foundation

struct CoreMessage: Codable, Sendable {
    let role: String
    let content: String
}

enum CoreFailure: LocalizedError {
    case unavailable, rejected(String)
    var errorDescription: String? {
        switch self {
        case .unavailable: return "Rust Core недоступне"
        case .rejected(let reason): return reason
        }
    }
}

struct CoreMood: Codable, Sendable, Equatable {
    var mood = "Calm"
    var intensity: UInt8 = 0
    var updated: UInt64 = 0
}
struct CoreOptions: Codable, Sendable {
    var mode = "Companion"
    var traits: [UInt8] = [90,80,90,90,90,80,70]
    var dynamic_style = true
    var emotion_enabled = false
    var mood = CoreMood()
    static let modes = ["Companion","Assistant","Developer","Researcher"]
    var validated: CoreOptions {
        var value = self
        if !Self.modes.contains(value.mode) { value.mode = "Companion" }
        if value.traits.count != 7 { value.traits = CoreOptions().traits }
        value.traits = value.traits.map { min($0,100) }
        if !["Calm","Joy","Concern","Surprise","Curiosity"].contains(value.mood.mood) { value.mood = CoreMood() }
        value.mood.intensity = min(value.mood.intensity,100)
        if !value.emotion_enabled { value.mood = CoreMood() }
        return value
    }
}
struct CoreProfile: Codable, Sendable {
    var interests = ""
    var communication = ""
    var goals = ""
}
struct CoreMemory: Codable, Sendable {
    let id: Int64
    let content: String
    let importance: UInt8
}
struct CoreContext: Codable, Sendable {
    let history: [CoreMessage]
    var options: CoreOptions
    var memories: [CoreMemory] = []
    var profile = CoreProfile()
}
struct CoreOutput: Codable, Sendable {
    let messages: [CoreMessage]
    let mood: CoreMood
    let effective_traits: [UInt8]
}
enum NativeCore {
    private static func consume(_ ptr: UnsafeMutablePointer<CChar>?) throws -> String {
        guard let ptr else { throw CoreFailure.unavailable }
        defer { hatoriel_free(ptr) }
        guard let value = String(validatingUTF8: ptr) else { throw CoreFailure.unavailable }
        return value
    }
    static func version() throws -> String { try consume(hatoriel_version()) }
    static func prepare(_ history: [CoreMessage]) throws -> [CoreMessage] {
        let input = try JSONEncoder().encode(history)
        let output = try input.withUnsafeBytes { buffer in
            try consume(hatoriel_prepare(buffer.bindMemory(to: UInt8.self).baseAddress, buffer.count))
        }
        let data = Data(output.utf8)
        if let failure = try? JSONDecoder().decode([String: String].self, from: data),
           let error = failure["error"] { throw CoreFailure.rejected(error) }
        return try JSONDecoder().decode([CoreMessage].self, from: data)
    }
    static func prepareContext(_ context: CoreContext) throws -> CoreOutput {
        let input = try JSONEncoder().encode(context)
        let output = try input.withUnsafeBytes { buffer in
            try consume(hatoriel_prepare_context(buffer.bindMemory(to: UInt8.self).baseAddress, buffer.count))
        }
        let data = Data(output.utf8)
        if let failure = try? JSONDecoder().decode([String: String].self, from: data),
           let error = failure["error"] { throw CoreFailure.rejected(error) }
        return try JSONDecoder().decode(CoreOutput.self, from: data)
    }
    static func suggestMemory(_ text: String) throws -> MemorySuggestion? {
        let input = Data(text.utf8)
        let output = try input.withUnsafeBytes { buffer in
            try consume(hatoriel_suggest_memory(buffer.bindMemory(to: UInt8.self).baseAddress, buffer.count))
        }
        return try JSONDecoder().decode(MemorySuggestion?.self, from: Data(output.utf8))
    }
    static func sync<T: Encodable, R: Decodable>(_ command: T, as type: R.Type) throws -> R {
        let input = try JSONEncoder().encode(command)
        let output = try input.withUnsafeBytes { buffer in
            try consume(hatoriel_sync(buffer.bindMemory(to: UInt8.self).baseAddress, buffer.count))
        }
        let data = Data(output.utf8)
        if let failure = try? JSONDecoder().decode([String: String].self, from: data), let error = failure["error"] { throw CoreFailure.rejected(error) }
        return try JSONDecoder().decode(type, from: data)
    }
}
