$ErrorActionPreference = 'Stop'
[Console]::OutputEncoding = New-Object System.Text.UTF8Encoding($false)
try {
    Add-Type -AssemblyName System.Speech
    $reference = [System.Speech.Recognition.SpeechRecognitionEngine].Assembly.Location
    Add-Type -ReferencedAssemblies @($reference, 'System.dll', 'System.Core.dll') -TypeDefinition @'
using System;
using System.Globalization;
using System.Linq;
using System.Speech.Recognition;
using System.Text;
using System.Threading;

public static class HatorielDictation {
    private static readonly object gate = new object();
    private static int accepted = 0;
    private static void Emit(string tag, string text) {
        lock (gate) {
            Console.WriteLine(tag + " " + Convert.ToBase64String(Encoding.UTF8.GetBytes(text ?? "")));
            Console.Out.Flush();
        }
    }
    public static void Run(string language) {
        var installed = SpeechRecognitionEngine.InstalledRecognizers()
            .FirstOrDefault(r => String.Equals(r.Culture.Name, language, StringComparison.OrdinalIgnoreCase));
        if (installed == null) throw new InvalidOperationException("No installed desktop speech recognizer for " + language);
        using (var recognizer = new SpeechRecognitionEngine(installed.Id))
        using (var completed = new ManualResetEvent(false)) {
            recognizer.LoadGrammar(new DictationGrammar());
            recognizer.SetInputToDefaultAudioDevice();
            recognizer.SpeechRecognized += (sender, e) => {
                if (e.Result == null || String.IsNullOrWhiteSpace(e.Result.Text)) return;
                lock (gate) {
                    if (accepted >= 12000) return;
                    string text = e.Result.Text;
                    int limit = Math.Min(12000 - accepted, text.Length);
                    if (limit > 0 && Char.IsHighSurrogate(text[limit - 1])) limit--;
                    if (limit <= 0) return;
                    accepted += limit;
                    Emit("TEXT", text.Substring(0, limit));
                }
            };
            recognizer.RecognizeCompleted += (sender, e) => {
                if (e.Error != null) Emit("ERROR", e.Error.Message);
                completed.Set();
            };
            // EOF on the private stdin pipe means release/stop. No global keyboard hook.
            var stop = new ManualResetEvent(false);
            var control = new Thread(() => { try { Console.ReadLine(); } catch {} finally { stop.Set(); } });
            control.IsBackground = true;
            control.Start();
            recognizer.RecognizeAsync(RecognizeMode.Multiple);
            Emit("READY", "");
            int signal = WaitHandle.WaitAny(new WaitHandle[] { stop, completed }, 45000);
            if (signal != 1) {
                recognizer.RecognizeAsyncStop();
                if (!completed.WaitOne(2500)) {
                    recognizer.RecognizeAsyncCancel();
                    completed.WaitOne(1000);
                }
            }
            Emit("DONE", "");
        }
    }
}
'@
    [HatorielDictation]::Run($env:HATORIEL_STT_LANG)
} catch {
    $message = $_.Exception.Message
    [Console]::WriteLine("ERROR " + [Convert]::ToBase64String([Text.Encoding]::UTF8.GetBytes($message)))
    exit 1
}
