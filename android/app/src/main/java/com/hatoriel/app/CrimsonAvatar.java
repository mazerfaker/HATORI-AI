package com.hatoriel.app;
import android.content.Context;
import android.graphics.*;
import android.view.View;
import android.os.SystemClock;

final class CrimsonAvatar extends View {
    static final int RED=0xffee3434;
    private final Paint p=new Paint(Paint.ANTI_ALIAS_FLAG);
    private boolean busy;
    CrimsonAvatar(Context context){super(context);setContentDescription("Hatoriel · Crimson Core");}
    void setBusy(boolean value){busy=value;invalidate();}
    @Override protected void onDraw(Canvas canvas){
        super.onDraw(canvas);
        float t=(SystemClock.elapsedRealtime()%60000)/1000f;
        float x=getWidth()/2f,y=getHeight()/2f;
        float r=Math.min(x,y)*.56f*(1f+.035f*(float)Math.sin(t*(busy?4:1.8)));
        float density=getResources().getDisplayMetrics().density;
        p.setStyle(Paint.Style.STROKE);p.setColor(RED);
        p.setAlpha(24);p.setStrokeWidth(7*density);canvas.drawCircle(x,y,r,p);
        p.setAlpha(255);p.setStrokeWidth(2*density);canvas.drawCircle(x,y,r,p);
        for(int i=0;i<4;i++){double a=i*Math.PI/2;float dx=(float)Math.cos(a),dy=(float)Math.sin(a);canvas.drawLine(x+dx*r*.68f,y+dy*r*.68f,x+dx*r*1.42f,y+dy*r*1.42f,p);}
        p.setStyle(Paint.Style.FILL);canvas.drawCircle(x,y,Math.max(1.5f*density,r*.055f),p);
        if(busy){p.setStyle(Paint.Style.STROKE);p.setStrokeWidth(1.5f*density);p.setAlpha(180);float outer=r*1.65f;for(int i=0;i<3;i++)canvas.drawArc(x-outer,y-outer,x+outer,y+outer,t*85+i*120,60,false,p);}
        if(isShown()&&getWindowVisibility()==VISIBLE)postInvalidateDelayed(33);
    }
    @Override protected void onVisibilityChanged(View changed,int visibility){super.onVisibilityChanged(changed,visibility);if(visibility==VISIBLE)invalidate();}
    @Override protected void onWindowVisibilityChanged(int visibility){super.onWindowVisibilityChanged(visibility);if(visibility==VISIBLE)invalidate();}
}
