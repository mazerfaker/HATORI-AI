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
    @Published private(set) var feedbackState: CrimsonState = .idle
    @Published private(set) var feedbackRevision = 0
    @Published var status = "Налаштуй HTTPS-сервер для діалогу"
    @Published private(set) var endpoint = ""
    @Published private(set) var model = "qwen2.5:3b"
    @Published private(set) var options = CoreOptions()
    @Published private(set) var effectiveTraits: [UInt8] = CoreOptions().traits
    @Published private(set) var localMode = false
    @Published private(set) var localProfile: LocalProfile = .balanced
    @Published private(set) var localSize: Int64 = 0
    @Published private(set) var importing = false
    @Published private(set) var memory = MemoryVault()
    @Published private(set) var memoryError: String? = nil
    @Published private(set) var suggestion: MemorySuggestion? = nil
    @Published private(set) var desktopMode = false
    @Published private(set) var connectionStatus = "Не перевірено"
    @Published private(set) var checkingConnection = false
    private var sessionKey = ""
    private var worker: Task<Void, Never>?
    private var generation: UUID?
    private let defaults = UserDefaults.standard

    init() {
        if let data = defaults.data(forKey: "core.options"), data.count < 16384,
           let stored = try? JSONDecoder().decode(CoreOptions.self, from: data) {
            options = stored.validated
            effectiveTraits = options.traits
        }
        localMode = defaults.bool(forKey: "local.enabled")
        localProfile = LocalProfile(rawValue: defaults.string(forKey: "local.profile") ?? "") ?? .balanced
        localSize = LocalModel.size()
        reloadMemory()
        endpoint = defaults.string(forKey: "chat.endpoint") ?? ""
        model = defaults.string(forKey: "chat.model") ?? "qwen2.5:3b"
        desktopMode = defaults.bool(forKey: "chat.desktopMode")
        if desktopMode { connectionStatus = "Введи новий ключ сеансу ПК" }
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
    func configureLocal(enabled: Bool, profile: LocalProfile) {
        guard !busy && !importing else { return }
        localMode = enabled; localProfile = profile
        defaults.set(enabled, forKey: "local.enabled")
        defaults.set(profile.rawValue, forKey: "local.profile")
        status = enabled ? "Локальний режим · CPU Preview" : "Мережевий режим"
    }
    func importModel(_ url: URL) {
        guard !busy && !importing else { return }
        importing = true; status = "Імпорт GGUF…"
        Task { [weak self] in
            do {
                let size = try await Task.detached(priority: .utility) { try LocalModel.importFile(url) }.value
                self?.localSize = size; self?.status = "GGUF імпортовано"
            } catch { self?.status = "Імпорт: " + error.localizedDescription }
            self?.importing = false
        }
    }
    func removeModel() {
        guard !busy && !importing else { return }
        do { try LocalModel.remove(); localSize = 0; status = "Локальну копію видалено" }
        catch { status = error.localizedDescription }
    }
    private func validateBackend() throws {
        if localMode { try LocalModel.verify(localSize, profile: localProfile) }
        else { _ = try ChatTransport.endpoint(endpoint); if desktopMode { try ChatTransport.validateKey(sessionKey) } }
    }
    func reloadMemory() {
        guard !busy else { return }
        do {
            var loaded = try MemoryVault.load()
            if let stored = loaded.personality {
                options = stored.validated
            } else {
                // Migrate legacy UserDefaults settings only after a successful atomic write.
                loaded.personality = options.validated
                try loaded.persist()
            }
            memory = loaded; effectiveTraits = options.traits; memoryError = nil
            defaults.removeObject(forKey: "core.options")
        }
        catch { memoryError = "Пам’ять недоступна: " + error.localizedDescription }
    }
    private func updateMemory(_ change: (inout MemoryVault) throws -> Void) throws {
        guard !busy else { throw CoreFailure.rejected("Дочекайся завершення генерації.") }
        guard memoryError == nil else { throw CoreFailure.rejected(memoryError ?? "Пам’ять недоступна.") }
        var proposed = memory
        try change(&proposed)
        try proposed.persist()
        memory = proposed
    }
    func saveMemory(id: Int64?, content: String, category: String, importance: UInt8) throws {
        try updateMemory { try $0.save(id: id, content: content, category: category, importance: importance) }
    }
    func removeMemory(_ id: Int64) throws { try updateMemory { $0.records.removeAll { $0.id == id } } }
    func saveProfile(_ profile: CoreProfile) throws { try updateMemory { try $0.setProfile(profile) } }
    func setMemoryOptions(local: Bool, suggestions: Bool, remote: Bool) throws {
        if remote { _ = try ChatTransport.endpoint(endpoint) }
        try updateMemory { $0.localEnabled = local; $0.suggestionsEnabled = suggestions; $0.approvedEndpoint = remote ? endpoint : nil }
        if !suggestions { suggestion = nil }
    }
    func dismissSuggestion() { suggestion = nil }
    func approveSuggestion() throws {
        guard let item = suggestion else { return }
        try saveMemory(id: nil, content: item.content, category: item.category, importance: item.importance)
        suggestion = nil
    }
    private func proposeMemory(_ text: String) {
        guard memoryError == nil, memory.suggestionsEnabled, suggestion == nil,
              let item = try? NativeCore.suggestMemory(text),
              !memory.records.contains(where: { MemoryVault.normalized($0.content) == MemoryVault.normalized(item.content) }) else { return }
        suggestion = item
    }
    func syncSnapshot() throws -> SyncSnapshot {
        guard !busy, memoryError == nil else { throw CoreFailure.rejected("Пам’ять недоступна або генерація активна.") }
        var safe = options; safe.mood = CoreMood()
        return SyncSnapshot(version: 1, records: memory.records.map { SyncRecord(content: $0.content, category: $0.category, importance: $0.importance) }, profile: memory.profile, options: safe)
    }
    func applySync(_ plan: SyncPlan, profile: Bool, personality: Bool) throws {
        try updateMemory { vault in
            for row in plan.additions { try vault.save(id: nil, content: row.content, category: row.category, importance: row.importance) }
            if profile { try vault.setProfile(plan.profile) }
            if personality { vault.personality = plan.options.validated }
            vault.approvedEndpoint = nil
        }
        if personality, let stored = memory.personality {
            options = stored; effectiveTraits = stored.traits
        }
    }
    func configurePersonality(_ proposed: CoreOptions) {
        do {
            let validated = proposed.validated
            try updateMemory { $0.personality = validated }
            options = validated; effectiveTraits = options.traits
            status = "Особистість збережено"
        } catch { status = error.localizedDescription }
    }
    private func persistMood(_ mood: CoreMood, effective: [UInt8]) {
        // The active generation cannot use updateMemory, which intentionally rejects busy edits.
        guard memoryError == nil else { return }
        var proposed = memory
        var next = options; next.mood = mood; next = next.validated
        proposed.personality = next
        do {
            try proposed.persist()
            memory = proposed; options = next; effectiveTraits = effective
        } catch { memoryError = "Не вдалося зберегти стан: " + error.localizedDescription }
    }
    func configure(endpoint: String, model: String, desktop: Bool, key: String) throws {
        guard !busy && !checkingConnection else { throw CoreFailure.rejected("Зачекай завершення поточної операції.") }
        _ = try ChatTransport.endpoint(endpoint)
        let name = model.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !name.isEmpty, name.utf8.count <= 256 else { throw CoreFailure.rejected("Вкажи назву моделі (до 256 байтів).") }
        let newEndpoint = endpoint.trimmingCharacters(in: .whitespacesAndNewlines)
        let candidate = key.trimmingCharacters(in: .whitespacesAndNewlines)
        if desktop { try ChatTransport.validateKey(candidate) }
        if newEndpoint != self.endpoint || desktop != desktopMode {
            guard memoryError == nil else { throw CoreFailure.rejected("Спочатку віднови доступ до сховища пам’яті, щоб безпечно змінити сервер.") }
            if memory.approvedEndpoint != nil { try updateMemory { $0.approvedEndpoint = nil } }
        }
        desktopMode = desktop
        sessionKey = desktop ? candidate : ""
        defaults.set(desktop, forKey: "chat.desktopMode")
        connectionStatus = "Не перевірено"
        self.endpoint = newEndpoint
        self.model = name
        defaults.set(self.endpoint, forKey: "chat.endpoint")
        defaults.set(name, forKey: "chat.model")
        status = "Сервер налаштовано; з'єднання ще не перевірено"
    }
    func checkDesktop() async {
        guard desktopMode && !busy && !checkingConnection else { return }
        checkingConnection = true
        defer { checkingConnection = false }
        connectionStatus = "Перевірка шлюзу…"
        do {
            let info = try await ChatTransport.probe(base: endpoint, key: sessionKey)
            if info.model == model { connectionStatus = "Шлюз авторизовано · " + info.model }
            else { connectionStatus = "Модель ПК відрізняється: " + info.model }
        } catch {
            connectionStatus = "Перевірка: " + error.localizedDescription
        }
    }
    func receiveFile(code: String, progress: @escaping @MainActor (Int) -> Void) async throws -> (String, Data) {
        guard desktopMode && !busy && !checkingConnection else { throw CoreFailure.rejected("Підключи шлюз ПК") }
        checkingConnection = true
        defer { checkingConnection = false }
        let server = endpoint, key = sessionKey
        let result = try await ChatTransport.receiveFile(base: server, key: key, code: code, progress: progress)
        try Task.checkCancellation()
        guard endpoint == server, sessionKey == key, desktopMode else { throw CoreFailure.rejected("Підключення змінилося") }
        return result
    }
    func forgetDesktopKey() {
        guard !busy && !checkingConnection else { return }
        sessionKey = ""
        connectionStatus = "Ключ видалено з пам’яті"
    }
    var hasDesktopKey: Bool { !sessionKey.isEmpty }
    @discardableResult func send(_ text: String) -> Bool {
        let value = text.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !busy, !importing, !checkingConnection, !value.isEmpty else { return false }
        guard value.unicodeScalars.count <= 12000 else { status = "Максимум 12000 символів"; return false }
        do { try validateBackend() }
        catch { status = error.localizedDescription; return false }
        let old = history
        history.append(ChatEntry(role: "user", content: value))
        do { try persist() }
        catch { history = old; status = error.localizedDescription; return false }
        start(replace: nil)
        proposeMemory(value)
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
        persistMood(CoreMood(), effective: options.traits)
        status = "Історію очищено"
    }
    func cancel() {
        guard busy else { return }
        generation = nil
        worker?.cancel(); worker = nil
        partial = ""; busy = false; status = "Зупинено"; feedbackState = .cancelled; feedbackRevision += 1
    }
    private func start(replace: UUID?) {
        guard !busy && !importing && !checkingConnection else { return }
        do { try validateBackend() }
        catch { status = error.localizedDescription; return }
        let source = replace == nil ? history : Array(history.dropLast())
        var context = source.suffix(32).map { CoreMessage(role: $0.role, content: $0.content) }
        while context.count > 1 && ((try? JSONEncoder().encode(context).count) ?? Int.max) > 240000 { context.removeFirst() }
        let useMemory = memoryError == nil && memory.allows(local: localMode, endpoint: endpoint)
        let snapshot = CoreContext(history: context, options: options, memories: useMemory ? memory.records.map(\.core) : [], profile: useMemory ? memory.profile : CoreProfile()), server = endpoint, selectedModel = model, id = UUID(), useLocal = localMode, profile = localProfile, key = desktopMode ? sessionKey : ""
        generation = id; busy = true; partial = ""; status = "Hatoriel думає…"; feedbackState = .thinking
        worker = Task { [weak self] in
            do {
                let prepared = try await Task.detached(priority: .userInitiated) { try NativeCore.prepareContext(snapshot) }.value
                try Task.checkCancellation()
                guard self?.generation == id else { return }
                self?.persistMood(prepared.mood, effective: prepared.effective_traits)
                let update: @MainActor (String) -> Void = { [weak self] text in
                    guard let self, self.generation == id else { return }
                    self.partial = text
                    if !text.isEmpty { self.feedbackState = .responding }
                }
                let reply: String
                if useLocal {
                    reply = try await LocalRuntime.generate(messages: prepared.messages, profile: profile, onPartial: update)
                } else {
                    reply = try await ChatTransport.generate(base: server, model: selectedModel, messages: prepared.messages, sessionKey: key, onPartial: update)
                }
                guard let self, self.generation == id else { return }
                let old = self.history
                if let replace {
                    guard self.history.last?.id == replace else { throw CoreFailure.rejected("Історія змінилася.") }
                    self.history.removeLast()
                }
                self.history.append(ChatEntry(role: "assistant", content: reply))
                do { try self.persist() } catch { self.history = old; throw error }
                self.finish(id, status: "Готово", outcome: .completed)
            } catch {
                self?.finish(id, status: Task.isCancelled ? "Зупинено" : "Помилка: " + error.localizedDescription, outcome: Task.isCancelled ? .cancelled : .failed)
            }
        }
    }
    private func finish(_ id: UUID, status: String, outcome: CrimsonState) {
        guard generation == id else { return }
        generation = nil; worker = nil; busy = false; partial = ""; self.status = status; feedbackState = outcome; feedbackRevision += 1
    }
}
