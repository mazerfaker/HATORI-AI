import SwiftUI
import AVFoundation
import Speech

@MainActor final class VoiceController: NSObject, ObservableObject {
    @Published private(set) var listening = false
    @Published private(set) var requesting = false
    @Published private(set) var speaking = false
    @Published private(set) var transcript = ""
    @Published private(set) var status = ""
    private let engine = AVAudioEngine()
    private let synthesizer = AVSpeechSynthesizer()
    private var recognition: SFSpeechRecognitionTask?
    private var request: SFSpeechAudioBufferRecognitionRequest?
    private var timer: Task<Void, Never>?
    private var ticket: UUID?
    private var tapInstalled = false

    override init() {
        super.init()
    }
    func start(language: String) {
        guard ticket == nil, !speaking else { return }
        let id = UUID()
        ticket = id; requesting = true; transcript = ""; status = "Перевірка дозволів…"
        Task {
            let speech = await withCheckedContinuation { (c: CheckedContinuation<SFSpeechRecognizerAuthorizationStatus,Never>) in
                SFSpeechRecognizer.requestAuthorization { c.resume(returning: $0) }
            }
            guard ticket == id else { return }
            guard speech == .authorized else { stop("Немає дозволу на розпізнавання."); return }
            let mic = await withCheckedContinuation { (c: CheckedContinuation<Bool,Never>) in
                AVAudioSession.sharedInstance().requestRecordPermission { c.resume(returning: $0) }
            }
            guard ticket == id else { return }
            guard mic else { stop("Немає дозволу на мікрофон."); return }
            guard let recognizer = SFSpeechRecognizer(locale: Locale(identifier: language)),
                  recognizer.isAvailable, recognizer.supportsOnDeviceRecognition else {
                stop("Локальне розпізнавання цієї мови недоступне."); return
            }
            do {
                let audio = AVAudioSession.sharedInstance()
                try audio.setCategory(.record, mode: .measurement)
                try audio.setActive(true)
                let req = SFSpeechAudioBufferRecognitionRequest()
                req.requiresOnDeviceRecognition = true
                req.shouldReportPartialResults = true
                request = req
                let node = engine.inputNode, format = node.outputFormat(forBus: 0)
                guard format.sampleRate > 0, format.channelCount > 0 else { stop("Мікрофон недоступний."); return }
                node.installTap(onBus: 0, bufferSize: 1024, format: format) { buffer, _ in req.append(buffer) }
                tapInstalled = true
                engine.prepare(); try engine.start()
                listening = true; status = "Слухаю · до 45 с"
                recognition = recognizer.recognitionTask(with: req) { [weak self] result, error in
                    let text = result?.bestTranscription.formattedString
                    let final = result?.isFinal == true
                    let failed = error != nil
                    Task { @MainActor in
                        guard let self, self.ticket == id else { return }
                        if let text { self.transcript = String(text.unicodeScalars.prefix(12000)) }
                        if failed || final { self.stop(failed ? "Розпізнавання завершено з помилкою." : "Чернетка готова.") }
                    }
                }
                timer = Task { [weak self] in
                    try? await Task.sleep(nanoseconds: 45_000_000_000)
                    guard !Task.isCancelled, let self, self.ticket == id else { return }
                    self.stop("Ліміт запису · чернетка готова.")
                }
            } catch { stop("Не вдалося запустити мікрофон.") }
        }
    }
    func stop(_ message: String = "Чернетка готова.") {
        guard ticket != nil else { return }
        ticket = nil; requesting = false; timer?.cancel(); timer = nil
        engine.stop()
        if tapInstalled { engine.inputNode.removeTap(onBus: 0); tapInstalled = false }
        request?.endAudio(); recognition?.cancel()
        recognition = nil; request = nil; listening = false; status = message
        try? AVAudioSession.sharedInstance().setActive(false, options: .notifyOthersOnDeactivation)
    }
    func speak(_ text: String, language: String) {
        stop()
        stopSpeech()
        guard !text.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty,
              let voice = AVSpeechSynthesisVoice(language: language) else { status = "Голос цієї мови недоступний."; return }
        do {
            try AVAudioSession.sharedInstance().setCategory(.playback, mode: .spokenAudio)
            try AVAudioSession.sharedInstance().setActive(true)
        } catch { status = "Аудіовихід недоступний."; return }
        let utterance = AVSpeechUtterance(string: String(text.prefix(4000)))
        utterance.voice = voice
        synthesizer.speak(utterance); speaking = true; status = "Озвучування"
        monitor()
    }
    private var speechMonitor: Task<Void, Never>?
    private func monitor() {
        speechMonitor?.cancel()
        speechMonitor = Task { [weak self] in
            while !Task.isCancelled {
                try? await Task.sleep(nanoseconds: 250_000_000)
                guard !Task.isCancelled, let self else { return }
                if !self.synthesizer.isSpeaking { self.stopSpeech(); return }
            }
        }
    }
    func stopSpeech() {
        speechMonitor?.cancel(); speechMonitor = nil
        if speaking { synthesizer.stopSpeaking(at: .immediate); speaking = false; status = "Озвучування завершено"; try? AVAudioSession.sharedInstance().setActive(false, options: .notifyOthersOnDeactivation) }
    }
    func clearTranscript() { transcript = "" }
    func shutdown() { stop(); stopSpeech() }
}
