import SwiftUI
import UniformTypeIdentifiers
import CryptoKit

struct ReceivedFile: FileDocument {
    static var readableContentTypes: [UTType] { [.data] }
    let bytes: Data
    init(bytes: Data) { self.bytes = bytes }
    init(configuration: ReadConfiguration) throws { throw TransportFailure.incomplete }
    func fileWrapper(configuration: WriteConfiguration) throws -> FileWrapper { FileWrapper(regularFileWithContents: bytes) }
}

struct FileTransferView: View {
    @ObservedObject var chat: ChatStore
    @State private var code = ""
    @State private var status = "Оригінал файлу · до 8 МіБ"
    @State private var task: Task<Void,Never>?
    @State private var file: ReceivedFile?
    @State private var filename = "attachment"
    @State private var exporting = false
    var body: some View {
        SecureField("Одноразовий код файлу", text: $code).textInputAutocapitalization(.never).autocorrectionDisabled()
        Button("Отримати файл із ПК") {
            let value = code.trimmingCharacters(in: .whitespacesAndNewlines)
            code = ""; status = "Отримання…"
            task = Task {
                defer { task = nil }
                do {
                    let received = try await chat.receiveFile(code: value) { percent in status = "Отримання \(percent)%" }
                    try Task.checkCancellation()
                    filename = received.0; file = ReceivedFile(bytes: received.1)
                    status = "SHA-256 перевірено"; exporting = true
                } catch { status = error.localizedDescription }
            }
        }.disabled(!chat.hasDesktopKey || chat.busy || chat.checkingConnection || task != nil || file != nil)
        if task != nil { Button("Скасувати отримання", role: .cancel) { task?.cancel() } }
        Text(status).font(.footnote)
            .fileExporter(isPresented: $exporting, document: file, contentType: .data, defaultFilename: filename) { result in
                switch result { case .success: status = "Файл збережено"; case .failure(let error): status = error.localizedDescription }
                file = nil
            }
            .onChange(of: exporting) { value in if !value { file = nil } }
            .onDisappear { task?.cancel(); code = "" }
    }
}
