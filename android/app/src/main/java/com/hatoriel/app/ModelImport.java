package com.hatoriel.app;

import android.content.Context;
import android.net.Uri;
import java.io.*;

final class ModelImport {
    private static final long MAX_BYTES=8L*1024*1024*1024;
    static File directory(Context context)throws IOException{
        File dir=new File(context.getFilesDir(),"models");
        if(!dir.isDirectory()&&!dir.mkdirs())throw new IOException("Cannot create model directory");
        return dir;
    }
    static File selected(Context context)throws IOException{
        return new File(directory(context),"local.gguf");
    }
    static synchronized boolean remove(Context context)throws IOException{File f=selected(context);return !f.exists()||f.delete();}
    static synchronized long copy(Context context,Uri uri)throws IOException{
        File dir=directory(context),dest=selected(context),tmp=File.createTempFile("import-",".tmp",dir);
        try(InputStream in=context.getContentResolver().openInputStream(uri)){
            if(in==null)throw new IOException("Cannot open model");
            byte[] magic=new byte[4];int read=0;
            while(read<4){int n=in.read(magic,read,4-read);if(n<0)throw new IOException("Truncated model");read+=n;}
            if(magic[0]!='G'||magic[1]!='G'||magic[2]!='U'||magic[3]!='F')throw new IOException("Not a GGUF file");
            long total=4;
            try(FileOutputStream out=new FileOutputStream(tmp)){
                out.write(magic);
                byte[] buffer=new byte[128*1024];int n;
                while((n=in.read(buffer))!=-1){
                    if(Thread.currentThread().isInterrupted())throw new InterruptedIOException("Import cancelled");
                    if(total+n>MAX_BYTES)throw new IOException("Model exceeds 8 GiB limit");
                    if(dir.getUsableSpace()<n+32L*1024*1024)throw new IOException("Not enough storage");
                    out.write(buffer,0,n);total+=n;
                }
                out.getFD().sync();
            }
            if(Thread.currentThread().isInterrupted())throw new InterruptedIOException("Import cancelled");
            if(total<24)throw new IOException("Truncated GGUF header");
            // Android private storage: atomic rename replaces the prior file only after successful copy.
            android.system.Os.rename(tmp.getAbsolutePath(),dest.getAbsolutePath());
            return total;
        }catch(android.system.ErrnoException e){throw new IOException("Cannot commit imported model",e);}
        finally{if(tmp.exists())tmp.delete();}
    }
}
