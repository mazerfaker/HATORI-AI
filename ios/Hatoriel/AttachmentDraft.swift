import SwiftUI
import UIKit
import ImageIO
import UniformTypeIdentifiers

struct DraftAttachment: Identifiable {
    let id = UUID()
    let name: String
    let data: Data
    let thumbnail: UIImage?
}
@MainActor final class AttachmentDraft: ObservableObject {
    @Published private(set) var items: [DraftAttachment] = []
    @Published private(set) var loading = false
    @Published var status = ""
    private var ticket: UUID?
    private static let maxFile = 8 * 1024 * 1024
    private static let maxTotal = 24 * 1024 * 1024
    var empty: Bool { items.isEmpty && !loading }

    func add(_ url: URL) {
        guard !loading, items.count < 4 else { status = "Максимум 4 вкладення"; return }
        let id = UUID()
        ticket = id; loading = true; status = "Читання файлу…"
        let remaining = min(Self.maxFile, Self.maxTotal - items.reduce(0) { $0 + $1.data.count })
        Task {
            do {
                let item = try await Task.detached(priority: .userInitiated) {
                    try Self.read(url, limit: remaining)
                }.value
                guard ticket == id else { return }
                if items.contains(where: { $0.data == item.data }) { status = "Файл вже додано" }
                else { items.append(item); status = "Локальна чернетка" }
            } catch {
                guard ticket == id else { return }; status = error.localizedDescription
            }
            if ticket == id { ticket = nil; loading = false }
        }
    }
    nonisolated private static func read(_ url: URL, limit: Int) throws -> DraftAttachment {
        let started = url.startAccessingSecurityScopedResource()
        defer { if started { url.stopAccessingSecurityScopedResource() } }
        let values = try url.resourceValues(forKeys: [.isRegularFileKey, .fileSizeKey])
        guard values.isRegularFile == true, limit > 0, (values.fileSize ?? 0) <= limit else { throw Failure.limit }
        let file = try FileHandle(forReadingFrom: url)
        defer { try? file.close() }
        var data = Data()
        while data.count <= limit {
            let chunk = try file.read(upToCount: min(65536, limit + 1 - data.count)) ?? Data()
            if chunk.isEmpty { break }
            data.append(chunk)
        }
        guard !data.isEmpty, data.count <= limit else { throw Failure.limit }
        let name = String(url.lastPathComponent.unicodeScalars.filter { !CharacterSet.controlCharacters.contains($0) }.prefix(160))
        var thumbnail: UIImage?
        if let source = CGImageSourceCreateWithData(data as CFData, [kCGImageSourceShouldCache:false] as CFDictionary),
           let type = CGImageSourceGetType(source) as String?, let ut = UTType(type), ut.conforms(to: .png) || ut.conforms(to: .jpeg),
           let info = CGImageSourceCopyPropertiesAtIndex(source,0,nil) as? [CFString:Any],
           let w = info[kCGImagePropertyPixelWidth] as? Int, let h = info[kCGImagePropertyPixelHeight] as? Int,
           w > 0, h > 0, w <= 4096, h <= 4096,
           let cg = CGImageSourceCreateThumbnailAtIndex(source,0,[kCGImageSourceCreateThumbnailFromImageAlways:true,kCGImageSourceThumbnailMaxPixelSize:160,kCGImageSourceCreateThumbnailWithTransform:true,kCGImageSourceShouldCacheImmediately:true] as CFDictionary) {
            thumbnail = UIImage(cgImage: cg)
        }
        return DraftAttachment(name: name.isEmpty ? "attachment" : name,data: data,thumbnail: thumbnail)
    }
    func remove(_ id: UUID) { items.removeAll { $0.id == id } }
    func clear() { ticket = nil; loading = false; items.removeAll(); status = "" }
    enum Failure: LocalizedError {
        case limit
        var errorDescription: String? { "Файл має бути 1 байт–8 МіБ, загалом до 24 МіБ." }
    }
}
struct AttachmentPanel: View {
    @ObservedObject var draft: AttachmentDraft
    var enabled: Bool
    @State private var choose = false
    var body: some View {
        VStack(alignment: .leading, spacing: 5) {
            HStack {
                Button { choose = true } label: { Label("Файл", systemImage: "paperclip") }.disabled(!enabled || draft.loading || draft.items.count >= 4)
                Text("\(draft.items.count)/4").font(.caption)
                Spacer()
                if draft.loading { ProgressView() }
                Button("Очистити") { draft.clear() }.disabled(!enabled || draft.empty)
            }
            if !draft.empty {
                Text("Аналіз ще недоступний. Видали вкладення, щоб надіслати текст.").font(.caption2)
                ScrollView(.horizontal) {
                    HStack {
                        ForEach(draft.items) { item in
                            VStack(alignment: .leading, spacing: 3) {
                                if let image = item.thumbnail { Image(uiImage: image).resizable().scaledToFit().frame(width: 96,height: 60) }
                                else { Image(systemName: "doc").frame(width: 96,height: 60) }
                                Text(item.name).lineLimit(1).frame(maxWidth: 105)
                                Text("\(item.data.count / 1024) KiB").foregroundStyle(.secondary)
                                Button("Видалити") { draft.remove(item.id) }.disabled(!enabled)
                            }.font(.caption2).padding(6).background(.white.opacity(0.06), in: RoundedRectangle(cornerRadius: 8))
                        }
                    }
                }
            }
            if !draft.status.isEmpty { Text(draft.status).font(.caption2).foregroundStyle(.secondary) }
        }
        .fileImporter(isPresented: $choose, allowedContentTypes: [.item]) { result in
            guard enabled else { return }
            switch result { case .success(let url): draft.add(url); case .failure(let error): draft.status = error.localizedDescription }
        }
    }
}
