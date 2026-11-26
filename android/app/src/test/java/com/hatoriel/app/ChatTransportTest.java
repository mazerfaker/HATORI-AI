package com.hatoriel.app;
import org.junit.Test;
import static org.junit.Assert.*;
import java.io.*;
import java.nio.charset.StandardCharsets;
import java.util.ArrayList;
public class ChatTransportTest {
    private String parse(String text)throws Exception{return ChatTransport.decode(new ByteArrayInputStream(text.getBytes(StandardCharsets.UTF_8)),s->{},()->false);}
    @Test public void endpoints()throws Exception{
        assertEquals("http://127.0.0.1:11434/api/chat",ChatTransport.endpoint("http://127.0.0.1:11434").toString());
        assertEquals("https://example.com/api/chat",ChatTransport.endpoint("https://example.com/").toString());
        for(String url:new String[]{"http://example.com","https://u:p@example.com","https://example.com/a","https://example.com?q=a","https://example.com/#x","file:///tmp/test","https://example.com:99999"}){
            try{ChatTransport.endpoint(url);fail(url);}catch(IOException|java.net.URISyntaxException expected){}
        }
    }
    @Test public void unicodeAndFinalLine()throws Exception{
        ArrayList<String> chunks=new ArrayList<>();
        String raw="{\"message\":{\"content\":\"Привіт \"}}\n{\"message\":{\"content\":\"🌹\"},\"done\":true}";
        String result=ChatTransport.decode(new ByteArrayInputStream(raw.getBytes(StandardCharsets.UTF_8)),chunks::add,()->false);
        assertEquals("Привіт 🌹",result);assertEquals(2,chunks.size());
    }
    @Test public void truncated()throws Exception{assertThrows(IOException.class,()->parse("{\"message\":{\"content\":\"hi\"}}\n"));}
    @Test public void empty()throws Exception{assertThrows(IOException.class,()->parse("{\"done\":true}"));}
    @Test public void serverError()throws Exception{assertThrows(IOException.class,()->parse("{\"error\":\"failure\"}"));}
    @Test public void cancelled()throws Exception{assertThrows(InterruptedIOException.class,()->ChatTransport.decode(new ByteArrayInputStream("{\"done\":true}".getBytes(StandardCharsets.UTF_8)),s->{},()->true));}
    @Test public void oversizedFinalLine()throws Exception{StringBuilder s=new StringBuilder();for(int i=0;i<262145;i++)s.append('x');assertThrows(IOException.class,()->parse("{\"message\":{\"content\":\""+s+"\"},\"done\":true}"));}
}
