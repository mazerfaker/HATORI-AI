package com.hatoriel.app;
import java.net.*;
import java.io.*;
import java.nio.charset.StandardCharsets;
import org.json.*;

final class ChatTransport {
    private volatile HttpURLConnection connection;
    private volatile boolean cancelled;
    static URL endpoint(String base) throws Exception {
        URI uri=new URI(base.trim());
        if(uri.getPort()>65535||uri.getPort()==0)throw new IOException("Invalid port");
        String host=uri.getHost();
        boolean loopback="127.0.0.1".equals(host)||"localhost".equalsIgnoreCase(host);
        if(host==null||uri.getUserInfo()!=null||uri.getQuery()!=null||uri.getFragment()!=null||
           (uri.getPath()!=null&&!uri.getPath().isEmpty()&&!"/".equals(uri.getPath()))||
           !("https".equalsIgnoreCase(uri.getScheme())||("http".equalsIgnoreCase(uri.getScheme())&&loopback)))
            throw new IOException("Дозволено HTTPS або HTTP лише на localhost/127.0.0.1");
        return uri.resolve("/api/chat").toURL();
    }
    interface Chunk { void accept(String partial); }
    String request(String base,String model,JSONArray messages,String sessionKey,Chunk chunk) throws Exception {
        URL url=endpoint(base);
        if(!sessionKey.isEmpty()&&(!"https".equalsIgnoreCase(url.getProtocol())||!sessionKey.matches("[0-9a-f]{64}")))throw new IOException("Ключ Hatoriel потребує HTTPS і 64 hex-символи");
        if(model.trim().isEmpty())throw new IOException("Вкажи модель");
        HttpURLConnection c=(HttpURLConnection)url.openConnection();
        connection=c;
        try {
            if(cancelled)throw new InterruptedIOException("Cancelled");
            c.setInstanceFollowRedirects(false);
            c.setConnectTimeout(8000);c.setReadTimeout(120000);
            c.setRequestMethod("POST");c.setDoOutput(true);
            c.setRequestProperty("Content-Type","application/json");
            if(!sessionKey.isEmpty())c.setRequestProperty("Authorization","Bearer "+sessionKey);
            byte[] data=new JSONObject().put("model",model).put("messages",messages).put("stream",true).toString().getBytes(StandardCharsets.UTF_8);
            if(data.length>262144)throw new IOException("Request too large");
            c.setFixedLengthStreamingMode(data.length);
            try(OutputStream out=c.getOutputStream()){out.write(data);}
            int code=c.getResponseCode();
            if(code<200||code>=300)throw new IOException("HTTP "+code);
            try(InputStream in=c.getInputStream()){return decode(in,chunk,()->cancelled);}
        } finally {c.disconnect();connection=null;}
    }
    static String decode(InputStream in,Chunk chunk,java.util.function.BooleanSupplier cancelled)throws Exception{
        final int maxBytes=2097152,maxChars=262144;
        ByteArrayOutputStream line=new ByteArrayOutputStream();
        StringBuilder reply=new StringBuilder();int total=0,n;
        byte[] buffer=new byte[4096];boolean done=false;
        while(!done&&(n=in.read(buffer))!=-1){
            if(cancelled.getAsBoolean())throw new InterruptedIOException("Cancelled");
            total+=n;if(total>maxBytes)throw new IOException("Stream too large");
            for(int i=0;i<n;i++){
                if(buffer[i]!='\n'){line.write(buffer[i]);continue;}
                done=decodeLine(line,reply,chunk,maxChars);line.reset();
                if(done)break;
            }
        }
        if(!done&&line.size()>0)done=decodeLine(line,reply,chunk,maxChars);
        if(cancelled.getAsBoolean())throw new InterruptedIOException("Cancelled");
        if(!done)throw new IOException("Incomplete stream");
        if(reply.toString().trim().isEmpty())throw new IOException("Empty response");
        return reply.toString();
    }
    private static boolean decodeLine(ByteArrayOutputStream line,StringBuilder reply,Chunk chunk,int maxChars)throws Exception{
        String json=line.toString("UTF-8").trim();if(json.isEmpty())return false;
        JSONObject event=new JSONObject(json);
        if(event.has("error"))throw new IOException(event.getString("error"));
        JSONObject message=event.optJSONObject("message");
        if(message!=null){
            String part=message.optString("content","");
            if(part.length()>maxChars-reply.length())throw new IOException("Response too large");
            if(!part.isEmpty()){reply.append(part);chunk.accept(reply.toString());}
        }
        return event.optBoolean("done");
    }
    boolean isCancelled(){return cancelled;}
    void cancel(){if(cancelled)return;cancelled=true;HttpURLConnection c=connection;if(c!=null)new Thread(c::disconnect,"hatoriel-cancel").start();}
}
