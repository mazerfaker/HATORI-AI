package com.hatoriel.app;
import java.io.*;
import java.net.*;
import java.security.MessageDigest;
import java.nio.charset.StandardCharsets;

final class FileTransfer {
    static final int MAX=8*1024*1024;
    static final class Received {
        final String name; final byte[] bytes;
        Received(String n,byte[] b){name=n;bytes=b;}
    }
    private volatile HttpURLConnection connection;
    private volatile boolean cancelled;
    void cancel(){cancelled=true;HttpURLConnection c=connection;if(c!=null)new Thread(c::disconnect,"hatoriel-transfer-cancel").start();}
    Received receive(String base,String key,String code,java.util.function.IntConsumer progress)throws Exception{
        URL chat=ChatTransport.endpoint(base);
        if(!"https".equalsIgnoreCase(chat.getProtocol())||!key.matches("[0-9a-f]{64}")||!code.matches("[0-9a-f]{32}"))throw new IOException("Потрібен HTTPS, ключ сеансу та одноразовий код");
        URL url=new URL(chat,"/api/transfer");HttpURLConnection c=(HttpURLConnection)url.openConnection();connection=c;
        try{
            if(cancelled)throw new InterruptedIOException("Cancelled");
            c.setInstanceFollowRedirects(false);c.setConnectTimeout(8000);c.setReadTimeout(20000);c.setUseCaches(false);
            c.setRequestProperty("Authorization","Bearer "+key);c.setRequestProperty("X-Hatoriel-Transfer",code);c.setRequestProperty("Accept-Encoding","identity");
            if(c.getResponseCode()!=200)throw new IOException("Файл недоступний: HTTP "+c.getResponseCode());
            long size=c.getContentLengthLong();if(size<0||size>MAX)throw new IOException("Invalid length");
            String hash=c.getHeaderField("X-Hatoriel-SHA256"),encoded=c.getHeaderField("X-Hatoriel-Name");
            if(hash==null||!hash.matches("[0-9a-f]{64}")||encoded==null||encoded.length()>1024)throw new IOException("Invalid transfer metadata");
            String name=new String(android.util.Base64.decode(encoded,android.util.Base64.DEFAULT),StandardCharsets.UTF_8).replaceAll("[\\\\/:*?\"<>|\\p{Cntrl}]","").trim();
            if(name.isEmpty()||name.equals(".")||name.equals("..")||name.length()>320)name="attachment";
            ByteArrayOutputStream out=new ByteArrayOutputStream();byte[] buffer=new byte[16384];long start=android.os.SystemClock.elapsedRealtime();int last=-1;
            try(InputStream in=c.getInputStream()){int n;while((n=in.read(buffer))!=-1){
                if(cancelled||android.os.SystemClock.elapsedRealtime()-start>120000)throw new InterruptedIOException("Cancelled or timed out");
                if(n>MAX-out.size()||n>size-out.size())throw new IOException("Too large");out.write(buffer,0,n);
                int percent=size==0?100:(int)(100L*out.size()/size);if(percent/5!=last){last=percent/5;progress.accept(percent);}
            }}
            byte[] data=out.toByteArray();if(cancelled||data.length!=size)throw new IOException("Incomplete transfer");
            byte[] digest=MessageDigest.getInstance("SHA-256").digest(data);StringBuilder hex=new StringBuilder();for(byte b:digest)hex.append(String.format(java.util.Locale.ROOT,"%02x",b&255));
            if(!hash.equals(hex.toString()))throw new IOException("SHA-256 mismatch");
            progress.accept(100);return new Received(name,data);
        }finally{c.disconnect();connection=null;}
    }
}
