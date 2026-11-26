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
    String request(String base,String model,JSONArray messages) throws Exception {
        URL url=endpoint(base);
        if(model.trim().isEmpty())throw new IOException("Вкажи модель");
        HttpURLConnection c=(HttpURLConnection)url.openConnection();
        connection=c;
        try {
            if(cancelled)throw new InterruptedIOException("Cancelled");
            c.setInstanceFollowRedirects(false);
            c.setConnectTimeout(8000);c.setReadTimeout(120000);
            c.setRequestMethod("POST");c.setDoOutput(true);
            c.setRequestProperty("Content-Type","application/json");
            byte[] data=new JSONObject().put("model",model).put("messages",messages).put("stream",false).toString().getBytes(StandardCharsets.UTF_8);
            c.setFixedLengthStreamingMode(data.length);
            try(OutputStream out=c.getOutputStream()){out.write(data);}
            int code=c.getResponseCode();
            if(code<200||code>=300)throw new IOException("HTTP "+code);
            ByteArrayOutputStream bytes=new ByteArrayOutputStream();
            try(InputStream in=c.getInputStream()){byte[] buffer=new byte[4096];int n;while((n=in.read(buffer))!=-1){if(cancelled)throw new InterruptedIOException("Cancelled");if(bytes.size()+n>524288)throw new IOException("Response too large");bytes.write(buffer,0,n);}}
            JSONObject response=new JSONObject(new String(bytes.toByteArray(),StandardCharsets.UTF_8));
            String reply=response.getJSONObject("message").getString("content").trim();
            if(reply.isEmpty())throw new IOException("Empty response");
            return reply;
        } finally {c.disconnect();connection=null;}
    }
    boolean isCancelled(){return cancelled;}
    void cancel(){cancelled=true;HttpURLConnection c=connection;if(c!=null)c.disconnect();}
}
