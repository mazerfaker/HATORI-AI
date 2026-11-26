import Foundation
import Darwin
import os

enum LocalFailure: LocalizedError {
    case rejected(String)
    var errorDescription: String? { if case .rejected(let message) = self { return message }; return nil }
}

enum LocalProfile: String, CaseIterable, Codable, Sendable {
    case eco = "Eco", balanced = "Balanced", performance = "Performance"
    var context: Int32 { self == .eco ? 1024 : self == .balanced ? 2048 : 4096 }
    var threads: Int32 { Int32(max(1, min(self == .eco ? 2 : self == .balanced ? 4 : 6, ProcessInfo.processInfo.activeProcessorCount / 2))) }
}

enum LocalModel {
    static func url() throws -> URL {
        let fm = FileManager.default
        var dir = try fm.url(for: .applicationSupportDirectory, in: .userDomainMask, appropriateFor: nil, create: true).appendingPathComponent("Models", isDirectory: true)
        try fm.createDirectory(at: dir, withIntermediateDirectories: true)
        var values = URLResourceValues(); values.isExcludedFromBackup = true
        try dir.setResourceValues(values)
        return dir.appendingPathComponent("local.gguf")
    }
    static func size() -> Int64 {
        guard let path = try? url(), let values = try? path.resourceValues(forKeys: [.fileSizeKey]), let size = values.fileSize else { return 0 }
        return Int64(size)
    }
    static func estimate(_ size: Int64, profile: LocalProfile) -> Int64 {
        size + size/4 + 512*1024*1024 + Int64(profile.context)*128*1024
    }
    static var budget: Int64 {
        // Conservative estimate; iOS may still terminate a process under memory pressure.
        Int64(ProcessInfo.processInfo.physicalMemory * 40 / 100)
    }
    static func verify(_ size: Int64, profile: LocalProfile) throws {
        guard size > 24, size <= 4*1024*1024*1024 else { throw LocalFailure.rejected("Імпортуй GGUF-модель (до 4 GiB).") }
        guard estimate(size, profile: profile) <= budget else { throw LocalFailure.rejected("Орієнтовна RAM перевищує безпечний бюджет. Обери меншу модель або контекст.") }
        let state = ProcessInfo.processInfo.thermalState
        guard state != .serious && state != .critical else { throw LocalFailure.rejected("Пристрій перегрівається. Спробуй пізніше.") }
    }
    static func importFile(_ source: URL) throws -> Int64 {
        let scoped = source.startAccessingSecurityScopedResource()
        defer { if scoped { source.stopAccessingSecurityScopedResource() } }
        let dest = try url()
        let tmp = dest.deletingLastPathComponent().appendingPathComponent(UUID().uuidString + ".tmp")
        defer { try? FileManager.default.removeItem(at: tmp) }
        let input = try FileHandle(forReadingFrom: source)
        defer { try? input.close() }
        guard try input.read(upToCount: 4) == Data([71,71,85,70]) else { throw LocalFailure.rejected("Некоректний заголовок GGUF.") }
        guard FileManager.default.createFile(atPath: tmp.path, contents: Data([71,71,85,70])) else { throw LocalFailure.rejected("Не вдалося створити файл.") }
        let output = try FileHandle(forWritingTo: tmp)
        defer { try? output.close() }
        try output.seekToEnd()
        var total: Int64 = 4
        while let data = try input.read(upToCount: 128*1024), !data.isEmpty {
            total += Int64(data.count)
            guard total <= 4*1024*1024*1024 else { throw LocalFailure.rejected("Модель перевищує 4 GiB.") }
            try output.write(contentsOf: data)
        }
        guard total > 24 else { throw LocalFailure.rejected("Неповний GGUF.") }
        try output.synchronize()
        try output.close()
        let result = tmp.path.withCString { a in dest.path.withCString { b in Darwin.rename(a,b) } }
        guard result == 0 else { throw LocalFailure.rejected("Не вдалося зберегти модель.") }
        return total
    }
    static func remove() throws {
        let path = try url()
        if FileManager.default.fileExists(atPath: path.path) { try FileManager.default.removeItem(at: path) }
    }
}

private final class LocalJob: @unchecked Sendable {
    private let lock = NSLock()
    private var stopped = false
    private var bytes = Data()
    private var last = 0.0
    let partial: @Sendable (String) -> Void
    init(partial: @escaping @Sendable (String) -> Void) { self.partial = partial }
    func cancel() { lock.lock(); stopped = true; lock.unlock() }
    func receive(_ ptr: UnsafePointer<UInt8>?, _ size: Int) -> Int32 {
        lock.lock()
        if stopped { lock.unlock(); return 0 }
        guard let ptr, size > 0 else { lock.unlock(); return 1 }
        bytes.append(ptr, count: size)
        let now = ProcessInfo.processInfo.systemUptime
        var text: String?
        if now-last >= 0.1 { text = String(data: bytes, encoding: .utf8); last = now }
        lock.unlock()
        if let text { partial(text) }
        return 1
    }
    func result() throws -> String {
        lock.lock(); defer { lock.unlock() }
        if stopped { throw CancellationError() }
        guard let value = String(data: bytes, encoding: .utf8), !value.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty else { throw LocalFailure.rejected("Порожня або некоректна UTF-8 відповідь.") }
        return value
    }
}

enum LocalRuntime {
    static func generate(messages: [CoreMessage], profile: LocalProfile, onPartial: @escaping @MainActor (String)->Void) async throws -> String {
        let job = LocalJob { text in Task { @MainActor in onPartial(text) } }
        return try await withTaskCancellationHandler(operation: {
            try Task.checkCancellation()
            return try await Task.detached(priority: .userInitiated) {
                try LocalModel.verify(LocalModel.size(), profile: profile)
                let path = try LocalModel.url()
                guard !messages.contains(where: { $0.content.utf8.contains(0) }) else { throw LocalFailure.rejected("Недопустимий нульовий символ у контексті.") }
                let roles = messages.map { strdup($0.role) }
                let contents = messages.map { strdup($0.content) }
                defer { roles.forEach { free($0) }; contents.forEach { free($0) } }
                guard roles.allSatisfy({ $0 != nil }), contents.allSatisfy({ $0 != nil }) else { throw LocalFailure.rejected("Недостатньо пам'яті.") }
                let rp: [UnsafePointer<CChar>?] = roles.map { $0.map { UnsafePointer($0) } }
                let cp: [UnsafePointer<CChar>?] = contents.map { $0.map { UnsafePointer($0) } }
                var error = [CChar](repeating: 0, count: 512)
                let result = rp.withUnsafeBufferPointer { r in
                    cp.withUnsafeBufferPointer { c in
                        error.withUnsafeMutableBufferPointer { e in
                            path.path.withCString { p in
                                hatoriel_local_generate(p,r.baseAddress,c.baseAddress,messages.count,profile.context,profile.threads,{ ptr,size,opaque in
                                    guard let opaque else { return 0 }
                                    return Unmanaged<LocalJob>.fromOpaque(opaque).takeUnretainedValue().receive(ptr,size)
                                },Unmanaged.passUnretained(job).toOpaque(),e.baseAddress,e.count)
                            }
                        }
                    }
                }
                if result != 0 { throw LocalFailure.rejected(String(cString: error)) }
                return try job.result()
            }.value
        }, onCancel: { job.cancel() })
    }
}
