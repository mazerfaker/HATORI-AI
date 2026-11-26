import Foundation

struct MemorySuggestion: Codable, Sendable {
    let content: String
    let category: String
    let importance: UInt8
}

struct MemoryRecord: Codable, Identifiable, Sendable {
    let id: Int64
    var content: String
    var category: String
    var importance: UInt8
    let created: Date
    var core: CoreMemory { CoreMemory(id: id, content: content, importance: importance) }
}

struct MemoryVault: Codable, Sendable {
    static let categories = ["General","Interests","Preferences","Skills","Goals","Projects","Relationships","Other"]
    static let labels = ["General":"Загальне","Interests":"Інтереси","Preferences":"Уподобання","Skills":"Навички","Goals":"Цілі","Projects":"Проєкти","Relationships":"Стосунки","Other":"Інше"]
    var records: [MemoryRecord] = []
    var profile = CoreProfile()
    // Optional for compatibility with vaults created before 0.5.7.1.
    var personality: CoreOptions? = nil
    var localEnabled = true
    var suggestionsEnabled = true
    var approvedEndpoint: String? = nil
    var nextID: Int64 = 1

    static func normalized(_ text: String) -> String {
        text.split(whereSeparator: { $0.isWhitespace }).joined(separator: " ").lowercased()
    }
    mutating func save(id: Int64?, content: String, category: String, importance: UInt8) throws {
        let value = content.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !value.isEmpty, value.unicodeScalars.count <= 500, Self.categories.contains(category), importance <= 100 else {
            throw CoreFailure.rejected("Спогад: 1–500 символів, коректна категорія та важливість 0–100.")
        }
        guard !records.contains(where: { $0.id != id && Self.normalized($0.content) == Self.normalized(value) }) else {
            throw CoreFailure.rejected("Такий спогад уже існує.")
        }
        if let id {
            guard let index = records.firstIndex(where: { $0.id == id }) else { throw CoreFailure.rejected("Спогад не знайдено.") }
            records[index].content = value; records[index].category = category; records[index].importance = importance
        } else {
            guard records.count < 100, nextID > 0, nextID < Int64.max else { throw CoreFailure.rejected("Ліміт 100 спогадів.") }
            records.append(MemoryRecord(id: nextID, content: value, category: category, importance: importance, created: Date()))
            nextID += 1
        }
    }
    mutating func setProfile(_ proposed: CoreProfile) throws {
        guard [proposed.interests, proposed.communication, proposed.goals].allSatisfy({ $0.unicodeScalars.count <= 1000 }) else {
            throw CoreFailure.rejected("До 1000 символів у кожному полі.")
        }
        profile = proposed
    }
    func allows(local: Bool, endpoint: String) -> Bool {
        local ? localEnabled : (!endpoint.isEmpty && approvedEndpoint == endpoint)
    }
    private static func location() throws -> URL {
        let fm = FileManager.default
        var dir = try fm.url(for: .applicationSupportDirectory, in: .userDomainMask, appropriateFor: nil, create: true).appendingPathComponent("MemoryVault", isDirectory: true)
        try fm.createDirectory(at: dir, withIntermediateDirectories: true)
        var values = URLResourceValues(); values.isExcludedFromBackup = true
        try dir.setResourceValues(values)
        return dir.appendingPathComponent("memory.json")
    }
    static func load() throws -> MemoryVault {
        let url = try location()
        guard FileManager.default.fileExists(atPath: url.path) else { return MemoryVault() }
        let data = try Data(contentsOf: url)
        guard data.count <= 1024*1024 else { throw CoreFailure.rejected("Сховище пам’яті перевищує ліміт.") }
        let vault = try JSONDecoder().decode(MemoryVault.self, from: data)
        guard vault.records.count <= 100, Set(vault.records.map(\.id)).count == vault.records.count,
              vault.records.allSatisfy({ $0.id > 0 && $0.id < vault.nextID && !$0.content.isEmpty && $0.content.unicodeScalars.count <= 500 && Self.categories.contains($0.category) && $0.importance <= 100 }),
              vault.nextID > 0, [vault.profile.interests,vault.profile.communication,vault.profile.goals].allSatisfy({ $0.unicodeScalars.count <= 1000 }) else {
            throw CoreFailure.rejected("Некоректний формат пам’яті. Файл не змінено.")
        }
        return vault
    }
    func persist() throws {
        let data = try JSONEncoder().encode(self)
        guard data.count <= 1024*1024 else { throw CoreFailure.rejected("Сховище перевищує ліміт.") }
        try data.write(to: Self.location(), options: [.atomic, .completeFileProtection])
    }
}
