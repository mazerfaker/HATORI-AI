package com.hatoriel.app;
import android.content.ContentResolver;
import android.database.Cursor;
import android.net.Uri;
import android.provider.OpenableColumns;
import android.graphics.Bitmap;
import android.graphics.BitmapFactory;
import java.io.*;
import java.util.ArrayList;

final class AttachmentDraft {
    static final int MAX_FILE=8*1024*1024,MAX_TOTAL=24*1024*1024,MAX_COUNT=4;
    static final class Item {
        final String name;
        final byte[] bytes;
        final Bitmap preview;
        Item(String n,byte[] b,Bitmap p){name=n;bytes=b;preview=p;}
    }
    final ArrayList<Item> items=new ArrayList<>();
    int remaining(){int total=0;for(Item i:items)total+=i.bytes.length;return Math.min(MAX_FILE,MAX_TOTAL-total);}
    boolean add(Item item){
        if(items.size()>=MAX_COUNT||item.bytes.length==0||item.bytes.length>remaining())return false;
        for(Item i:items)if(java.util.Arrays.equals(i.bytes,item.bytes))return false;
        items.add(item);return true;
    }
    static Item read(ContentResolver resolver,Uri uri,int limit)throws IOException{
        if(!"content".equals(uri.getScheme())||limit<=0)throw new IOException("Непідтримуваний файл або ліміт");
        String name="attachment";
        try(Cursor c=resolver.query(uri,new String[]{OpenableColumns.DISPLAY_NAME,OpenableColumns.SIZE},null,null,null)){
            if(c!=null&&c.moveToFirst()){
                if(!c.isNull(1)&&c.getLong(1)>limit)throw new IOException("Файл перевищує ліміт");
                if(!c.isNull(0))name=c.getString(0);
            }
        }
        name=name.replaceAll("[\\p{Cntrl}]","");if(name.length()>160)name=name.substring(0,160);if(name.isEmpty())name="attachment";
        byte[] data;
        try(InputStream in=resolver.openInputStream(uri);ByteArrayOutputStream out=new ByteArrayOutputStream()){
            if(in==null)throw new IOException("Не вдалося відкрити файл");
            byte[] buffer=new byte[65536];int n;
            while((n=in.read(buffer))!=-1){if(Thread.currentThread().isInterrupted())throw new IOException("Скасовано");if(n>limit-out.size())throw new IOException("Файл перевищує ліміт");out.write(buffer,0,n);}
            data=out.toByteArray();
        }
        if(data.length==0)throw new IOException("Порожній файл");
        Bitmap preview=null;
        BitmapFactory.Options options=new BitmapFactory.Options();options.inJustDecodeBounds=true;BitmapFactory.decodeByteArray(data,0,data.length,options);
        if(options.outWidth>0&&options.outHeight>0&&options.outWidth<=4096&&options.outHeight<=4096&&("image/png".equals(options.outMimeType)||"image/jpeg".equals(options.outMimeType))){
            int sample=1;while(options.outWidth/sample>160||options.outHeight/sample>160)sample*=2;
            options.inJustDecodeBounds=false;options.inSampleSize=sample;preview=BitmapFactory.decodeByteArray(data,0,data.length,options);
        }
        return new Item(name,data,preview);
    }
}
