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
            c.setFixedLengthStreamingMode(data.length);
            try(OutputStream out=c.getOutputStream()){out.write(data);}
            int code=c.getResponseCode();
            if(code<200||code>=300)throw new IOException("HTTP "+code);
            StringBuilder reply=new StringBuilder();
            boolean done=false;
            int total=0;
            try(InputStream in=c.getInputStream()){
                ByteArrayOutputStream line=new ByteArrayOutputStream();
                int next;byte[] buffer=new byte[4096];
                while(!done&&(next=in.read(buffer))!=-1){
                    if(cancelled)throw new InterruptedIOException("Cancelled");
                    total+=next;if(total>2097152)throw new IOException("Stream too large");
                    for(int i=0;i<next;i++){
                        if(buffer[i]!='\n'){line.write(buffer[i]);continue;}
                        String json=line.toString("UTF-8").trim();line.reset();
                        if(json.isEmpty())continue;
                        JSONObject event=new JSONObject(json);
                        if(event.has("error"))throw new IOException(event.getString("error"));
                        JSONObject message=event.optJSONObject("message");
                        if(message!=null){reply.append(message.optString("content",""));if(reply.length()>262144)throw new IOException("Response too large");chunk.accept(reply.toString());}
                        if(event.optBoolean("done")){done=true;break;}
                    }
                }
                if(!done&&line.size()>0){JSONObject event=new JSONObject(line.toString("UTF-8"));if(event.has("error"))throw new IOException(event.getString("error"));JSONObject message=event.optJSONObject("message");if(message!=null)reply.append(message.optString("content",""));done=event.optBoolean("done");}
            }
            if(cancelled)throw new InterruptedIOException("Cancelled");
            if(!done)throw new IOException("Incomplete stream");
            if(reply.toString().trim().isEmpty())throw new IOException("Empty response");
            return reply.toString();
        } finally {c.disconnect();connection=null;}
    }
    boolean isCancelled(){return cancelled;}
    void cancel(){if(cancelled)return;cancelled=true;HttpURLConnection c=connection;if(c!=null)new Thread(c::disconnect,"hatoriel-cancel").start();}
}
