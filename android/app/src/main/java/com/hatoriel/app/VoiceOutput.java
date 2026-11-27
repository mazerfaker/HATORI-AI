package com.hatoriel.app;
import android.content.Context;
import android.speech.tts.TextToSpeech;
import android.speech.tts.Voice;
import java.util.Locale;
final class VoiceOutput {
    private TextToSpeech tts;
    private boolean ready, closed;
    VoiceOutput(Context context){
        tts=new TextToSpeech(context.getApplicationContext(), result->{if(!closed)ready=result==TextToSpeech.SUCCESS;});
    }
    String speak(String text, String language){
        if(!ready||closed)return "Синтез мовлення ще недоступний";
        Locale locale=Locale.forLanguageTag(language);
        java.util.Set<Voice> voices=tts.getVoices();
        Voice selected=null;
        if(voices!=null)for(Voice v:voices){
            if(!v.isNetworkConnectionRequired()&&v.getLocale().getLanguage().equals(locale.getLanguage())&&!v.getFeatures().contains(TextToSpeech.Engine.KEY_FEATURE_NOT_INSTALLED)){selected=v;break;}
        }
        if(selected==null)return "Встанови локальний голос потрібної мови в Android";
        if(tts.setVoice(selected)!=TextToSpeech.SUCCESS)return "Не вдалося вибрати голос";
        int limit=Math.min(4000,TextToSpeech.getMaxSpeechInputLength());
        int end=Math.min(text.length(),limit);if(end>0&&Character.isHighSurrogate(text.charAt(end-1)))end--;
        if(end==0)return "Немає тексту";
        return tts.speak(text.substring(0,end),TextToSpeech.QUEUE_FLUSH,null,"hatoriel-"+System.nanoTime())==TextToSpeech.SUCCESS?"Озвучування запущено":"Помилка озвучування";
    }
    void stop(){if(tts!=null)tts.stop();}
    void close(){closed=true;ready=false;if(tts!=null){tts.stop();tts.shutdown();tts=null;}}
}
