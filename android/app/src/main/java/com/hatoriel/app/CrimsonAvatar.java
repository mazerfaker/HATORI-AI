package com.hatoriel.app;
import android.content.Context;
import android.graphics.*;
import android.view.View;
import android.os.SystemClock;

final class CrimsonAvatar extends View {
    static final int RED=0xffee3434;
    enum State {
        IDLE("Очікування"), DICTATION("Системне диктування"), THINKING("Обробка запиту"), RESPONDING("Генерація відповіді"), SPEAKING("Озвучування");
        final String label;
        State(String label){this.label=label;}
        static State select(boolean external, boolean busy, boolean streaming, boolean speaking){
            if(external)return DICTATION;
            if(busy)return streaming?RESPONDING:THINKING;
            return speaking?SPEAKING:IDLE;
        }
    }
    private final Paint p=new Paint(Paint.ANTI_ALIAS_FLAG);
    private State state=State.IDLE;
    private boolean motion=true;
    CrimsonAvatar(Context context){super(context);setContentDescription("Hatoriel · "+state.label);}
    void setMotion(boolean enabled){if(motion!=enabled){motion=enabled;invalidate();}}
    void setState(State next){if(state!=next){state=next;setContentDescription("Hatoriel · "+state.label);invalidate();}}
    @Override protected void onDraw(Canvas canvas){
        super.onDraw(canvas);
        boolean animated=motion&&android.animation.ValueAnimator.areAnimatorsEnabled();
        float t=animated?(SystemClock.elapsedRealtime()%60000)/1000f:0f;
        float x=getWidth()/2f,y=getHeight()/2f;
        float speed=state==State.SPEAKING?6:state==State.RESPONDING?5:state==State.THINKING?3:1.8f;
        float r=Math.min(x,y)*.53f*(1f+(state==State.SPEAKING?.065f:.035f)*(float)Math.sin(t*speed));
        float density=getResources().getDisplayMetrics().density;
        p.setStyle(Paint.Style.STROKE);p.setColor(RED);
        p.setAlpha(24);p.setStrokeWidth(7*density);canvas.drawCircle(x,y,r,p);
        p.setAlpha(255);p.setStrokeWidth(2*density);canvas.drawCircle(x,y,r,p);
        for(int i=0;i<4;i++){double a=i*Math.PI/2;float dx=(float)Math.cos(a),dy=(float)Math.sin(a);canvas.drawLine(x+dx*r*.68f,y+dy*r*.68f,x+dx*r*1.42f,y+dy*r*1.42f,p);}
        p.setStyle(Paint.Style.FILL);canvas.drawCircle(x,y,Math.max(1.5f*density,r*.055f),p);
        p.setStyle(Paint.Style.STROKE);p.setStrokeWidth(1.5f*density);p.setAlpha(180);
        if(state==State.THINKING){float outer=r*1.65f;for(int i=0;i<3;i++)canvas.drawArc(x-outer,y-outer,x+outer,y+outer,t*85+i*120,60,false,p);}
        else if(state==State.DICTATION){p.setAlpha(115);canvas.drawCircle(x,y,r*1.54f,p);}
        else if(state==State.RESPONDING){p.setAlpha(100);canvas.drawCircle(x,y,r*(1.48f+.08f*(float)Math.sin(t*4)),p);}
        else if(state==State.SPEAKING){for(int i=-3;i<=3;i++){float bx=x+i*r*.28f,by=y+r*1.55f,h=r*(.12f+.17f*(float)Math.abs(Math.sin(t*6+i*.9f)));canvas.drawLine(bx,by-h,bx,by+h,p);}}
        if(animated&&isAttachedToWindow()&&isShown()&&getWindowVisibility()==VISIBLE)postInvalidateDelayed(state==State.IDLE?100:33);
    }
    @Override protected void onVisibilityChanged(View changed,int visibility){super.onVisibilityChanged(changed,visibility);if(visibility==VISIBLE)invalidate();}
    @Override protected void onWindowVisibilityChanged(int visibility){super.onWindowVisibilityChanged(visibility);if(visibility==VISIBLE)invalidate();}
}
