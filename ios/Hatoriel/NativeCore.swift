import Foundation

struct CoreMessage: Codable {
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
}
