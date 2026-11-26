import Foundation

enum TransportFailure: LocalizedError {
    case invalidEndpoint, redirect, http(Int), tooLarge, incomplete, empty, server(String)
    var errorDescription: String? {
        switch self {
        case .invalidEndpoint: return "Вкажи коректну HTTPS-адресу сервера без шляху, параметрів чи пароля."
        case .redirect: return "Перенаправлення заборонено."
        case .http(let code): return "HTTP \(code)"
        case .tooLarge: return "Перевищено ліміт розміру повідомлення."
        case .incomplete: return "Потік відповіді перервано."
        case .empty: return "Порожня відповідь."
        case .server(let text): return text
        }
    }
}

private final class NoRedirect: NSObject, URLSessionTaskDelegate {
    func urlSession(_ session: URLSession, task: URLSessionTask, willPerformHTTPRedirection response: HTTPURLResponse, newRequest request: URLRequest, completionHandler: @escaping (URLRequest?) -> Void) {
        completionHandler(nil)
    }
}

enum ChatTransport {
    static func endpoint(_ base: String) throws -> URL {
        guard var c = URLComponents(string: base.trimmingCharacters(in: .whitespacesAndNewlines)),
              c.scheme?.lowercased() == "https", let host = c.host, !host.isEmpty,
              c.user == nil, c.password == nil, c.query == nil, c.fragment == nil,
              c.path.isEmpty || c.path == "/",
              c.port.map({ (1...65535).contains($0) }) ?? true else { throw TransportFailure.invalidEndpoint }
        c.path = "/api/chat"
        guard let url = c.url else { throw TransportFailure.invalidEndpoint }
        return url
    }

    static func validateKey(_ key: String) throws {
        guard key.utf8.count == 64, key.utf8.allSatisfy({ (48...57).contains($0) || (97...102).contains($0) }) else {
            throw CoreFailure.rejected("Потрібен ключ сеансу: 64 символи 0–9 або a–f.")
        }
    }
    struct BridgeStatus: Decodable {
        let service: String
        let `protocol`: Int
        let model: String
        let backend: String
    }
    static func probe(base: String, key: String) async throws -> BridgeStatus {
        try validateKey(key)
        let chatURL = try endpoint(base)
        guard var c = URLComponents(url: chatURL, resolvingAgainstBaseURL: false) else { throw TransportFailure.invalidEndpoint }
        c.path = "/api/status"
        guard let url = c.url else { throw TransportFailure.invalidEndpoint }
        var request = URLRequest(url: url)
        request.httpMethod = "GET"
        request.setValue("Bearer " + key, forHTTPHeaderField: "Authorization")
        request.cachePolicy = .reloadIgnoringLocalCacheData
        let config = URLSessionConfiguration.ephemeral
        config.timeoutIntervalForRequest = 10
        config.timeoutIntervalForResource = 15
        config.httpShouldSetCookies = false
        config.urlCache = nil
        config.urlCredentialStorage = nil
        let session = URLSession(configuration: config, delegate: NoRedirect(), delegateQueue: nil)
        defer { session.invalidateAndCancel() }
        let (bytes, response) = try await session.bytes(for: request)
        guard let http = response as? HTTPURLResponse else { throw TransportFailure.incomplete }
        guard http.statusCode == 200 else { throw TransportFailure.http(http.statusCode) }
        var data = Data()
        for try await byte in bytes {
            try Task.checkCancellation()
            guard data.count < 4096 else { throw TransportFailure.tooLarge }
            data.append(byte)
        }
        let info = try JSONDecoder().decode(BridgeStatus.self, from: data)
        guard info.service == "hatoriel-desktop", info.protocol == 1, info.backend == "ollama" else {
            throw CoreFailure.rejected("Несумісний шлюз Hatoriel.")
        }
        return info
    }
    private struct Payload: Encodable {
        let model: String
        let messages: [CoreMessage]
        let stream = true
    }
    private struct Event: Decodable {
        struct Message: Decodable { let content: String? }
        let message: Message?
        let done: Bool?
        let error: String?
    }

    static func generate(base: String, model: String, messages: [CoreMessage], sessionKey: String = "", onPartial: @escaping @MainActor (String) -> Void) async throws -> String {
        let url = try endpoint(base)
        var request = URLRequest(url: url)
        request.httpMethod = "POST"
        request.setValue("application/json", forHTTPHeaderField: "Content-Type")
        if !sessionKey.isEmpty { try validateKey(sessionKey); request.setValue("Bearer " + sessionKey, forHTTPHeaderField: "Authorization") }
        let body = try JSONEncoder().encode(Payload(model: model, messages: messages))
        guard body.count <= 262144 else { throw TransportFailure.tooLarge }
        request.httpBody = body
        let config = URLSessionConfiguration.ephemeral
        config.urlCredentialStorage = nil
        config.timeoutIntervalForRequest = 120
        config.timeoutIntervalForResource = 300
        config.httpShouldSetCookies = false
        config.urlCache = nil
        let session = URLSession(configuration: config, delegate: NoRedirect(), delegateQueue: nil)
        defer { session.invalidateAndCancel() }
        let (bytes, response) = try await session.bytes(for: request)
        guard let http = response as? HTTPURLResponse else { throw TransportFailure.incomplete }
        guard (200...299).contains(http.statusCode) else { throw TransportFailure.http(http.statusCode) }
        var line = Data(), reply = "", replyBytes = 0, total = 0, done = false
        var lastUpdate = ProcessInfo.processInfo.systemUptime
        func parse(_ data: Data) throws -> Bool {
            if data.isEmpty || data.allSatisfy({ $0 == 13 || $0 == 32 || $0 == 9 }) { return false }
            let event = try JSONDecoder().decode(Event.self, from: data)
            if let error = event.error { throw TransportFailure.server(String(error.prefix(500))) }
            if let part = event.message?.content {
                replyBytes += part.utf8.count
                guard replyBytes <= 262144 else { throw TransportFailure.tooLarge }
                reply += part
            }
            return event.done == true
        }
        for try await byte in bytes {
            try Task.checkCancellation()
            total += 1
            guard total <= 2097152 else { throw TransportFailure.tooLarge }
            if byte == 10 {
                done = try parse(line)
                line.removeAll(keepingCapacity: true)
                let now = ProcessInfo.processInfo.systemUptime
                if now - lastUpdate >= 0.08 || done {
                    lastUpdate = now
                    await onPartial(reply)
                }
                if done { break }
            } else { line.append(byte) }
        }
        if !done && !line.isEmpty { done = try parse(line) }
        try Task.checkCancellation()
        guard done else { throw TransportFailure.incomplete }
        guard !reply.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty else { throw TransportFailure.empty }
        return reply
    }
}
