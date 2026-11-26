import Foundation
import SwiftUI

struct ChatEntry: Codable, Identifiable {
    let id: UUID
    let role: String
    let content: String
    init(role: String, content: String) { id = UUID(); self.role = role; self.content = content }
}

@MainActor final class ChatStore: ObservableObject {
    @Published private(set) var history: [ChatEntry] = []
    @Published private(set) var partial = ""
    @Published private(set) var busy = false
    @Published var status = "Налаштуй HTTPS-сервер для діалогу"
    @Published private(set) var endpoint = ""
    @Published private(set) var model = "qwen2.5:3b"
    private var worker: Task<Void, Never>?
    private var generation: UUID?
    private let defaults = UserDefaults.standard

    init() {
        endpoint = defaults.string(forKey: "chat.endpoint") ?? ""
        model = defaults.string(forKey: "chat.model") ?? "qwen2.5:3b"
        if let data = defaults.data(forKey: "chat.history"), data.count <= 4_194_304,
           let entries = try? JSONDecoder().decode([ChatEntry].self, from: data) {
            history = Array(entries.suffix(100))
        }
    }
    private func persist() throws {
        var entries = Array(history.suffix(100))
        var data = try JSONEncoder().encode(entries)
        while data.count > 4_194_304 && entries.count > 1 {
            entries.removeFirst()
            data = try JSONEncoder().encode(entries)
        }
        guard data.count <= 4_194_304 else { throw TransportFailure.tooLarge }
        defaults.set(data, forKey: "chat.history")
        history = entries
    }
    func configure(endpoint: String, model: String) throws {
        guard !busy else { return }
        _ = try ChatTransport.endpoint(endpoint)
        let name = model.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !name.isEmpty, name.utf8.count <= 256 else { throw CoreFailure.rejected("Вкажи назву моделі (до 256 байтів).") }
        self.endpoint = endpoint.trimmingCharacters(in: .whitespacesAndNewlines)
        self.model = name
        defaults.set(self.endpoint, forKey: "chat.endpoint")
        defaults.set(name, forKey: "chat.model")
        status = "Сервер налаштовано; з'єднання ще не перевірено"
    }
    @discardableResult func send(_ text: String) -> Bool {
        let value = text.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !busy, !value.isEmpty else { return false }
        guard value.unicodeScalars.count <= 12000 else { status = "Максимум 12000 символів"; return false }
        do { _ = try ChatTransport.endpoint(endpoint) }
        catch { status = error.localizedDescription; return false }
        let old = history
        history.append(ChatEntry(role: "user", content: value))
        do { try persist() }
        catch { history = old; status = error.localizedDescription; return false }
        start(replace: nil)
        return true
    }
    func retry() {
        guard !busy, let last = history.last else { return }
        if last.role == "user" { start(replace: nil) }
        else if last.role == "assistant", history.dropLast().last?.role == "user" { start(replace: last.id) }
    }
    func clear() {
        guard !busy else { return }
        history = []
        defaults.removeObject(forKey: "chat.history")
        status = "Історію очищено"
    }
    func cancel() {
        guard busy else { return }
        generation = nil
        worker?.cancel(); worker = nil
        partial = ""; busy = false; status = "Зупинено"
    }
    private func start(replace: UUID?) {
        guard !busy else { return }
        do { _ = try ChatTransport.endpoint(endpoint) }
        catch { status = error.localizedDescription; return }
        let source = replace == nil ? history : Array(history.dropLast())
        var context = source.suffix(32).map { CoreMessage(role: $0.role, content: $0.content) }
        while context.count > 1 && ((try? JSONEncoder().encode(context).count) ?? Int.max) > 240000 { context.removeFirst() }
        let snapshot = context, server = endpoint, selectedModel = model, id = UUID()
        generation = id; busy = true; partial = ""; status = "Hatoriel думає…"
        worker = Task { [weak self] in
            do {
                let prepared = try await Task.detached(priority: .userInitiated) { try NativeCore.prepare(snapshot) }.value
                try Task.checkCancellation()
                let reply = try await ChatTransport.generate(base: server, model: selectedModel, messages: prepared) { [weak self] text in
                    guard let self, self.generation == id else { return }
                    self.partial = text
                }
                guard let self, self.generation == id else { return }
                let old = self.history
                if let replace {
                    guard self.history.last?.id == replace else { throw CoreFailure.rejected("Історія змінилася.") }
                    self.history.removeLast()
                }
                self.history.append(ChatEntry(role: "assistant", content: reply))
                do { try self.persist() } catch { self.history = old; throw error }
                self.finish(id, status: "Готово")
            } catch {
                self?.finish(id, status: Task.isCancelled ? "Зупинено" : "Помилка: " + error.localizedDescription)
            }
        }
    }
    private func finish(_ id: UUID, status: String) {
        guard generation == id else { return }
        generation = nil; worker = nil; busy = false; partial = ""; self.status = status
    }
}
